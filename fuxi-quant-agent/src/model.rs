use anyhow::Result;
use llama_cpp_2::{
    LogOptions,
    context::{LlamaContext, params::LlamaContextParams},
    llama_backend::LlamaBackend,
    llama_batch::LlamaBatch,
    model::{AddBos, LlamaModel, Special, params::LlamaModelParams},
    sampling::LlamaSampler,
    send_logs_to_tracing,
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

        let mut output = String::new();
        let mut n_cur = n_prompt;
        let max_ctx = self.ctx_params.n_ctx().map(NonZeroU32::get).unwrap_or(0) as usize;

        // 在循环外创建采样器（不包含 seed）
        let mut sampler = LlamaSampler::chain_simple([
            LlamaSampler::top_k(params.top_k),
            LlamaSampler::top_p(params.top_p, 1),
            LlamaSampler::min_p(params.min_p, 1),
            LlamaSampler::temp(params.temperature),
            LlamaSampler::dist(n_cur as u32),
        ]);

        loop {
            if max_ctx > 0 && n_cur >= max_ctx.saturating_sub(1) {
                break;
            }

            let next_token = sampler.sample(&ctx, batch.n_tokens() - 1);
            sampler.accept(next_token);

            // 只检查真正的 EOS，不检查 <|im_end|> 等其他 EOG token
            if next_token == self.model.token_eos() {
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

        // 记录 assistant 回复开始位置，用于后续清理 thinking 内容
        let assistant_start_pos = self.n_past;

        // 生成回复
        let mut output = String::new();
        let max_ctx = self
            .llama
            .ctx_params
            .n_ctx()
            .map(NonZeroU32::get)
            .unwrap_or(0) as usize;

        // 在循环外创建采样器（使用 Thinking 模式参数）
        let params = SamplingParams::thinking();
        let mut sampler = LlamaSampler::chain_simple([
            LlamaSampler::top_k(params.top_k),
            LlamaSampler::top_p(params.top_p, 1),
            LlamaSampler::min_p(params.min_p, 1),
            LlamaSampler::temp(params.temperature),
            LlamaSampler::dist(self.n_past as u32),
        ]);

        // 追踪 </think> 结束位置
        let mut think_end_pos: Option<usize> = None;

        loop {
            if max_ctx > 0 && self.n_past >= max_ctx.saturating_sub(1) {
                break;
            }

            let next_token = sampler.sample(&self.ctx, batch.n_tokens() - 1);
            sampler.accept(next_token);

            // 只检查真正的 EOS
            if next_token == self.llama.model.token_eos() {
                break;
            }

            if let Ok(piece) = self.llama.model.token_to_str(next_token, Special::Tokenize) {
                on_token(&piece);
                output.push_str(&piece);

                // 检测 </think> 结束位置（包含后面的换行）
                if think_end_pos.is_none() && output.contains("</think>\n\n") {
                    think_end_pos = Some(self.n_past + 1); // +1 因为当前 token 还未计入
                }
            }

            batch.clear();
            batch.add(next_token, self.n_past as i32, &[0], true)?;
            self.n_past += 1;

            self.ctx.decode(&mut batch)?;
        }

        // 官方最佳实践：多轮对话中，历史记录不应包含 thinking 内容
        // 从 KV Cache 中删除 thinking 部分，只保留最终输出
        if let Some(think_end) = think_end_pos {
            let thinking_len = think_end - assistant_start_pos;
            if thinking_len > 0 {
                // 删除 thinking 内容的 KV Cache
                let _ = self.ctx.clear_kv_cache_seq(
                    Some(0), // sequence id
                    Some(assistant_start_pos as u32),
                    Some(think_end as u32),
                );
                // 将后续内容位置前移
                let _ = self.ctx.kv_cache_seq_add(
                    0,
                    Some(think_end as u32),
                    Some(self.n_past as u32),
                    -(thinking_len as i32),
                );
                self.n_past -= thinking_len;
            }
        }

        // 生成结束后，编码 <|im_end|>\n 到 KV Cache，确保下一轮对话正确
        let end_tokens = self
            .llama
            .model
            .str_to_token("<|im_end|>\n", AddBos::Never)
            .map_err(|e| anyhow::anyhow!(e))?;
        batch.clear();
        for (i, token) in end_tokens.iter().enumerate() {
            let pos = (self.n_past + i) as i32;
            batch.add(*token, pos, &[0], i == end_tokens.len() - 1)?;
        }
        self.ctx.decode(&mut batch)?;
        self.n_past += end_tokens.len();

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
