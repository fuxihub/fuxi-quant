//! 智能体模块

use crate::mcp::{McpConfig, McpManager};
use crate::tool::{Tool, ToolCall, ToolResult, has_tool_call, parse_tool_calls};
use anyhow::Result;
use llama_cpp_2::{
    context::{LlamaContext, params::LlamaContextParams},
    llama_batch::LlamaBatch,
    model::{AddBos, Special},
    sampling::LlamaSampler,
};
use std::{io::Write, num::NonZeroU32, sync::Arc};

use crate::model::Model;

/// 内置系统提示词
const SYSTEM_PROMPT: &str = r#"你是一个智能助手，可以使用工具来帮助用户完成任务。

使用工具时，请遵循以下格式：
Thought: 思考需要做什么
Action: 工具名称
Action Input: {"参数名": "参数值"}
Observation: 工具返回结果

完成任务后，使用以下格式回复：
Thought: 我现在知道最终答案了
Final Answer: 最终答案"#;

/// 智能体
pub struct Agent {
    model: &'static Model,
    ctx: LlamaContext<'static>,
    ctx_len: u32,
    mcp_manager: Option<Arc<McpManager>>,
    tools: Vec<Tool>,
    n_cur: usize,
    is_first_turn: bool,
}

unsafe impl Send for Agent {}
unsafe impl Sync for Agent {}

impl Agent {
    /// 创建智能体
    ///
    /// # 参数
    /// - `model`: 模型实例
    /// - `ctx_len`: 上下文最大长度
    /// - `mcp_config`: MCP 配置（可选）
    pub fn new(model: &'static Model, ctx_len: u32, mcp_config: Option<McpConfig>) -> Result<Self> {
        let ctx = model.model.new_context(
            &model.backend,
            LlamaContextParams::default()
                .with_n_ctx(NonZeroU32::new(ctx_len))
                .with_n_batch(512),
        )?;

        // 初始化 MCP 管理器并获取工具
        let (mcp_manager, tools) = if let Some(config) = mcp_config {
            let manager = Arc::new(McpManager::new());
            let mcp_tools = manager.init(&config).unwrap_or_default();
            (Some(manager), mcp_tools)
        } else {
            (None, Vec::new())
        };

        Ok(Self {
            model,
            ctx,
            ctx_len,
            mcp_manager,
            tools,
            n_cur: 0,
            is_first_turn: true,
        })
    }

    /// 重置对话
    pub fn reset(&mut self) -> Result<()> {
        self.ctx = self.model.model.new_context(
            &self.model.backend,
            LlamaContextParams::default()
                .with_n_ctx(NonZeroU32::new(self.ctx_len))
                .with_n_batch(512),
        )?;
        self.n_cur = 0;
        self.is_first_turn = true;
        Ok(())
    }

    /// 获取工具列表
    pub fn tools(&self) -> &[Tool] {
        &self.tools
    }

    /// 构建系统提示词（包含工具描述）
    fn build_system_prompt(&self) -> String {
        if self.tools.is_empty() {
            return SYSTEM_PROMPT.to_string();
        }

        let mut prompt = SYSTEM_PROMPT.to_string();
        prompt.push_str("\n\n可用工具：\n");

        for tool in &self.tools {
            let params_json = serde_json::to_string(&tool.function.parameters).unwrap_or_default();
            prompt.push_str(&format!(
                "\n{}: {}\n参数: {}\n",
                tool.function.name, tool.function.description, params_json
            ));
        }

        prompt
    }

    /// 执行工具调用
    fn execute_tool(&self, call: &ToolCall) -> Option<ToolResult> {
        if let Some(ref manager) = self.mcp_manager {
            match manager.call_tool(call) {
                Ok(result) => Some(result),
                Err(e) => Some(ToolResult {
                    name: call.name.clone(),
                    content: serde_json::json!({ "error": e.to_string() }),
                }),
            }
        } else {
            None
        }
    }

    /// 流式对话
    ///
    /// # 参数
    /// - `message`: 用户消息
    /// - `on_token`: token 回调函数
    ///
    /// # 返回
    /// 完整的响应文本
    pub fn chat<F>(&mut self, message: &str, mut on_token: F) -> Result<String>
    where
        F: FnMut(&str),
    {
        let mut current_message = message.to_string();
        let max_rounds = 10;

        for _ in 0..max_rounds {
            let response = self.generate(&current_message, &mut on_token)?;

            // 检查工具调用
            if has_tool_call(&response) {
                let tool_calls = parse_tool_calls(&response);
                let mut results = Vec::new();

                for call in &tool_calls {
                    if let Some(result) = self.execute_tool(call) {
                        results.push(result);
                    }
                }

                if !results.is_empty() {
                    // 格式化工具结果作为下一轮输入
                    current_message = results
                        .iter()
                        .map(|r| {
                            let json = serde_json::to_string_pretty(&r.content).unwrap_or_default();
                            format!("Observation: {}", json)
                        })
                        .collect::<Vec<_>>()
                        .join("\n");
                    continue;
                }
            }

            // 提取最终答案
            return Ok(extract_final_answer(&response));
        }

        Ok(String::new())
    }

    /// 核心生成方法（始终使用 think 模式）
    fn generate<F>(&mut self, message: &str, on_token: &mut F) -> Result<String>
    where
        F: FnMut(&str),
    {
        // 构建 prompt
        let prompt = if self.is_first_turn {
            let sys = self.build_system_prompt();
            format!(
                "<|im_start|>system\n{}<|im_end|>\n<|im_start|>user\n{}<|im_end|>\n<|im_start|>assistant\n<think>",
                sys, message
            )
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

        // Tokenize & decode prompt
        let tokens = self
            .model
            .model
            .str_to_token(&prompt, add_bos)
            .map_err(|e| anyhow::anyhow!(e))?;
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
                            if !after.contains("Action") {
                                on_token(after);
                            }
                        }
                        buffer.clear();
                    }
                } else {
                    response.push_str(&piece);
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
            self.ctx.clear_kv_cache_seq(
                Some(0),
                Some(think_start as u32),
                Some(self.n_cur as u32),
            )?;
            self.ctx
                .kv_cache_seq_add(0, Some(self.n_cur as u32), None, -(len as i32))?;
            self.n_cur = think_start;
        }

        // 添加 im_end
        let end_tokens = self
            .model
            .model
            .str_to_token("<|im_end|>\n", AddBos::Never)
            .map_err(|e| anyhow::anyhow!(e))?;
        batch.clear();
        for (i, &t) in end_tokens.iter().enumerate() {
            batch.add(t, (self.n_cur + i) as i32, &[0], i == end_tokens.len() - 1)?;
        }
        self.ctx.decode(&mut batch)?;
        self.n_cur += end_tokens.len();

        Ok(response)
    }
}

/// 提取最终答案
fn extract_final_answer(output: &str) -> String {
    if let Some(pos) = output.find("Final Answer:") {
        let start = pos + "Final Answer:".len();
        return output[start..].trim().to_string();
    }
    if let Some(pos) = output.find("Action:") {
        return output[..pos].trim().to_string();
    }
    output.trim().to_string()
}
