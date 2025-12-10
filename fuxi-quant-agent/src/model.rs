use anyhow::Result;
use llama_cpp_2::{
    LogOptions,
    context::{LlamaContext, params::LlamaContextParams},
    llama_backend::LlamaBackend,
    llama_batch::LlamaBatch,
    model::{AddBos, LlamaModel, Special, params::LlamaModelParams},
    send_logs_to_tracing,
    token::data_array::LlamaTokenDataArray,
};
use std::{num::NonZeroU32, path::Path};

/// 采样参数（官方推荐值）
#[derive(Debug, Clone, Copy)]
pub struct SamplingParams {
    pub temperature: f32,
    pub top_p: f32,
    pub top_k: i32,
    pub min_p: f32,
    pub presence_penalty: f32,
}

impl Default for SamplingParams {
    /// 默认使用 Non-Thinking 模式参数
    fn default() -> Self {
        Self {
            temperature: 0.7,
            top_p: 0.8,
            top_k: 20,
            min_p: 0.0,
            presence_penalty: 1.5,
        }
    }
}

impl SamplingParams {
    /// Thinking 模式采样参数
    pub fn thinking() -> Self {
        Self {
            temperature: 0.6,
            top_p: 0.95,
            top_k: 20,
            min_p: 0.0,
            presence_penalty: 1.5,
        }
    }

    /// Non-Thinking 模式采样参数
    pub fn non_thinking() -> Self {
        Self::default()
    }
}

/// 简单的 Qwen3 GGUF 推理封装（基于 llama-cpp-2）
pub struct Qwen3Llama {
    backend: LlamaBackend,
    model: LlamaModel,
    ctx_params: LlamaContextParams,
    n_ctx: u32,
}

impl Qwen3Llama {
    /// 默认 GPU 层数（999 = 全部放 GPU，Mac Metal 加速）
    const DEFAULT_GPU_LAYERS: u32 = 999;

    /// 从 GGUF 文件加载模型（使用模型默认配置，上下文使用模型支持的最大值）
    pub fn load(model_path: impl AsRef<Path>) -> Result<Self> {
        // 禁用 llama.cpp 底层日志输出
        send_logs_to_tracing(LogOptions::default().with_logs_enabled(false));

        let backend = LlamaBackend::init()?;
        let model_params = LlamaModelParams::default().with_n_gpu_layers(Self::DEFAULT_GPU_LAYERS);
        let model = LlamaModel::load_from_file(&backend, model_path.as_ref(), &model_params)?;

        // 从模型元数据读取最大上下文长度
        let n_ctx = model.n_ctx_train();

        let ctx_params = LlamaContextParams::default()
            .with_n_ctx(NonZeroU32::new(n_ctx))
            .with_n_batch(512);

        Ok(Self {
            backend,
            model,
            ctx_params,
            n_ctx,
        })
    }

    /// 获取上下文长度
    pub fn n_ctx(&self) -> u32 {
        self.n_ctx
    }

    /// 流式生成回复（每生成一个 token 调用回调）
    pub fn chat_stream<F>(&self, system: Option<&str>, user: &str, on_token: F) -> Result<String>
    where
        F: FnMut(&str),
    {
        let prompt = Self::format_prompt(system, user);
        self.generate_stream(&prompt, SamplingParams::default(), on_token)
    }

    /// 流式生成（每生成一个 token 调用回调）
    pub fn generate_stream<F>(
        &self,
        prompt: &str,
        params: SamplingParams,
        mut on_token: F,
    ) -> Result<String>
    where
        F: FnMut(&str),
    {
        let mut ctx: LlamaContext = self
            .model
            .new_context(&self.backend, self.ctx_params.clone())?;

        let tokens = self
            .model
            .str_to_token(prompt, AddBos::Always)
            .map_err(|e| anyhow::anyhow!(e))?;
        let n_prompt = tokens.len();

        // 分批处理 prompt tokens（每批最多 512 个）
        const BATCH_SIZE: usize = 512;
        let mut batch = LlamaBatch::new(BATCH_SIZE, 1);

        for chunk_start in (0..n_prompt).step_by(BATCH_SIZE) {
            batch.clear();
            let chunk_end = (chunk_start + BATCH_SIZE).min(n_prompt);
            for (i, &token) in tokens.iter().enumerate().take(chunk_end).skip(chunk_start) {
                let is_last = i == n_prompt - 1;
                batch.add(token, i as i32, &[0], is_last)?;
            }
            ctx.decode(&mut batch)?;
        }

        let eos = self.model.token_eos();
        let mut output = String::new();
        let mut n_cur = n_prompt;
        let max_ctx = self.ctx_params.n_ctx().map(NonZeroU32::get).unwrap_or(0) as usize;

        loop {
            if max_ctx > 0 && n_cur >= max_ctx.saturating_sub(1) {
                break;
            }

            let mut candidates =
                LlamaTokenDataArray::from_iter(ctx.candidates_ith(batch.n_tokens() - 1), false);

            let _ = params;
            let seed = (n_cur as u32).wrapping_mul(1103515245).wrapping_add(12345);
            let next_token = candidates.sample_token(seed);

            if next_token == eos {
                break;
            }

            if let Ok(piece) = self.model.token_to_str(next_token, Special::Tokenize) {
                on_token(&piece); // 流式回调
                output.push_str(&piece);
            }

            batch.clear();
            batch.add(next_token, n_cur as i32, &[0], true)?;
            n_cur += 1;

            ctx.decode(&mut batch)?;
        }

        Ok(output)
    }

    /// 简易 ChatML 模板
    fn format_prompt(system: Option<&str>, user: &str) -> String {
        let mut prompt = String::new();
        if let Some(sys) = system {
            prompt.push_str("<|im_start|>system\n");
            prompt.push_str(sys);
            prompt.push_str("<|im_end|>\n");
        }
        prompt.push_str("<|im_start|>user\n");
        prompt.push_str(user);
        prompt.push_str("<|im_end|>\n");
        prompt.push_str("<|im_start|>assistant\n");
        prompt
    }

    /// 带工具的 ChatML 模板（Hermes-style tool use）
    fn format_prompt_with_tools(
        system: Option<&str>,
        user: &str,
        tools: &crate::tool::ToolRegistry,
    ) -> String {
        let mut prompt = String::new();

        // System message with tools
        prompt.push_str("<|im_start|>system\n");
        if let Some(sys) = system {
            prompt.push_str(sys);
            prompt.push_str("\n\n");
        }
        prompt.push_str(&tools.to_tool_prompt());
        prompt.push_str("<|im_end|>\n");

        // User message
        prompt.push_str("<|im_start|>user\n");
        prompt.push_str(user);
        prompt.push_str("<|im_end|>\n");

        prompt.push_str("<|im_start|>assistant\n");
        prompt
    }

    /// 带工具调用的流式对话
    pub fn chat_with_tools_stream<F>(
        &self,
        system: Option<&str>,
        user: &str,
        tools: &crate::tool::ToolRegistry,
        on_token: F,
    ) -> Result<String>
    where
        F: FnMut(&str),
    {
        let prompt = Self::format_prompt_with_tools(system, user, tools);
        self.generate_stream(&prompt, SamplingParams::default(), on_token)
    }
}

/// 对话消息
#[derive(Debug, Clone)]
pub struct Message {
    pub role: Role,
    pub content: String,
}

/// 消息角色
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    System,
    User,
    Assistant,
}

/// 支持连续对话的会话（复用 KV Cache）
pub struct ChatSession<'a> {
    llama: &'a Qwen3Llama,
    ctx: LlamaContext<'a>,
    messages: Vec<Message>,
    n_past: usize, // 已编码的 token 数量
}

impl<'a> ChatSession<'a> {
    /// 创建新会话
    pub fn new(llama: &'a Qwen3Llama) -> Result<Self> {
        let ctx = llama
            .model
            .new_context(&llama.backend, llama.ctx_params.clone())?;
        Ok(Self {
            llama,
            ctx,
            messages: Vec::new(),
            n_past: 0,
        })
    }

    /// 创建带系统提示的会话
    pub fn with_system(llama: &'a Qwen3Llama, system: &str) -> Result<Self> {
        let ctx = llama
            .model
            .new_context(&llama.backend, llama.ctx_params.clone())?;
        Ok(Self {
            llama,
            ctx,
            messages: vec![Message {
                role: Role::System,
                content: system.to_string(),
            }],
            n_past: 0,
        })
    }

    /// 流式发送消息（复用 KV Cache）
    pub fn send_stream<F>(&mut self, user_msg: &str, mut on_token: F) -> Result<String>
    where
        F: FnMut(&str),
    {
        // 添加用户消息
        self.messages.push(Message {
            role: Role::User,
            content: user_msg.to_string(),
        });

        // 只编码新增部分
        let new_prompt = self.build_incremental_prompt();
        let new_tokens = self
            .llama
            .model
            .str_to_token(&new_prompt, AddBos::Never)
            .map_err(|e| anyhow::anyhow!(e))?;

        // 首轮需要 BOS
        let tokens = if self.n_past == 0 {
            self.llama
                .model
                .str_to_token(&new_prompt, AddBos::Always)
                .map_err(|e| anyhow::anyhow!(e))?
        } else {
            new_tokens
        };

        let n_new = tokens.len();

        // 编码新 tokens
        let mut batch = LlamaBatch::new(512, 1);
        for (i, token) in tokens.iter().enumerate() {
            let pos = (self.n_past + i) as i32;
            batch.add(*token, pos, &[0], i == n_new - 1)?;
        }
        self.ctx.decode(&mut batch)?;
        self.n_past += n_new;

        // 生成回复
        let eos = self.llama.model.token_eos();
        let mut output = String::new();
        let max_ctx = self
            .llama
            .ctx_params
            .n_ctx()
            .map(NonZeroU32::get)
            .unwrap_or(0) as usize;

        loop {
            if max_ctx > 0 && self.n_past >= max_ctx.saturating_sub(1) {
                break;
            }

            let mut candidates = LlamaTokenDataArray::from_iter(
                self.ctx.candidates_ith(batch.n_tokens() - 1),
                false,
            );

            let seed = (self.n_past as u32)
                .wrapping_mul(1103515245)
                .wrapping_add(12345);
            let next_token = candidates.sample_token(seed);

            if next_token == eos {
                break;
            }

            if let Ok(piece) = self.llama.model.token_to_str(next_token, Special::Tokenize) {
                on_token(&piece);
                output.push_str(&piece);
            }

            batch.clear();
            batch.add(next_token, self.n_past as i32, &[0], true)?;
            self.n_past += 1;

            self.ctx.decode(&mut batch)?;
        }

        // 编码结束标记
        let end_tokens = self
            .llama
            .model
            .str_to_token("<|im_end|>\n", AddBos::Never)
            .map_err(|e| anyhow::anyhow!(e))?;
        batch.clear();
        for (i, token) in end_tokens.iter().enumerate() {
            batch.add(
                *token,
                (self.n_past + i) as i32,
                &[0],
                i == end_tokens.len() - 1,
            )?;
        }
        self.ctx.decode(&mut batch)?;
        self.n_past += end_tokens.len();

        // 保存助手回复
        self.messages.push(Message {
            role: Role::Assistant,
            content: output.clone(),
        });

        Ok(output)
    }

    /// 获取对话历史
    pub fn history(&self) -> &[Message] {
        &self.messages
    }

    /// 清空对话历史并重置 KV Cache
    pub fn clear(&mut self) -> Result<()> {
        self.messages.clear();
        self.n_past = 0;
        self.ctx = self
            .llama
            .model
            .new_context(&self.llama.backend, self.llama.ctx_params.clone())?;
        Ok(())
    }

    /// 构建增量 prompt（只返回最新一条用户消息部分）
    fn build_incremental_prompt(&self) -> String {
        // 如果是首次调用，返回完整 prompt
        if self.n_past == 0 {
            return self.build_full_prompt();
        }

        // 否则只返回新增的用户消息
        let mut prompt = String::new();
        prompt.push_str("<|im_start|>user\n");
        if let Some(msg) = self.messages.last() {
            prompt.push_str(&msg.content);
        }
        prompt.push_str("<|im_end|>\n");
        prompt.push_str("<|im_start|>assistant\n");
        prompt
    }

    /// 构建完整的 ChatML prompt
    fn build_full_prompt(&self) -> String {
        let mut prompt = String::new();
        for msg in &self.messages {
            match msg.role {
                Role::System => {
                    prompt.push_str("<|im_start|>system\n");
                    prompt.push_str(&msg.content);
                    prompt.push_str("<|im_end|>\n");
                }
                Role::User => {
                    prompt.push_str("<|im_start|>user\n");
                    prompt.push_str(&msg.content);
                    prompt.push_str("<|im_end|>\n");
                }
                Role::Assistant => {
                    prompt.push_str("<|im_start|>assistant\n");
                    prompt.push_str(&msg.content);
                    prompt.push_str("<|im_end|>\n");
                }
            }
        }
        prompt.push_str("<|im_start|>assistant\n");
        prompt
    }
}

#[cfg(test)]
mod tests {
    use super::{ChatSession, Qwen3Llama};
    use anyhow::Result;
    use std::io::Write;
    use std::path::PathBuf;

    fn model_path() -> PathBuf {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.pop();
        path.join("fuxi-quant-app")
            .join("public")
            .join("Qwen3-0.6B-Q8_0.gguf")
    }

    #[test]
    fn test_chat_stream() -> Result<()> {
        let llama = Qwen3Llama::load(model_path())?;

        println!("Streaming response:");
        let resp = llama.chat_stream(
            Some("你是一个量化高手"),
            "帮我写一个python交易策略,写一个趋势交易的策略,要有风控",
            |token| {
                print!("{}", token);
                std::io::stdout().flush().ok();
            },
        )?;
        println!(); // 换行

        assert!(!resp.trim().is_empty());
        Ok(())
    }

    #[test]
    fn test_multi_turn() -> Result<()> {
        let llama = Qwen3Llama::load(model_path())?;
        let mut session = ChatSession::with_system(&llama, "你是一个量化交易助手")?;

        std::thread::sleep(std::time::Duration::from_secs(3));

        // 第一轮对话
        println!("=== 第一轮 ===");
        let resp1 = session.send_stream("什么是均线策略?", |token| {
            print!("{}", token);
            std::io::stdout().flush().ok();
        })?;
        println!("\n");

        std::thread::sleep(std::time::Duration::from_secs(3));

        // 第二轮对话（模型应该记住上下文）
        println!("=== 第二轮 ===");
        let resp2 = session.send_stream("给我一个简单的代码示例", |token| {
            print!("{}", token);
            std::io::stdout().flush().ok();
        })?;
        println!("\n");

        std::thread::sleep(std::time::Duration::from_secs(3));

        assert!(!resp1.trim().is_empty());
        assert!(!resp2.trim().is_empty());

        // 查看历史记录
        println!("对话轮数: {}", session.history().len());
        Ok(())
    }

    #[test]
    fn test_function_calling() -> Result<()> {
        use crate::tool::{create_quant_tools, parse_tool_calls};

        let llama = Qwen3Llama::load(model_path())?;
        let tools = create_quant_tools();

        println!("=== Function Calling 测试 ===");
        println!("工具提示词:\n{}\n", tools.to_tool_prompt());

        let resp = llama.chat_with_tools_stream(
            Some("你是一个量化交易助手，请根据用户需求调用合适的工具。"),
            r#"
            - 帮我查一下苹果公司(AAPL)的股价.
            - 帮我查一下特斯拉公司的股价.
            "#,
            &tools,
            |token| {
                print!("{}", token);
                std::io::stdout().flush().ok();
            },
        )?;
        println!("\n");

        // 解析工具调用
        let calls = parse_tool_calls(&resp);
        println!("解析到 {} 个工具调用", calls.len());
        for call in &calls {
            println!("  - {}: {:?}", call.name, call.arguments);
        }

        assert!(!resp.trim().is_empty());
        Ok(())
    }
}
