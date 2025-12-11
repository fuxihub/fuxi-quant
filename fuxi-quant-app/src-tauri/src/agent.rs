use fuxi_quant_agent::agent::{Agent, AgentConfig, StreamEvent};
use fuxi_quant_agent::model::Model;
use fuxi_quant_agent::tool::builtin;
use std::sync::{Arc, Mutex, OnceLock};
use tauri::ipc::Channel;

/// 全局模型实例
static MODEL: OnceLock<Arc<Model>> = OnceLock::new();

/// 全局 Agent（单例）
static AGENT: OnceLock<Mutex<Agent>> = OnceLock::new();

fn agent() -> Result<&'static Mutex<Agent>, String> {
    AGENT.get().ok_or_else(|| "智能体未初始化".to_string())
}

fn model() -> Result<&'static Model, String> {
    MODEL
        .get()
        .map(|m| {
            let ptr = Arc::as_ptr(m);
            unsafe { &*ptr }
        })
        .ok_or_else(|| "智能体未初始化".to_string())
}

/// 初始化智能体
#[tauri::command]
pub async fn init_agent(model_path: String) -> Result<String, String> {
    if MODEL.get().is_some() && AGENT.get().is_some() {
        return Ok("智能体已初始化".into());
    }

    tauri::async_runtime::spawn_blocking(move || {
        if MODEL.get().is_none() {
            let model = Model::load(&model_path).map_err(|e| e.to_string())?;
            MODEL.set(model).map_err(|_| "智能体已被初始化")?;
        }

        if AGENT.get().is_none() {
            let model = model()?;
            let config = AgentConfig::new()
                .with_system_prompt(include_str!("../../config/prompt.md"))
                .with_ctx_len(8192);
            let agent = Agent::new(model, config).map_err(|e| e.to_string())?;
            AGENT
                .set(Mutex::new(agent))
                .map_err(|_| "智能体已被初始化")?;
        }

        Ok("智能体初始化成功".into())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// 发送消息（流式响应）
#[tauri::command]
pub async fn chat(message: String, channel: Channel<StreamEvent>) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        let agent_mutex = agent()?;
        let mut agent = agent_mutex.lock().map_err(|e| e.to_string())?;

        let result = agent.chat_with_tools(
            &message,
            |event| {
                let _ = channel.send(event);
            },
            builtin::execute_builtin,
        );

        match result {
            Ok(_) => {
                let _ = channel.send(StreamEvent::Done);
                Ok(())
            }
            Err(e) => {
                let _ = channel.send(StreamEvent::Error(e.to_string()));
                Err(e.to_string())
            }
        }
    })
    .await
    .map_err(|e| e.to_string())?
}

/// 清空上下文
#[tauri::command]
pub async fn clear_chat() -> Result<(), String> {
    let agent_mutex = agent()?;
    let mut agent = agent_mutex.lock().map_err(|e| e.to_string())?;
    agent.reset().map_err(|e| format!("重置智能体失败: {e}"))
}
