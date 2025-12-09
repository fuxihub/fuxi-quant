use anyhow::{Result, bail};
use hf_hub::Repo;
use hf_hub::api::tokio::ApiBuilder;
use std::path::{Path, PathBuf};

/// 明确跳过的文件扩展名（其他框架的权重格式）
const SKIP_EXTENSIONS: &[&str] = &[
    ".bin",     // PyTorch 格式
    ".msgpack", // Flax 格式
    ".h5",      // TensorFlow/Keras 格式
    ".onnx",    // ONNX 格式
    ".ot",      // rust-bert 格式
    ".pt",      // PyTorch checkpoint
    ".pth",     // PyTorch checkpoint
];

/// 明确跳过的文件名
const SKIP_FILES: &[&str] = &[
    ".gitattributes",
    "README.md",
    "LICENSE",
    "LICENSE.md",
    "LICENSE.txt",
    "NOTICE",
    "NOTICE.md",
];

/// 判断文件是否需要下载（排除法：只跳过明确不需要的）
fn should_download(filename: &str) -> bool {
    // 跳过隐藏文件
    if filename.starts_with('.') {
        return false;
    }

    // 跳过明确不需要的文件名
    if SKIP_FILES.contains(&filename) {
        return false;
    }

    // 跳过其他框架的权重格式
    if SKIP_EXTENSIONS.iter().any(|ext| filename.ends_with(ext)) {
        return false;
    }

    true
}

/// 下载 Hugging Face 模型
///
/// # 参数
/// - `save_dir`: 模型文件保存目录
/// - `model_name`: HF 模型名 (例如 "bert-base-uncased", "sentence-transformers/all-MiniLM-L6-v2")
///
/// # 返回
/// 下载的文件路径列表（仅包含 candle 需要的文件）
pub async fn download_model(save_dir: impl AsRef<Path>, model_name: &str) -> Result<Vec<PathBuf>> {
    let save_dir = save_dir.as_ref();

    std::fs::create_dir_all(save_dir)?;

    let api = ApiBuilder::new()
        .with_cache_dir(save_dir.to_path_buf())
        .build()?;

    let repo = api.repo(Repo::model(model_name.to_string()));

    let info = repo.info().await?;

    let mut files = Vec::new();

    for sibling in &info.siblings {
        if !should_download(&sibling.rfilename) {
            continue;
        }

        let path = repo.get(&sibling.rfilename).await?;
        files.push(path);
    }

    if files.is_empty() {
        bail!("No model files found for '{}'", model_name);
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
        let mut save_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        save_dir.pop();
        save_dir.push(".cache");
        save_dir.push("agent");
        save_dir.push("models");

        let files = download_model(&save_dir, "hf-internal-testing/tiny-random-bert").await?;

        assert!(!files.is_empty());

        for file in &files {
            println!("Downloaded: {:?}", file);
        }

        Ok(())
    }
}
