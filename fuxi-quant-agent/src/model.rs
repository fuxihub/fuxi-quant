use anyhow::{Result, bail};
use candle_core::{DType, Device, Tensor};
use candle_nn::VarBuilder;
use candle_transformers::generation::LogitsProcessor;
use candle_transformers::models::qwen3::{Config, ModelForCausalLM};
use hf_hub::Repo;
use hf_hub::api::tokio::ApiBuilder;
use std::io::Write;
use std::path::{Path, PathBuf};
use tokenizers::Tokenizer;

const MARKER_FILE: &str = ".downloaded";

pub async fn download_model(save_dir: impl AsRef<Path>, model_name: &str) -> Result<()> {
    let save_dir = save_dir.as_ref();
    std::fs::create_dir_all(save_dir)?;

    let api = ApiBuilder::new().with_progress(false).build()?;
    let repo = api.repo(Repo::model(model_name.to_string()));
    let info = repo.info().await?;

    for sibling in &info.siblings {
        let cached = repo.get(&sibling.rfilename).await?;
        let target = save_dir.join(&sibling.rfilename);

        if !target.exists() {
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::copy(&cached, &target)?;
        }
    }

    std::fs::write(save_dir.join(MARKER_FILE), "ok")?;
    Ok(())
}

pub fn get_model_file(model_dir: impl AsRef<Path>, filename: &str) -> Result<PathBuf> {
    let model_dir = model_dir.as_ref();

    if !model_dir.join(MARKER_FILE).exists() {
        bail!("Model not downloaded: {:?}", model_dir);
    }

    let path = model_dir.join(filename);
    if !path.exists() {
        bail!("File not found: {:?}", path);
    }

    Ok(path)
}

/// 获取设备 (优先使用 Metal)
fn get_device() -> Result<Device> {
    Ok(Device::new_metal(0)?)
}

pub struct Qwen3Chat {
    model: ModelForCausalLM,
    tokenizer: Tokenizer,
    device: Device,
}

impl Qwen3Chat {
    /// 加载 Qwen3 模型
    pub fn load(dir: impl AsRef<Path>) -> Result<Self> {
        let dir = dir.as_ref();
        let device = get_device()?;
        let dtype = DType::BF16;

        println!("Using device: {:?}", device);

        let config: Config =
            serde_json::from_str(&std::fs::read_to_string(dir.join("config.json"))?)?;

        let tokenizer =
            Tokenizer::from_file(dir.join("tokenizer.json")).map_err(|e| anyhow::anyhow!(e))?;

        let vb = unsafe {
            VarBuilder::from_mmaped_safetensors(&[dir.join("model.safetensors")], dtype, &device)?
        };

        let model = ModelForCausalLM::new(&config, vb)?;

        Ok(Self {
            model,
            tokenizer,
            device,
        })
    }

    /// 格式化对话为 ChatML 格式
    pub fn format_prompt(&self, system: Option<&str>, user: &str) -> String {
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

    /// 简单对话
    pub fn chat(&mut self, user_input: &str, max_tokens: usize) -> Result<String> {
        let prompt = self.format_prompt(None, user_input);
        self.generate(&prompt, max_tokens)
    }

    /// 带系统提示的对话
    pub fn chat_with_system(
        &mut self,
        system: &str,
        user_input: &str,
        max_tokens: usize,
    ) -> Result<String> {
        let prompt = self.format_prompt(Some(system), user_input);
        self.generate(&prompt, max_tokens)
    }

    /// 生成回复
    pub fn generate(&mut self, prompt: &str, max_tokens: usize) -> Result<String> {
        let mut logits_processor = LogitsProcessor::new(42, Some(0.7), Some(0.9));

        let encoding = self
            .tokenizer
            .encode(prompt, true)
            .map_err(|e| anyhow::anyhow!(e))?;
        let mut tokens: Vec<u32> = encoding.get_ids().to_vec();

        let eos_token = self.tokenizer.token_to_id("<|im_end|>").unwrap_or(151645);

        let mut output = String::new();
        self.model.clear_kv_cache();

        for index in 0..max_tokens {
            let context_size = if index > 0 { 1 } else { tokens.len() };
            let start_pos = tokens.len().saturating_sub(context_size);
            let input_ids = &tokens[start_pos..];

            let input = Tensor::new(input_ids, &self.device)?.unsqueeze(0)?;
            let logits = self.model.forward(&input, start_pos)?;
            let logits = logits.squeeze(0)?.squeeze(0)?.to_dtype(DType::F32)?;

            let next_token = logits_processor.sample(&logits)?;
            tokens.push(next_token);

            if next_token == eos_token {
                break;
            }

            if let Ok(text) = self.tokenizer.decode(&[next_token], false) {
                output.push_str(&text);
                print!("{}", text);
                std::io::stdout().flush()?;
            }
        }

        println!();
        Ok(output)
    }
}

#[cfg(test)]
mod tests {
    use super::{Qwen3Chat, download_model};
    use anyhow::Result;
    use std::path::PathBuf;

    fn get_model_dir() -> PathBuf {
        let mut model_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        model_dir.pop();
        model_dir.push(".cache");
        model_dir.push("agent");
        model_dir.push("models");
        model_dir.push("Qwen3-0.6B");
        model_dir
    }

    #[tokio::test]
    async fn test_download_model() -> Result<()> {
        let model_dir = get_model_dir();
        download_model(&model_dir, "Qwen/Qwen3-0.6B").await?;
        assert!(model_dir.join("config.json").exists());
        assert!(model_dir.join("model.safetensors").exists());
        Ok(())
    }

    #[test]
    fn test_qwen3_chat() -> Result<()> {
        let model_dir = get_model_dir();

        if !model_dir.join("config.json").exists() {
            println!("Model not downloaded, skipping test");
            return Ok(());
        }

        println!("Loading model from {:?}", model_dir);
        let mut chat = Qwen3Chat::load(&model_dir)?;

        println!("\n--- Test: Simple chat ---");
        let response = chat.chat("你好", 1000)?;
        println!("Response: {}", response);
        assert!(!response.is_empty());

        Ok(())
    }
}
