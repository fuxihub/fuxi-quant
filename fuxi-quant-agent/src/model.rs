use anyhow::Result;
use llama_cpp_2::{
    LogOptions,
    context::{LlamaContext, params::LlamaContextParams},
    llama_backend::LlamaBackend,
    llama_batch::LlamaBatch,
    model::{AddBos, LlamaModel, Special, params::LlamaModelParams},
    sampling::LlamaSampler,
    send_logs_to_tracing,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs,
    io::{self, Write},
    num::NonZeroU32,
    path::Path,
    sync::Arc,
};

// ============================================================================
// YAML 配置结构体
// ============================================================================

/// 模型配置文件根结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelsConfig {
    pub models: Vec<ModelDefinition>,
}

impl ModelsConfig {
    /// 从 YAML 文件加载配置
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let content = fs::read_to_string(path)?;
        let config: Self = serde_yml::from_str(&content)?;
        Ok(config)
    }

    /// 从 YAML 字符串解析
    pub fn from_yaml(yaml: &str) -> Result<Self> {
        let config: Self = serde_yml::from_str(yaml)?;
        Ok(config)
    }

    /// 转换为模型注册表
    pub fn into_registry(self) -> ModelRegistry {
        ModelRegistry::from_config(self)
    }
}

/// 单个模型定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelDefinition {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub family: String,
    #[serde(default)]
    pub tier: String,
    #[serde(default)]
    pub download_url: Option<String>,
    #[serde(default)]
    pub filename: Option<String>,
    #[serde(default = "default_true")]
    pub supports_thinking: bool,
    #[serde(default = "default_true")]
    pub supports_tools: bool,
    pub template: TemplateConfig,
    pub sampling: SamplingConfig,
}

fn default_true() -> bool {
    true
}

/// 对话模板配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateConfig {
    pub system_prefix: String,
    pub system_suffix: String,
    pub user_prefix: String,
    pub user_suffix: String,
    pub assistant_prefix: String,
    pub assistant_suffix: String,
    #[serde(default)]
    pub think_tag: Option<String>,
}

impl TemplateConfig {
    /// 格式化 system 消息
    pub fn format_system(&self, content: &str) -> String {
        format!("{}{}{}", self.system_prefix, content, self.system_suffix)
    }

    /// 格式化 user 消息
    pub fn format_user(&self, content: &str) -> String {
        format!("{}{}{}", self.user_prefix, content, self.user_suffix)
    }

    /// 格式化 assistant 开始标记
    pub fn format_assistant_start(&self, thinking: bool) -> String {
        if thinking && let Some(think_tag) = &self.think_tag {
            return format!("{}{}", self.assistant_prefix, think_tag);
        }
        self.assistant_prefix.clone()
    }

    /// 格式化 assistant 结束标记
    pub fn format_assistant_end(&self) -> &str {
        &self.assistant_suffix
    }

    /// 格式化完整 prompt
    pub fn format_prompt(&self, system: Option<&str>, user: &str, thinking: bool) -> String {
        let mut prompt = String::new();
        if let Some(sys) = system {
            prompt.push_str(&self.format_system(sys));
        }
        prompt.push_str(&self.format_user(user));
        prompt.push_str(&self.format_assistant_start(thinking));
        prompt
    }

    /// 格式化带工具的 prompt
    pub fn format_prompt_with_tools(
        &self,
        system: Option<&str>,
        user: &str,
        tools: &crate::tool::ToolRegistry,
        thinking: bool,
    ) -> String {
        let mut prompt = String::new();
        let sys_content = match system {
            Some(sys) => format!("{}\n\n{}", sys, tools.to_tool_prompt()),
            None => tools.to_tool_prompt(),
        };
        prompt.push_str(&self.format_system(&sys_content));
        prompt.push_str(&self.format_user(user));
        prompt.push_str(&self.format_assistant_start(thinking));
        prompt
    }
}

/// 采样参数配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SamplingConfig {
    pub thinking: SamplingParams,
    pub non_thinking: SamplingParams,
}

/// 采样参数
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct SamplingParams {
    #[serde(default = "default_temperature")]
    pub temperature: f32,
    #[serde(default)]
    pub min_p: f32,
    #[serde(default = "default_top_p")]
    pub top_p: f32,
    #[serde(default = "default_top_k")]
    pub top_k: i32,
}

fn default_temperature() -> f32 {
    0.6
}
fn default_top_p() -> f32 {
    0.95
}
fn default_top_k() -> i32 {
    20
}

impl Default for SamplingParams {
    fn default() -> Self {
        Self {
            temperature: 0.6,
            min_p: 0.0,
            top_p: 0.95,
            top_k: 20,
        }
    }
}

// ============================================================================
// 模型注册表
// ============================================================================

/// 模型注册表，管理所有模型配置
#[derive(Debug, Clone, Default)]
pub struct ModelRegistry {
    models: HashMap<String, Arc<ModelDefinition>>,
}

impl ModelRegistry {
    /// 创建空注册表
    pub fn new() -> Self {
        Self::default()
    }

    /// 从配置创建注册表
    pub fn from_config(config: ModelsConfig) -> Self {
        let mut models = HashMap::new();
        for model in config.models {
            models.insert(model.id.clone(), Arc::new(model));
        }
        Self { models }
    }

    /// 从 YAML 文件加载
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let config = ModelsConfig::load(path)?;
        Ok(Self::from_config(config))
    }

    /// 获取模型定义
    pub fn get(&self, id: &str) -> Option<Arc<ModelDefinition>> {
        self.models.get(id).cloned()
    }

    /// 获取所有模型 ID
    pub fn model_ids(&self) -> Vec<&str> {
        self.models.keys().map(|s| s.as_str()).collect()
    }

    /// 获取所有模型定义
    pub fn models(&self) -> Vec<Arc<ModelDefinition>> {
        self.models.values().cloned().collect()
    }

    /// 注册新模型
    pub fn register(&mut self, model: ModelDefinition) {
        self.models.insert(model.id.clone(), Arc::new(model));
    }
}

// ============================================================================
// 聊天模式
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ChatMode {
    #[default]
    Thinking,
    NonThinking,
}

impl ChatMode {
    pub fn is_thinking(&self) -> bool {
        matches!(self, ChatMode::Thinking)
    }
}

// ============================================================================
// Agent 模型（通用 GGUF 推理封装）
// ============================================================================

/// 通用 Agent 模型（基于 llama-cpp-2）
pub struct AgentModel {
    backend: LlamaBackend,
    model: LlamaModel,
    ctx_params: LlamaContextParams,
    definition: Arc<ModelDefinition>,
    n_ctx: u32,
}

impl AgentModel {
    /// 默认 GPU 层数（999 = 全部放 GPU，Mac Metal 加速）
    const DEFAULT_GPU_LAYERS: u32 = 999;

    /// 从 GGUF 文件加载模型（使用配置定义）
    pub fn load(model_path: impl AsRef<Path>, definition: Arc<ModelDefinition>) -> Result<Self> {
        // 禁用 llama.cpp 底层日志输出
        send_logs_to_tracing(LogOptions::default().with_logs_enabled(false));

        let backend = LlamaBackend::init()?;
        let model_params = LlamaModelParams::default().with_n_gpu_layers(Self::DEFAULT_GPU_LAYERS);
        let model = LlamaModel::load_from_file(&backend, model_path.as_ref(), &model_params)?;

        // 从模型元数据读取最大上下文长度
        let n_ctx = model.n_ctx_train();

        let ctx_params = LlamaContextParams::default()
            .with_n_ctx(NonZeroU32::new(n_ctx))
            .with_n_batch(512);

        Ok(Self {
            backend,
            model,
            ctx_params,
            definition,
            n_ctx,
        })
    }

    /// 获取模型 ID
    pub fn id(&self) -> &str {
        &self.definition.id
    }

    /// 获取模型名称
    pub fn name(&self) -> &str {
        &self.definition.name
    }

    /// 获取模型定义
    pub fn definition(&self) -> &ModelDefinition {
        &self.definition
    }

    /// 获取上下文长度
    pub fn n_ctx(&self) -> u32 {
        self.n_ctx
    }

    /// 根据模式获取采样参数
    fn sampling_params(&self, mode: ChatMode) -> SamplingParams {
        match mode {
            ChatMode::Thinking => self.definition.sampling.thinking,
            ChatMode::NonThinking => self.definition.sampling.non_thinking,
        }
    }

    /// 流式对话（每生成一个 token 调用回调）
    pub fn chat<F>(&self, system: Option<&str>, user: &str, on_token: F) -> Result<String>
    where
        F: FnMut(&str),
    {
        self.chat_with_mode(system, user, ChatMode::Thinking, on_token)
    }

    /// 流式对话（可指定模式：Thinking / NonThinking）
    pub fn chat_with_mode<F>(
        &self,
        system: Option<&str>,
        user: &str,
        mode: ChatMode,
        on_token: F,
    ) -> Result<String>
    where
        F: FnMut(&str),
    {
        let prompt = self
            .definition
            .template
            .format_prompt(system, user, mode.is_thinking());
        let params = self.sampling_params(mode);
        self.generate(&prompt, params, on_token)
    }

    /// 流式生成（内部方法）
    fn generate<F>(&self, prompt: &str, params: SamplingParams, mut on_token: F) -> Result<String>
    where
        F: FnMut(&str),
    {
        let mut ctx: LlamaContext = self
            .model
            .new_context(&self.backend, self.ctx_params.clone())?;

        let tokens = self
            .model
            .str_to_token(prompt, AddBos::Always)
            .map_err(|e| anyhow::anyhow!(e))?;
        let n_prompt = tokens.len();

        // 分批处理 prompt tokens（每批最多 512 个）
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
        let max_ctx = self.ctx_params.n_ctx().map(NonZeroU32::get).unwrap_or(0) as usize;

        // 在循环外创建采样器
        let mut sampler = LlamaSampler::chain_simple([
            LlamaSampler::top_k(params.top_k),
            LlamaSampler::top_p(params.top_p, 1),
            LlamaSampler::min_p(params.min_p, 1),
            LlamaSampler::temp(params.temperature),
            LlamaSampler::dist(n_cur as u32),
        ]);

        loop {
            if max_ctx > 0 && n_cur >= max_ctx.saturating_sub(1) {
                break;
            }

            let next_token = sampler.sample(&ctx, batch.n_tokens() - 1);
            sampler.accept(next_token);

            // 只检查真正的 EOS
            if next_token == self.model.token_eos() {
                break;
            }

            if let Ok(piece) = self.model.token_to_str(next_token, Special::Tokenize) {
                on_token(&piece);
                output.push_str(&piece);
            }

            batch.clear();
            batch.add(next_token, n_cur as i32, &[0], true)?;
            n_cur += 1;

            ctx.decode(&mut batch)?;
        }

        Ok(output)
    }

    /// 带工具调用的流式对话
    pub fn chat_with_tools<F>(
        &self,
        system: Option<&str>,
        user: &str,
        tools: &crate::tool::ToolRegistry,
        on_token: F,
    ) -> Result<String>
    where
        F: FnMut(&str),
    {
        self.chat_with_tools_mode(system, user, tools, ChatMode::Thinking, on_token)
    }

    /// 带工具调用的流式对话（可指定模式）
    pub fn chat_with_tools_mode<F>(
        &self,
        system: Option<&str>,
        user: &str,
        tools: &crate::tool::ToolRegistry,
        mode: ChatMode,
        on_token: F,
    ) -> Result<String>
    where
        F: FnMut(&str),
    {
        let prompt = self.definition.template.format_prompt_with_tools(
            system,
            user,
            tools,
            mode.is_thinking(),
        );
        let params = self.sampling_params(mode);
        self.generate(&prompt, params, on_token)
    }
}

/// 兼容别名（保持向后兼容）
pub type Qwen3Llama = AgentModel;

// ============================================================================
// 聊天会话（复用 KV Cache）
// ============================================================================

/// 支持连续对话的会话（复用 KV Cache）
pub struct ChatSession<'a> {
    agent: &'a AgentModel,
    ctx: LlamaContext<'a>,
    system: Option<String>,
    n_past: usize,
}

impl<'a> ChatSession<'a> {
    /// 创建会话
    pub fn new(agent: &'a AgentModel, system: Option<&str>) -> Result<Self> {
        let ctx = agent
            .model
            .new_context(&agent.backend, agent.ctx_params.clone())?;
        Ok(Self {
            agent,
            ctx,
            system: system.map(String::from),
            n_past: 0,
        })
    }

    /// 发送消息（流式回调，默认 Thinking）
    pub fn chat<F>(&mut self, user_msg: &str, on_token: F) -> Result<String>
    where
        F: FnMut(&str),
    {
        self.chat_with_mode(user_msg, ChatMode::Thinking, on_token)
    }

    /// 发送消息（可指定模式）
    pub fn chat_with_mode<F>(
        &mut self,
        user_msg: &str,
        mode: ChatMode,
        mut on_token: F,
    ) -> Result<String>
    where
        F: FnMut(&str),
    {
        let prompt = self.build_prompt(user_msg, mode);
        let add_bos = if self.n_past == 0 {
            AddBos::Always
        } else {
            AddBos::Never
        };
        let tokens = self
            .agent
            .model
            .str_to_token(&prompt, add_bos)
            .map_err(|e| anyhow::anyhow!(e))?;
        let n_new = tokens.len();

        // 编码新 tokens
        let mut batch = LlamaBatch::new(512, 1);
        for (i, token) in tokens.iter().enumerate() {
            let pos = (self.n_past + i) as i32;
            batch.add(*token, pos, &[0], i == n_new - 1)?;
        }
        self.ctx.decode(&mut batch)?;
        self.n_past += n_new;

        // 记录 assistant 回复开始位置，用于后续清理 thinking 内容
        let assistant_start_pos = self.n_past;

        // 生成回复
        let mut output = String::new();
        let max_ctx = self
            .agent
            .ctx_params
            .n_ctx()
            .map(NonZeroU32::get)
            .unwrap_or(0) as usize;

        // 在循环外创建采样器（根据模型配置和模式）
        let params = self.agent.sampling_params(mode);
        let mut sampler = LlamaSampler::chain_simple([
            LlamaSampler::top_k(params.top_k),
            LlamaSampler::top_p(params.top_p, 1),
            LlamaSampler::min_p(params.min_p, 1),
            LlamaSampler::temp(params.temperature),
            LlamaSampler::dist(self.n_past as u32),
        ]);

        // 追踪 </think> 结束位置
        let mut think_end_pos: Option<usize> = None;

        loop {
            if max_ctx > 0 && self.n_past >= max_ctx.saturating_sub(1) {
                break;
            }

            let next_token = sampler.sample(&self.ctx, batch.n_tokens() - 1);
            sampler.accept(next_token);

            // 只检查真正的 EOS
            if next_token == self.agent.model.token_eos() {
                break;
            }

            if let Ok(piece) = self.agent.model.token_to_str(next_token, Special::Tokenize) {
                on_token(&piece);
                output.push_str(&piece);
                if cfg!(debug_assertions) {
                    print!("{piece}");
                    let _ = io::stdout().flush();
                }

                // Thinking 模式下检测 </think> 结束位置
                if mode.is_thinking() && think_end_pos.is_none() && output.contains("</think>") {
                    think_end_pos = Some(self.n_past + 1); // +1 因为当前 token 还未计入
                }
            }

            batch.clear();
            batch.add(next_token, self.n_past as i32, &[0], true)?;
            self.n_past += 1;

            self.ctx.decode(&mut batch)?;
        }

        // 官方最佳实践：多轮对话中，历史记录不应包含 thinking 内容
        // 从 KV Cache 中删除 thinking 部分，只保留最终输出
        if mode.is_thinking()
            && let Some(think_end) = think_end_pos
        {
            let thinking_len = think_end - assistant_start_pos;
            if thinking_len > 0 {
                // 删除 thinking 内容的 KV Cache
                let _ = self.ctx.clear_kv_cache_seq(
                    Some(0), // sequence id
                    Some(assistant_start_pos as u32),
                    Some(think_end as u32),
                );
                // 将后续内容位置前移
                let _ = self.ctx.kv_cache_seq_add(
                    0,
                    Some(think_end as u32),
                    Some(self.n_past as u32),
                    -(thinking_len as i32),
                );
                self.n_past -= thinking_len;
            }
        }

        // 生成结束后，编码 assistant 结束标记到 KV Cache，确保下一轮对话正确
        let end_tokens = self
            .agent
            .model
            .str_to_token(
                self.agent.definition.template.format_assistant_end(),
                AddBos::Never,
            )
            .map_err(|e| anyhow::anyhow!(e))?;
        batch.clear();
        for (i, token) in end_tokens.iter().enumerate() {
            let pos = (self.n_past + i) as i32;
            batch.add(*token, pos, &[0], i == end_tokens.len() - 1)?;
        }
        self.ctx.decode(&mut batch)?;
        self.n_past += end_tokens.len();

        Ok(output)
    }

    /// 构建 prompt（使用模型配置的模板）
    fn build_prompt(&self, user_msg: &str, mode: ChatMode) -> String {
        let template = &self.agent.definition.template;
        let mut prompt = String::new();

        // 第一轮对话才添加 system 消息
        if self.n_past == 0
            && let Some(sys) = &self.system
        {
            prompt.push_str(&template.format_system(sys));
        }

        prompt.push_str(&template.format_user(user_msg));
        prompt.push_str(&template.format_assistant_start(mode.is_thinking()));
        prompt
    }
}
