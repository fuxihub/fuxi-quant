use crate::mcp::McpConfig;
use crate::model::Model;
use crate::tool::{
    Tool, ToolCall, ToolResult, build_tool_system_prompt, format_tool_responses, has_tool_call,
    parse_tool_calls,
};
use anyhow::Result;
use llama_cpp_2::{
    context::{LlamaContext, params::LlamaContextParams},
    llama_batch::LlamaBatch,
    model::{AddBos, Special},
    sampling::LlamaSampler,
};
use std::{io::Write, num::NonZeroU32};

/// Agent 配置
#[derive(Clone, Debug, Default)]
pub struct AgentConfig {
    /// 系统提示词
    pub system_prompt: Option<String>,
    /// 上下文长度
    pub ctx_len: u32,
    /// 工具列表
    pub tools: Vec<Tool>,
    /// 最大工具调用轮数
    pub max_tool_rounds: usize,
    /// MCP 配置
    pub mcp_config: Option<McpConfig>,
}

impl AgentConfig {
    pub fn new() -> Self {
        Self {
            system_prompt: None,
            ctx_len: 8192,
            tools: crate::tool::builtin::all_builtin_tools(),
            max_tool_rounds: 10,
            mcp_config: None,
        }
    }

    pub fn with_system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.system_prompt = Some(prompt.into());
        self
    }

    pub fn with_ctx_len(mut self, len: u32) -> Self {
        self.ctx_len = len;
        self
    }

    pub fn with_tools(mut self, tools: Vec<Tool>) -> Self {
        self.tools = tools;
        self
    }

    pub fn with_tool(mut self, tool: Tool) -> Self {
        self.tools.push(tool);
        self
    }

    pub fn with_max_tool_rounds(mut self, rounds: usize) -> Self {
        self.max_tool_rounds = rounds;
        self
    }

    pub fn with_mcp_config(mut self, config: McpConfig) -> Self {
        self.mcp_config = Some(config);
        self
    }
}

pub struct Agent {
    model: &'static Model,
    config: AgentConfig,
    ctx: LlamaContext<'static>,
    n_cur: usize,
    is_first_turn: bool,
}

unsafe impl Send for Agent {}
unsafe impl Sync for Agent {}

impl Agent {
    pub fn new(model: &'static Model, config: AgentConfig) -> Result<Self> {
        let ctx = model.model.new_context(
            &model.backend,
            LlamaContextParams::default()
                .with_n_ctx(NonZeroU32::new(config.ctx_len))
                .with_n_batch(512),
        )?;

        Ok(Self {
            model,
            config,
            ctx,
            n_cur: 0,
            is_first_turn: true,
        })
    }

    pub fn reset(&mut self) -> Result<()> {
        self.ctx = self.model.model.new_context(
            &self.model.backend,
            LlamaContextParams::default()
                .with_n_ctx(NonZeroU32::new(self.config.ctx_len))
                .with_n_batch(512),
        )?;
        self.n_cur = 0;
        self.is_first_turn = true;
        Ok(())
    }

    pub fn tools(&self) -> &[Tool] {
        &self.config.tools
    }

    fn build_system_prompt(&self) -> Option<String> {
        if self.config.tools.is_empty() {
            self.config.system_prompt.clone()
        } else {
            Some(build_tool_system_prompt(
                self.config.system_prompt.as_deref(),
                &self.config.tools,
            ))
        }
    }

    /// 流式对话（支持 thinking + 工具调用）
    pub fn chat<F, E>(
        &mut self,
        message: &str,
        mut on_token: F,
        mut tool_executor: E,
    ) -> Result<String>
    where
        F: FnMut(&str),
        E: FnMut(&ToolCall) -> Option<ToolResult>,
    {
        let mut current_message = message.to_string();
        let mut rounds = 0;

        loop {
            if rounds >= self.config.max_tool_rounds {
                break;
            }
            rounds += 1;

            let response = self.generate(&current_message, &mut on_token)?;

            // 检查工具调用
            if has_tool_call(&response) {
                let tool_calls = parse_tool_calls(&response);
                let mut results = Vec::new();

                for call in &tool_calls {
                    if let Some(result) = tool_executor(call) {
                        results.push(result);
                    }
                }

                if !results.is_empty() {
                    current_message = format_tool_responses(&results);
                    continue;
                }
            }

            return Ok(crate::tool::extract_content_without_tool_calls(&response));
        }

        Ok(String::new())
    }

    /// 核心生成方法
    fn generate<F>(&mut self, message: &str, on_token: &mut F) -> Result<String>
    where
        F: FnMut(&str),
    {
        // ReAct: 第一轮带工具时添加 Question 前缀
        let formatted = if self.is_first_turn && !self.config.tools.is_empty() {
            format!("Question: {}\nThought:", message)
        } else {
            message.to_string()
        };

        // 构建 prompt
        let prompt = if self.is_first_turn {
            let mut p = String::new();
            if let Some(sys) = self.build_system_prompt() {
                p.push_str("<|im_start|>system\n");
                p.push_str(&sys);
                p.push_str("<|im_end|>\n");
            }
            p.push_str("<|im_start|>user\n");
            p.push_str(&formatted);
            p.push_str("<|im_end|>\n");
            p.push_str("<|im_start|>assistant\n<think>");
            p
        } else {
            format!(
                "<|im_start|>user\n{}<|im_end|>\n<|im_start|>assistant\n<think>",
                formatted
            )
        };

        let add_bos = if self.is_first_turn { AddBos::Always } else { AddBos::Never };
        self.is_first_turn = false;

        // Tokenize & decode prompt
        let tokens = self.model.model.str_to_token(&prompt, add_bos).map_err(|e| anyhow::anyhow!(e))?;
        let mut batch = LlamaBatch::new(512, 1);

        for chunk_start in (0..tokens.len()).step_by(512) {
            batch.clear();
            let chunk_end = (chunk_start + 512).min(tokens.len());
            for (i, &token) in tokens.iter().enumerate().take(chunk_end).skip(chunk_start) {
                batch.add(token, (self.n_cur + i) as i32, &[0], i == tokens.len() - 1)?;
            }
            self.ctx.decode(&mut batch)?;
        }
        self.n_cur += tokens.len();

        let think_start = self.n_cur;
        let mut in_thinking = true;
        let mut buffer = String::new();
        let mut response = String::new();

        // Qwen3 Thinking Mode 采样参数
        let mut sampler = LlamaSampler::chain_simple([
            LlamaSampler::top_k(20),
            LlamaSampler::top_p(0.95, 1),
            LlamaSampler::min_p(0.0, 1),
            LlamaSampler::temp(0.6),
            LlamaSampler::dist(self.n_cur as u32),
        ]);

        // 生成循环
        loop {
            let token = sampler.sample(&self.ctx, -1);
            sampler.accept(token);

            if self.model.model.is_eog_token(token) {
                break;
            }

            if let Ok(piece) = self.model.model.token_to_str(token, Special::Tokenize) {
                if cfg!(debug_assertions) {
                    print!("{piece}");
                    let _ = std::io::stdout().flush();
                }

                if in_thinking {
                    buffer.push_str(&piece);
                    if let Some(pos) = buffer.find("</think>") {
                        in_thinking = false;
                        let after = buffer[pos + 8..].trim_start();
                        if !after.is_empty() {
                            response.push_str(after);
                            // 不在 thinking 阶段输出 Action 相关内容
                            if !after.contains("Action") {
                                on_token(after);
                            }
                        }
                        buffer.clear();
                    }
                } else {
                    response.push_str(&piece);
                    // 检测到 Action 后不再输出
                    if !response.contains("Action:") {
                        on_token(&piece);
                    }
                }
            }

            batch.clear();
            batch.add(token, self.n_cur as i32, &[0], true)?;
            self.n_cur += 1;
            self.ctx.decode(&mut batch)?;
        }

        // 清理 KV cache 中的 thinking 内容
        if think_start < self.n_cur {
            let len = self.n_cur - think_start;
            self.ctx.clear_kv_cache_seq(Some(0), Some(think_start as u32), Some(self.n_cur as u32))?;
            self.ctx.kv_cache_seq_add(0, Some(self.n_cur as u32), None, -(len as i32))?;
            self.n_cur = think_start;
        }

        // 添加 im_end
        let end_tokens = self.model.model.str_to_token("<|im_end|>\n", AddBos::Never).map_err(|e| anyhow::anyhow!(e))?;
        batch.clear();
        for (i, &t) in end_tokens.iter().enumerate() {
            batch.add(t, (self.n_cur + i) as i32, &[0], i == end_tokens.len() - 1)?;
        }
        self.ctx.decode(&mut batch)?;
        self.n_cur += end_tokens.len();

        Ok(response)
    }
}
