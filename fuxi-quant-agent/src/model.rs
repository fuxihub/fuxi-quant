use anyhow::{Result, bail};
use hf_hub::Repo;
use hf_hub::api::tokio::ApiBuilder;
use std::path::{Path, PathBuf};

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

#[cfg(test)]
mod tests {
    use super::{download_model, get_model_file};
    use anyhow::Result;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_download_and_get() -> Result<()> {
        let mut model_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        model_dir.pop();
        model_dir.push(".cache");
        model_dir.push("agent");
        model_dir.push("models");
        model_dir.push("tiny-random-bert");

        download_model(&model_dir, "hf-internal-testing/tiny-random-bert").await?;

        let config = get_model_file(&model_dir, "config.json")?;
        let weights = get_model_file(&model_dir, "model.safetensors")?;

        println!("Config: {:?}", config);
        println!("Weights: {:?}", weights);

        assert!(config.exists());
        assert!(weights.exists());

        Ok(())
    }
}
