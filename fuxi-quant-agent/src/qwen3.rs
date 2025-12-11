use crate::agent::{Agent, StreamEvent};
use crate::model::Model;
use anyhow::Result;
use llama_cpp_2::{
    context::{LlamaContext, params::LlamaContextParams},
    llama_batch::LlamaBatch,
    model::{AddBos, Special},
    sampling::LlamaSampler,
};
use std::{io::Write, num::NonZeroU32};

pub struct Qwen3Agent {
    model: &'static Model,
    sys_prompt: Option<String>,
    ctx: LlamaContext<'static>,
    n_cur: usize,
    is_first_turn: bool,
}

// Safety: LlamaContext 内部使用 NonNull 指针，但 llama.cpp 的 context 操作是线程安全的
unsafe impl Send for Qwen3Agent {}
unsafe impl Sync for Qwen3Agent {}

impl Qwen3Agent {
    pub fn create(model: &'static Model, sys_prompt: Option<String>, ctx_len: u32) -> Result<Self> {
        let ctx = model.model.new_context(
            &model.backend,
            LlamaContextParams::default()
                .with_n_ctx(NonZeroU32::new(ctx_len))
                .with_n_batch(512),
        )?;

        Ok(Self {
            model,
            sys_prompt,
            ctx,
            n_cur: 0,
            is_first_turn: true,
        })
    }
}

impl Agent for Qwen3Agent {
    fn new(model: &'static Model, sys_prompt: Option<String>, ctx_len: u32) -> Result<impl Agent> {
        Self::create(model, sys_prompt, ctx_len)
    }

    fn chat<F>(&mut self, message: &str, mut on_event: F) -> Result<()>
    where
        F: FnMut(StreamEvent),
    {
        on_event(StreamEvent::ThinkBegin);

        // 构建 prompt
        let prompt = if self.is_first_turn {
            let mut p = String::new();
            if let Some(sys) = &self.sys_prompt {
                p.push_str("<|im_start|>system\n");
                p.push_str(sys);
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

        // 编码并处理 prompt tokens
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

        // 采样生成
        let mut sampler = LlamaSampler::chain_simple([
            LlamaSampler::top_k(20),
            LlamaSampler::top_p(0.95, 1),
            LlamaSampler::temp(0.6),
            LlamaSampler::dist(self.n_cur as u32),
        ]);

        loop {
            let next_token = sampler.sample(ctx, -1);
            sampler.accept(next_token);

            if next_token == self.model.model.token_eos() {
                break;
            }

            if let Ok(piece) = self.model.model.token_to_str(next_token, Special::Tokenize) {
                if cfg!(debug_assertions) {
                    print!("{piece}");
                    let _ = std::io::stdout().flush();
                }
                on_event(StreamEvent::Token(piece));
            }

            batch.clear();
            batch.add(next_token, self.n_cur as i32, &[0], true)?;
            self.n_cur += 1;
            ctx.decode(&mut batch)?;
        }

        // 清除 thinking 内容的 KV cache
        if think_start < self.n_cur {
            let think_len = self.n_cur - think_start;
            ctx.clear_kv_cache_seq(Some(0), Some(think_start as u32), Some(self.n_cur as u32))?;
            ctx.kv_cache_seq_add(0, Some(self.n_cur as u32), None, -(think_len as i32))?;
            self.n_cur = think_start;
        }

        // 编码 <|im_end|>\n
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
