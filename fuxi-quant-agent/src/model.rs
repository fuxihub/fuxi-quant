use anyhow::{Result, ensure};
use llama_cpp_2::{
    context::{LlamaContext, params::LlamaContextParams},
    llama_backend::LlamaBackend,
    llama_batch::LlamaBatch,
    model::{AddBos, LlamaModel, Special, params::LlamaModelParams},
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
}

impl Qwen3Llama {
    /// 默认上下文长度（Qwen3-0.6B 最大支持 32768）
    const DEFAULT_CTX: u32 = 32768;
    /// 默认 GPU 层数（999 = 全部放 GPU，Mac Metal 加速）
    const DEFAULT_GPU_LAYERS: u32 = 999;

    /// 从 GGUF 文件加载模型（使用默认参数）
    pub fn load(model_path: impl AsRef<Path>) -> Result<Self> {
        Self::load_with_params(model_path, Self::DEFAULT_CTX, Self::DEFAULT_GPU_LAYERS)
    }

    /// 从 GGUF 文件加载模型（自定义参数）
    ///
    /// - `model_path`: GGUF 文件路径
    /// - `n_ctx`: 上下文长度
    /// - `n_gpu_layers`: GPU 层数（Mac/Metal 可设 999，CPU 可设 0）
    pub fn load_with_params(
        model_path: impl AsRef<Path>,
        n_ctx: u32,
        n_gpu_layers: u32,
    ) -> Result<Self> {
        ensure!(n_ctx > 0, "n_ctx must be > 0");
        let backend = LlamaBackend::init()?;
        let model_params = LlamaModelParams::default().with_n_gpu_layers(n_gpu_layers);
        let model = LlamaModel::load_from_file(&backend, model_path.as_ref(), &model_params)?;

        let ctx_params = LlamaContextParams::default()
            .with_n_ctx(NonZeroU32::new(n_ctx))
            .with_n_batch(512);

        Ok(Self {
            backend,
            model,
            ctx_params,
        })
    }

    /// 生成回复（自动构造 ChatML 风格提示词，使用默认采样参数）
    pub fn chat(&self, system: Option<&str>, user: &str, max_new_tokens: usize) -> Result<String> {
        self.chat_with_params(system, user, max_new_tokens, SamplingParams::default())
    }

    /// 生成回复（自动构造 ChatML 风格提示词，指定采样参数）
    pub fn chat_with_params(
        &self,
        system: Option<&str>,
        user: &str,
        max_new_tokens: usize,
        params: SamplingParams,
    ) -> Result<String> {
        let prompt = Self::format_prompt(system, user);
        self.generate(&prompt, max_new_tokens, params)
    }

    /// 流式生成回复（每生成一个 token 调用回调）
    pub fn chat_stream<F>(
        &self,
        system: Option<&str>,
        user: &str,
        max_new_tokens: usize,
        on_token: F,
    ) -> Result<String>
    where
        F: FnMut(&str),
    {
        let prompt = Self::format_prompt(system, user);
        self.generate_stream(&prompt, max_new_tokens, SamplingParams::default(), on_token)
    }

    /// 使用已有 prompt 生成（带采样参数）
    pub fn generate(
        &self,
        prompt: &str,
        max_new_tokens: usize,
        params: SamplingParams,
    ) -> Result<String> {
        let mut ctx: LlamaContext = self
            .model
            .new_context(&self.backend, self.ctx_params.clone())?;

        // 编码提示词
        let tokens = self
            .model
            .str_to_token(prompt, AddBos::Always)
            .map_err(|e| anyhow::anyhow!(e))?;
        let n_prompt = tokens.len();

        let mut batch = LlamaBatch::new(512, 1);
        for (i, token) in tokens.iter().enumerate() {
            batch.add(*token, i as i32, &[0], i == n_prompt - 1)?;
        }

        // 先前向一次得到首个 logits
        ctx.decode(&mut batch)?;

        let eos = self.model.token_eos();
        let mut output = String::new();
        let mut n_cur = n_prompt;
        let max_ctx = self.ctx_params.n_ctx().map(NonZeroU32::get).unwrap_or(0) as usize;

        for _ in 0..max_new_tokens {
            if max_ctx > 0 && n_cur >= max_ctx.saturating_sub(1) {
                break;
            }

            // 温度采样（官方推荐，避免贪心导致重复）
            let mut candidates =
                LlamaTokenDataArray::from_iter(ctx.candidates_ith(batch.n_tokens() - 1), false);

            // 使用随机 seed 进行采样，避免贪心导致的重复
            // TODO: 后续可用 LlamaSampler 实现完整的 top_k/top_p/temp 采样链
            let _ = params; // 暂时未用，保留接口兼容
            let seed = (n_cur as u32).wrapping_mul(1103515245).wrapping_add(12345);
            let next_token = candidates.sample_token(seed);

            if next_token == eos {
                break;
            }

            if let Ok(piece) = self.model.token_to_str(next_token, Special::Tokenize) {
                output.push_str(&piece);
            }

            // 准备下一个 token 的批次
            batch.clear();
            batch.add(next_token, n_cur as i32, &[0], true)?;
            n_cur += 1;

            ctx.decode(&mut batch)?;
        }

        Ok(output)
    }

    /// 流式生成（每生成一个 token 调用回调）
    pub fn generate_stream<F>(
        &self,
        prompt: &str,
        max_new_tokens: usize,
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

        let mut batch = LlamaBatch::new(512, 1);
        for (i, token) in tokens.iter().enumerate() {
            batch.add(*token, i as i32, &[0], i == n_prompt - 1)?;
        }

        ctx.decode(&mut batch)?;

        let eos = self.model.token_eos();
        let mut output = String::new();
        let mut n_cur = n_prompt;
        let max_ctx = self.ctx_params.n_ctx().map(NonZeroU32::get).unwrap_or(0) as usize;

        for _ in 0..max_new_tokens {
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

    /// 发送消息并获取回复
    pub fn send(&mut self, user_msg: &str, max_new_tokens: usize) -> Result<String> {
        self.send_stream(user_msg, max_new_tokens, |_| {})
    }

    /// 流式发送消息（复用 KV Cache）
    pub fn send_stream<F>(
        &mut self,
        user_msg: &str,
        max_new_tokens: usize,
        mut on_token: F,
    ) -> Result<String>
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

        for _ in 0..max_new_tokens {
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
    fn test_chat() -> Result<()> {
        let llama = Qwen3Llama::load(model_path())?;

        let resp = llama.chat(
            Some("你是一个量化高手"),
            "帮我写一个python交易策略,写一个趋势交易的策略,要有风控",
            32768,
        )?;
        println!("Response: {}", resp);

        assert!(!resp.trim().is_empty());
        Ok(())
    }

    #[test]
    fn test_chat_stream() -> Result<()> {
        let llama = Qwen3Llama::load(model_path())?;

        println!("Streaming response:");
        let resp = llama.chat_stream(
            Some("你是一个量化高手"),
            "帮我写一个python交易策略,写一个趋势交易的策略,要有风控",
            32768,
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
        let resp1 = session.send_stream("什么是均线策略?", 32768, |token| {
            print!("{}", token);
            std::io::stdout().flush().ok();
        })?;
        println!("\n");

        std::thread::sleep(std::time::Duration::from_secs(3));

        // 第二轮对话（模型应该记住上下文）
        println!("=== 第二轮 ===");
        let resp2 = session.send_stream("给我一个简单的代码示例", 32768, |token| {
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
}
