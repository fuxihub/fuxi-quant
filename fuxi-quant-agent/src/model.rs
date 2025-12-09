use anyhow::Result;
use hf_hub::Repo;
use hf_hub::api::tokio::ApiBuilder;
use std::path::{Path, PathBuf};

pub async fn download_model(model_name: &str, save_dir: impl AsRef<Path>) -> Result<Vec<PathBuf>> {
    let save_dir = save_dir.as_ref();

    std::fs::create_dir_all(save_dir)?;

    let api = ApiBuilder::new()
        .with_cache_dir(save_dir.to_path_buf())
        .build()?;

    let repo = api.repo(Repo::model(model_name.to_string()));

    let info = repo.info().await?;

    let mut files = Vec::new();

    for sibling in &info.siblings {
        let path = repo.get(&sibling.rfilename).await?;
        files.push(path);
    }

    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::download_model;
    use anyhow::Result;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_download_model() -> Result<()> {
        let mut data_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        data_dir.pop();
        data_dir.push(".cache");
        data_dir.push("agent");
        data_dir.push("models");
        data_dir.push("demo");

        let files = download_model("hf-internal-testing/tiny-random-bert", &data_dir).await;

        assert!(files.is_ok());
        let files = files.unwrap();
        assert!(!files.is_empty());

        for file in &files {
            println!("Downloaded: {:?}", file);
        }

        Ok(())
    }
}
