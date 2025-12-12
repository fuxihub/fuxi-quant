use anyhow::Result;
use llama_cpp_2::{
    LogOptions,
    llama_backend::LlamaBackend,
    model::{LlamaModel, params::LlamaModelParams},
    send_logs_to_tracing,
};
use std::path::Path;

/// 模型
pub struct Model {
    pub(crate) backend: LlamaBackend,
    pub(crate) model: LlamaModel,
}

impl Model {
    /// 加载模型
    pub fn load(model_path: impl AsRef<Path>) -> Result<Self> {
        send_logs_to_tracing(LogOptions::default().with_logs_enabled(false));

        let backend = LlamaBackend::init()?;
        let model_params = LlamaModelParams::default().with_n_gpu_layers(999);
        let model = LlamaModel::load_from_file(&backend, model_path.as_ref(), &model_params)?;

        Ok(Self { backend, model })
    }
}
