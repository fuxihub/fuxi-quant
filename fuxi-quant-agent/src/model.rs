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

    /// 流式对话（每生成一个 token 调用回调）
    pub fn chat<F>(&self, system: Option<&str>, user: &str, on_token: F) -> Result<String>
    where
        F: FnMut(&str),
    {
        let prompt = Self::format_prompt(system, user);
        self.generate(&prompt, SamplingParams::default(), on_token)
    }

    /// 流式生成（内部方法）
    fn generate<F>(&self, prompt: &str, params: SamplingParams, mut on_token: F) -> Result<String>
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
                on_token(&piece);
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
    pub fn chat_with_tools<F>(
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
        self.generate(&prompt, SamplingParams::default(), on_token)
    }
}

/// 支持连续对话的会话（复用 KV Cache）
pub struct ChatSession<'a> {
    llama: &'a Qwen3Llama,
    ctx: LlamaContext<'a>,
    system: Option<String>,
    n_past: usize,
}

impl<'a> ChatSession<'a> {
    /// 创建会话
    pub fn new(llama: &'a Qwen3Llama, system: Option<&str>) -> Result<Self> {
        let ctx = llama
            .model
            .new_context(&llama.backend, llama.ctx_params.clone())?;
        Ok(Self {
            llama,
            ctx,
            system: system.map(String::from),
            n_past: 0,
        })
    }

    /// 发送消息（流式回调）
    pub fn chat<F>(&mut self, user_msg: &str, mut on_token: F) -> Result<String>
    where
        F: FnMut(&str),
    {
        let prompt = self.build_prompt(user_msg);
        let add_bos = if self.n_past == 0 {
            AddBos::Always
        } else {
            AddBos::Never
        };
        let tokens = self
            .llama
            .model
            .str_to_token(&prompt, add_bos)
            .map_err(|e| anyhow::anyhow!(e))?;
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

        Ok(output)
    }

    /// 构建 prompt
    fn build_prompt(&self, user_msg: &str) -> String {
        let mut prompt = String::new();
        if self.n_past == 0
            && let Some(sys) = &self.system
        {
            prompt.push_str("<|im_start|>system\n");
            prompt.push_str(sys);
            prompt.push_str("<|im_end|>\n");
        }
        prompt.push_str("<|im_start|>user\n");
        prompt.push_str(user_msg);
        prompt.push_str("<|im_end|>\n");
        prompt.push_str("<|im_start|>assistant\n");
        prompt
    }
}
