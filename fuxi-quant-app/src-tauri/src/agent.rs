use fuxi_quant_agent::agent::{Agent, StreamEvent};
use fuxi_quant_agent::model::Model;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};
use tauri::ipc::Channel;

/// 全局模型实例
static MODEL: OnceLock<Arc<Model>> = OnceLock::new();

/// 全局会话 Map
static SESSIONS: OnceLock<Mutex<HashMap<String, Agent>>> = OnceLock::new();

fn sessions() -> &'static Mutex<HashMap<String, Agent>> {
    SESSIONS.get_or_init(|| Mutex::new(HashMap::new()))
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
    if MODEL.get().is_some() {
        return Ok("智能体已初始化".into());
    }

    tauri::async_runtime::spawn_blocking(move || {
        let model = Model::load(&model_path).map_err(|e| e.to_string())?;
        MODEL.set(model).map_err(|_| "智能体已被初始化")?;
        Ok("智能体初始化成功".into())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// 创建新会话
#[tauri::command]
pub async fn create_session(
    session_id: String,
    sys_prompt: Option<String>,
    ctx_len: Option<u32>,
) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        let model = model()?;
        let agent =
            Agent::new(model, sys_prompt, ctx_len.unwrap_or(8192)).map_err(|e| e.to_string())?;

        let mut map = sessions().lock().map_err(|e| e.to_string())?;
        map.insert(session_id, agent);
        Ok(())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// 发送消息（流式响应）
#[tauri::command]
pub async fn chat(
    session_id: String,
    message: String,
    channel: Channel<StreamEvent>,
) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        let mut map = sessions().lock().map_err(|e| e.to_string())?;
        let agent = map.get_mut(&session_id).ok_or("会话不存在")?;

        let result = agent.chat(&message, |event| {
            let _ = channel.send(event);
        });

        match result {
            Ok(()) => Ok(()),
            Err(e) => {
                let _ = channel.send(StreamEvent::Error(e.to_string()));
                Err(e.to_string())
            }
        }
    })
    .await
    .map_err(|e| e.to_string())?
}

/// 删除会话
#[tauri::command]
pub async fn remove_session(session_id: String) -> Result<(), String> {
    let mut map = sessions().lock().map_err(|e| e.to_string())?;
    map.remove(&session_id);
    Ok(())
}
