use anyhow::{Context, Result};
use hf_hub::Repo;
use hf_hub::api::tokio::ApiBuilder;
use std::path::{Path, PathBuf};

/// 下载 Hugging Face 模型到指定目录
///
/// # 参数
/// - `model_name`: 模型名称 (例如 "bert-base-uncased", "sentence-transformers/all-MiniLM-L6-v2")
/// - `save_dir`: 保存目录
///
/// # 返回
/// 下载的所有文件路径列表
///
/// # 说明
/// - 已下载的文件不会重复下载（自动缓存）
/// - 会自动创建保存目录（如果不存在）
///
/// # 示例
/// ```ignore
/// let files = download_model("bert-base-uncased", "./models").await?;
/// for file in &files {
///     println!("Downloaded: {:?}", file);
/// }
/// ```
pub async fn download_model(model_name: &str, save_dir: impl AsRef<Path>) -> Result<Vec<PathBuf>> {
    let save_dir = save_dir.as_ref();

    // 创建保存目录
    std::fs::create_dir_all(save_dir)
        .with_context(|| format!("Failed to create directory: {:?}", save_dir))?;

    // 创建 API，设置缓存目录
    let api = ApiBuilder::new()
        .with_cache_dir(save_dir.to_path_buf())
        .build()
        .context("Failed to create HF API client")?;

    let repo = api.repo(Repo::model(model_name.to_string()));

    // 获取仓库文件列表
    let info = repo
        .info()
        .await
        .with_context(|| format!("Failed to get repo info for '{}'", model_name))?;

    let mut files = Vec::new();

    // 下载所有文件（已存在的会跳过）
    for sibling in &info.siblings {
        let path = repo
            .get(&sibling.rfilename)
            .await
            .with_context(|| format!("Failed to download '{}'", sibling.rfilename))?;
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

        std::fs::create_dir_all(&data_dir)?;

        let files = download_model(
            "hf-internal-testing/tiny-random-bert",
            data_dir.to_string_lossy().as_ref(),
        )
        .await;

        assert!(files.is_ok());
        let files = files.unwrap();
        assert!(!files.is_empty());

        for file in &files {
            println!("Downloaded: {:?}", file);
        }

        Ok(())
    }
}
