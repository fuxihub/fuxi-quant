use anyhow::Result;
use llama_cpp_2::{
    LogOptions,
    llama_backend::LlamaBackend,
    model::{LlamaModel, params::LlamaModelParams},
    send_logs_to_tracing,
};
use std::path::Path;

// ============================================================================
// 模型（GGUF 模型加载封装）
// ============================================================================

/// GGUF 模型封装（基于 llama-cpp-2）
pub struct Model {
    pub(crate) backend: LlamaBackend,
    pub(crate) model: LlamaModel,
}

impl Model {
    /// 默认 GPU 层数（999 = 全部放 GPU，Mac Metal 加速）
    const DEFAULT_GPU_LAYERS: u32 = 999;

    /// 从 GGUF 文件加载模型
    pub fn load(model_path: impl AsRef<Path>) -> Result<Self> {
        // 禁用 llama.cpp 底层日志输出
        send_logs_to_tracing(LogOptions::default().with_logs_enabled(false));

        let backend = LlamaBackend::init()?;
        let model_params = LlamaModelParams::default().with_n_gpu_layers(Self::DEFAULT_GPU_LAYERS);
        let model = LlamaModel::load_from_file(&backend, model_path.as_ref(), &model_params)?;

        Ok(Self { backend, model })
    }

    /// 获取模型训练时的上下文长度
    pub fn n_ctx_train(&self) -> u32 {
        self.model.n_ctx_train()
    }
}
