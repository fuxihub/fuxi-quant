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
    /// 最大工具调用轮数（防止无限循环）
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

    /// 兼容旧 API
    pub fn new_simple(
        model: &'static Model,
        sys_prompt: Option<String>,
        ctx_len: u32,
    ) -> Result<Self> {
        let config = AgentConfig {
            system_prompt: sys_prompt,
            ctx_len,
            tools: Vec::new(),
            max_tool_rounds: 10,
            mcp_config: None,
        };
        Self::new(model, config)
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

    /// 获取工具列表
    pub fn tools(&self) -> &[Tool] {
        &self.config.tools
    }

    /// 构建系统提示词（包含工具定义）
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

    /// 流式对话，回调直接输出字符串
    pub fn chat<F>(&mut self, message: &str, mut on_token: F) -> Result<String>
    where
        F: FnMut(&str),
    {
        if cfg!(debug_assertions) {
            println!("{message}");
            let _ = std::io::stdout().flush();
        }

        let prompt = if self.is_first_turn {
            let mut p = String::new();
            if let Some(sys) = self.build_system_prompt() {
                p.push_str("<|im_start|>system\n");
                p.push_str(&sys);
                p.push_str("<|im_end|>\n");
            }
            p.push_str("<|im_start|>user\n");
            p.push_str(message);
            p.push_str("<|im_end|>\n");
            p.push_str("<|im_start|>assistant\n<think>");
            p
        } else {
            format!(
                "<|im_start|>user\n{}<|im_end|>\n<|im_start|>assistant\n<think>",
                message
            )
        };

        let add_bos = if self.is_first_turn {
            AddBos::Always
        } else {
            AddBos::Never
        };
        self.is_first_turn = false;

        let tokens = self
            .model
            .model
            .str_to_token(&prompt, add_bos)
            .map_err(|e| anyhow::anyhow!(e))?;

        let ctx = &mut self.ctx;
        let mut batch = LlamaBatch::new(512, 1);
        let n_tokens = tokens.len();

        for chunk_start in (0..n_tokens).step_by(512) {
            batch.clear();
            let chunk_end = (chunk_start + 512).min(n_tokens);
            for (i, &token) in tokens.iter().enumerate().take(chunk_end).skip(chunk_start) {
                batch.add(token, (self.n_cur + i) as i32, &[0], i == n_tokens - 1)?;
            }
            ctx.decode(&mut batch)?;
        }
        self.n_cur += n_tokens;

        let think_start = self.n_cur;
        let mut in_thinking = true;
        let mut buffer = String::new();
        let mut full_response = String::new();

        let mut sampler = LlamaSampler::chain_simple([
            LlamaSampler::top_k(20),
            LlamaSampler::top_p(0.95, 1),
            LlamaSampler::min_p(0.0, 1),
            LlamaSampler::temp(0.6),
            LlamaSampler::penalties(-1, 1.0, 0.0, 0.0),
            LlamaSampler::dist(self.n_cur as u32),
        ]);

        loop {
            let next_token = sampler.sample(ctx, -1);
            sampler.accept(next_token);

            if self.model.model.is_eog_token(next_token) {
                break;
            }

            if let Ok(piece) = self.model.model.token_to_str(next_token, Special::Tokenize) {
                if cfg!(debug_assertions) {
                    print!("{piece}");
                    let _ = std::io::stdout().flush();
                }

                if in_thinking {
                    buffer.push_str(&piece);
                    if let Some(pos) = buffer.find("</think>") {
                        in_thinking = false;
                        let after = buffer[pos + 8..].trim_start().to_string();
                        buffer.clear();
                        if !after.is_empty() {
                            on_token(&after);
                            full_response.push_str(&after);
                        }
                    }
                } else {
                    on_token(&piece);
                    full_response.push_str(&piece);
                }
            }

            batch.clear();
            batch.add(next_token, self.n_cur as i32, &[0], true)?;
            self.n_cur += 1;
            ctx.decode(&mut batch)?;
        }

        // 清理 thinking 内容从 KV cache
        if think_start < self.n_cur {
            let think_len = self.n_cur - think_start;
            ctx.clear_kv_cache_seq(Some(0), Some(think_start as u32), Some(self.n_cur as u32))?;
            ctx.kv_cache_seq_add(0, Some(self.n_cur as u32), None, -(think_len as i32))?;
            self.n_cur = think_start;
        }

        let im_end_tokens = self
            .model
            .model
            .str_to_token("<|im_end|>\n", AddBos::Never)
            .map_err(|e| anyhow::anyhow!(e))?;

        batch.clear();
        for (i, &token) in im_end_tokens.iter().enumerate() {
            let is_last = i == im_end_tokens.len() - 1;
            batch.add(token, (self.n_cur + i) as i32, &[0], is_last)?;
        }
        ctx.decode(&mut batch)?;
        self.n_cur += im_end_tokens.len();

        Ok(full_response)
    }

    /// 带工具调用的对话
    ///
    /// - `message`: 用户消息
    /// - `on_token`: 流式 token 回调
    /// - `tool_executor`: 工具执行器，接收 ToolCall 返回 ToolResult
    ///
    /// 返回最终的完整响应文本
    pub fn chat_with_tools<F, E>(
        &mut self,
        message: &str,
        mut on_token: F,
        mut tool_executor: E,
    ) -> Result<String>
    where
        F: FnMut(&str),
        E: FnMut(&ToolCall) -> Option<ToolResult>,
    {
        let mut full_response = String::new();
        let mut current_message = message.to_string();
        let mut rounds = 0;

        loop {
            if rounds >= self.config.max_tool_rounds {
                eprintln!("超过最大工具调用轮数");
                break;
            }
            rounds += 1;

            // 收集本轮响应
            let mut round_response = String::new();
            let mut had_tool_call = false;
            let mut pending_tokens = String::new();
            let mut in_tool_call = false;

            self.chat_internal(&current_message, |token| {
                round_response.push_str(token);

                if in_tool_call {
                    return;
                }

                pending_tokens.push_str(token);

                if pending_tokens.contains("<tool_call>") {
                    in_tool_call = true;
                    if let Some(pos) = pending_tokens.find("<tool_call>") {
                        let before = pending_tokens[..pos].trim();
                        if !before.is_empty() && !before.starts_with("{\"name\"") {
                            on_token(before);
                        }
                    }
                    pending_tokens.clear();
                } else if pending_tokens.contains("<tool") || pending_tokens.contains("{\"name\"") {
                    // 继续缓存
                } else {
                    on_token(&pending_tokens);
                    pending_tokens.clear();
                }
            })?;

            // 检查是否有工具调用
            if has_tool_call(&round_response) {
                had_tool_call = true;
                let tool_calls = parse_tool_calls(&round_response);

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

            full_response = if had_tool_call {
                crate::tool::extract_content_without_tool_calls(&round_response)
            } else {
                round_response
            };
            break;
        }

        Ok(full_response)
    }

    /// 内部 chat 方法
    fn chat_internal<F>(&mut self, message: &str, mut on_token: F) -> Result<()>
    where
        F: FnMut(&str),
    {
        if cfg!(debug_assertions) {
            println!("{message}");
            print!("<think>");
            let _ = std::io::stdout().flush();
        }

        let prompt = if self.is_first_turn {
            let mut p = String::new();
            if let Some(sys) = self.build_system_prompt() {
                p.push_str("<|im_start|>system\n");
                p.push_str(&sys);
                p.push_str("<|im_end|>\n");
            }
            p.push_str("<|im_start|>user\n");
            p.push_str(message);
            p.push_str("<|im_end|>\n");
            p.push_str("<|im_start|>assistant\n<think>");
            p
        } else {
            format!(
                "<|im_start|>user\n{}<|im_end|>\n<|im_start|>assistant\n<think>",
                message
            )
        };

        let add_bos = if self.is_first_turn {
            AddBos::Always
        } else {
            AddBos::Never
        };
        self.is_first_turn = false;

        let tokens = self
            .model
            .model
            .str_to_token(&prompt, add_bos)
            .map_err(|e| anyhow::anyhow!(e))?;

        let ctx = &mut self.ctx;
        let mut batch = LlamaBatch::new(512, 1);
        let n_tokens = tokens.len();

        for chunk_start in (0..n_tokens).step_by(512) {
            batch.clear();
            let chunk_end = (chunk_start + 512).min(n_tokens);
            for (i, &token) in tokens.iter().enumerate().take(chunk_end).skip(chunk_start) {
                batch.add(token, (self.n_cur + i) as i32, &[0], i == n_tokens - 1)?;
            }
            ctx.decode(&mut batch)?;
        }
        self.n_cur += n_tokens;

        let think_start = self.n_cur;
        let mut in_thinking = true;
        let mut buffer = String::new();

        let mut sampler = LlamaSampler::chain_simple([
            LlamaSampler::top_k(20),
            LlamaSampler::top_p(0.95, 1),
            LlamaSampler::min_p(0.0, 1),
            LlamaSampler::temp(0.6),
            LlamaSampler::penalties(-1, 1.0, 0.0, 0.0),
            LlamaSampler::dist(self.n_cur as u32),
        ]);

        loop {
            let next_token = sampler.sample(ctx, -1);
            sampler.accept(next_token);

            if self.model.model.is_eog_token(next_token) {
                break;
            }

            if let Ok(piece) = self.model.model.token_to_str(next_token, Special::Tokenize) {
                if cfg!(debug_assertions) {
                    print!("{piece}");
                    let _ = std::io::stdout().flush();
                }

                if in_thinking {
                    buffer.push_str(&piece);
                    if let Some(pos) = buffer.find("</think>") {
                        in_thinking = false;
                        let after = buffer[pos + 8..].trim_start().to_string();
                        buffer.clear();
                        if !after.is_empty() {
                            on_token(&after);
                        }
                    }
                } else {
                    on_token(&piece);
                }
            }

            batch.clear();
            batch.add(next_token, self.n_cur as i32, &[0], true)?;
            self.n_cur += 1;
            ctx.decode(&mut batch)?;
        }

        // 清理 thinking 内容从 KV cache
        if think_start < self.n_cur {
            let think_len = self.n_cur - think_start;
            ctx.clear_kv_cache_seq(Some(0), Some(think_start as u32), Some(self.n_cur as u32))?;
            ctx.kv_cache_seq_add(0, Some(self.n_cur as u32), None, -(think_len as i32))?;
            self.n_cur = think_start;
        }

        let im_end_tokens = self
            .model
            .model
            .str_to_token("<|im_end|>\n", AddBos::Never)
            .map_err(|e| anyhow::anyhow!(e))?;

        batch.clear();
        for (i, &token) in im_end_tokens.iter().enumerate() {
            let is_last = i == im_end_tokens.len() - 1;
            batch.add(token, (self.n_cur + i) as i32, &[0], is_last)?;
        }
        ctx.decode(&mut batch)?;
        self.n_cur += im_end_tokens.len();

        Ok(())
    }
}
