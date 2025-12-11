use crate::agent::Agent;
use crate::model::Model;
use anyhow::Result;
use llama_cpp_2::{
    context::{LlamaContext, params::LlamaContextParams},
    llama_batch::LlamaBatch,
    model::{AddBos, Special},
    sampling::LlamaSampler,
};
use std::{io::Write, num::NonZeroU32, sync::Arc};

const BATCH_SIZE: usize = 512;

pub struct Qwen3Agent {
    model: Arc<Model>,
    sys_prompt: Option<String>,
    ctx_len: Option<NonZeroU32>,
    ctx: Option<LlamaContext<'static>>,
    n_cur: usize,
    think_start: usize,
}

impl Agent for Qwen3Agent {
    fn new(model: Arc<Model>, sys_prompt: Option<String>, ctx_len: u32) -> impl Agent {
        Self {
            model,
            sys_prompt,
            ctx_len: NonZeroU32::new(ctx_len),
            ctx: None,
            n_cur: 0,
            think_start: 0,
        }
    }

    fn chat<F>(&mut self, message: &str, mut on_token: Option<F>) -> Result<String>
    where
        F: FnMut(&str),
    {
        let mut output = String::new();
        output.push_str("<think>");

        let is_first_turn = self.ctx.is_none();

        if is_first_turn {
            self.init_context()?;
        }

        let prompt = if is_first_turn {
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
            let mut p = String::new();
            p.push_str("<|im_start|>user\n");
            p.push_str(message);
            p.push_str("<|im_end|>\n");
            p.push_str("<|im_start|>assistant\n<think>");
            p
        };

        let add_bos = if is_first_turn {
            AddBos::Always
        } else {
            AddBos::Never
        };

        let tokens = self
            .model
            .model
            .str_to_token(&prompt, add_bos)
            .map_err(|e| anyhow::anyhow!(e))?;

        self.encode_tokens(&tokens)?;
        self.think_start = self.n_cur;

        let max_ctx = self.ctx_len.map(NonZeroU32::get).unwrap_or(0) as usize;

        let mut sampler = LlamaSampler::chain_simple([
            LlamaSampler::top_k(20),
            LlamaSampler::top_p(0.95, 1),
            LlamaSampler::temp(0.6),
            LlamaSampler::dist(self.n_cur as u32),
        ]);

        let ctx = self.ctx.as_mut().unwrap();
        let mut batch = LlamaBatch::new(BATCH_SIZE, 1);

        loop {
            if max_ctx > 0 && self.n_cur >= max_ctx.saturating_sub(1) {
                break;
            }

            let next_token = sampler.sample(ctx, -1);
            sampler.accept(next_token);

            if next_token == self.model.model.token_eos() {
                break;
            }

            if let Ok(piece) = self.model.model.token_to_str(next_token, Special::Tokenize) {
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
            batch.add(next_token, self.n_cur as i32, &[0], true)?;
            self.n_cur += 1;

            ctx.decode(&mut batch)?;
        }

        self.clear_thinking_kv()?;

        let im_end_tokens = self
            .model
            .model
            .str_to_token("<|im_end|>\n", AddBos::Never)
            .map_err(|e| anyhow::anyhow!(e))?;
        self.encode_tokens(&im_end_tokens)?;

        Ok(output)
    }

    fn reset(&mut self) {
        self.ctx = None;
        self.n_cur = 0;
        self.think_start = 0;
    }
}

impl Qwen3Agent {
    fn init_context(&mut self) -> Result<()> {
        let model_ptr = Arc::as_ptr(&self.model);
        let model_ref: &'static Model = unsafe { &*model_ptr };

        let ctx = model_ref.model.new_context(
            &model_ref.backend,
            LlamaContextParams::default()
                .with_n_ctx(self.ctx_len)
                .with_n_batch(BATCH_SIZE as u32),
        )?;

        self.ctx = Some(ctx);
        self.n_cur = 0;
        Ok(())
    }

    fn encode_tokens(&mut self, tokens: &[llama_cpp_2::token::LlamaToken]) -> Result<()> {
        let ctx = self.ctx.as_mut().unwrap();
        let n_tokens = tokens.len();
        let mut batch = LlamaBatch::new(BATCH_SIZE, 1);

        for chunk_start in (0..n_tokens).step_by(BATCH_SIZE) {
            batch.clear();
            let chunk_end = (chunk_start + BATCH_SIZE).min(n_tokens);
            for (i, &token) in tokens.iter().enumerate().take(chunk_end).skip(chunk_start) {
                let pos = self.n_cur + i;
                let is_last = i == n_tokens - 1;
                batch.add(token, pos as i32, &[0], is_last)?;
            }
            ctx.decode(&mut batch)?;
        }

        self.n_cur += n_tokens;
        Ok(())
    }

    fn clear_thinking_kv(&mut self) -> Result<()> {
        if self.think_start >= self.n_cur {
            return Ok(());
        }

        let ctx = self.ctx.as_mut().unwrap();
        let think_len = self.n_cur - self.think_start;

        ctx.clear_kv_cache_seq(
            Some(0),
            Some(self.think_start as u32),
            Some(self.n_cur as u32),
        )?;

        ctx.kv_cache_seq_add(0, Some(self.n_cur as u32), None, -(think_len as i32))?;

        self.n_cur = self.think_start;

        Ok(())
    }
}
