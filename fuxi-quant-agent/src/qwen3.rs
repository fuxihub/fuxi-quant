use crate::agent::Agent;
use crate::model::Model;
use anyhow::Result;
use llama_cpp_2::{
    context::params::LlamaContextParams,
    llama_batch::LlamaBatch,
    model::{AddBos, Special},
    sampling::LlamaSampler,
};
use std::{io::Write, num::NonZeroU32};

pub struct Qwen3Agent {
    sys_prompt: Option<String>,
    ctx_len: Option<NonZeroU32>,
}

impl Agent for Qwen3Agent {
    fn new(sys_prompt: Option<String>, ctx_len: u32) -> impl Agent {
        Self {
            sys_prompt,
            ctx_len: NonZeroU32::new(ctx_len),
        }
    }

    fn chat<F>(&self, model: &Model, message: &str, mut on_token: Option<F>) -> Result<String>
    where
        F: FnMut(&str),
    {
        let mut prompt = String::new();
        if let Some(sys) = &self.sys_prompt {
            prompt.push_str("<|im_start|>system\n");
            prompt.push_str(sys);
            prompt.push_str("<|im_end|>\n");
        }
        prompt.push_str("<|im_start|>user\n");
        prompt.push_str(message);
        prompt.push_str("<|im_end|>\n");
        prompt.push_str("<|im_start|>assistant\n<think>");

        let mut ctx = model.model.new_context(
            &model.backend,
            LlamaContextParams::default()
                .with_n_ctx(self.ctx_len)
                .with_n_batch(512),
        )?;

        let tokens = model
            .model
            .str_to_token(&prompt, AddBos::Always)
            .map_err(|e| anyhow::anyhow!(e))?;
        let n_prompt = tokens.len();

        // 分批处理 prompt tokens
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
        let max_ctx = self.ctx_len.map(NonZeroU32::get).unwrap_or(0) as usize;

        // 创建采样器 (Qwen3 Thinking: temp=0.6, top_p=0.95, top_k=20)
        let mut sampler = LlamaSampler::chain_simple([
            LlamaSampler::top_k(20),
            LlamaSampler::top_p(0.95, 1),
            LlamaSampler::temp(0.6),
            LlamaSampler::dist(n_cur as u32),
        ]);

        loop {
            if max_ctx > 0 && n_cur >= max_ctx.saturating_sub(1) {
                break;
            }

            let next_token = sampler.sample(&ctx, batch.n_tokens() - 1);
            sampler.accept(next_token);

            if next_token == model.model.token_eos() {
                break;
            }

            if let Ok(piece) = model.model.token_to_str(next_token, Special::Tokenize) {
                if let Some(ref mut callback) = on_token {
                    callback(&piece);
                }
                output.push_str(&piece);
                if cfg!(debug_assertions) {
                    print!("{piece}");
                    let _ = std::io::stdout().flush();
                }
            }

            batch.clear();
            batch.add(next_token, n_cur as i32, &[0], true)?;
            n_cur += 1;

            ctx.decode(&mut batch)?;
        }

        Ok(output)
    }
}
