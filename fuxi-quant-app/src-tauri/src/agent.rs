use fuxi_quant_agent::Qwen3Agent;
use fuxi_quant_agent::agent::Agent;
use fuxi_quant_agent::model::Model;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};
use tauri::ipc::Channel;

/// 全局模型实例
static MODEL: OnceLock<Arc<Model>> = OnceLock::new();

/// 全局会话 Map
static SESSIONS: OnceLock<Mutex<HashMap<String, Qwen3Agent>>> = OnceLock::new();

fn sessions() -> &'static Mutex<HashMap<String, Qwen3Agent>> {
    SESSIONS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn model() -> Result<&'static Model, String> {
    MODEL
        .get()
        .map(|m| {
            let ptr = Arc::as_ptr(m);
            unsafe { &*ptr }
        })
        .ok_or_else(|| "模型未加载".to_string())
}

/// 流式响应事件
#[derive(Clone, serde::Serialize)]
#[serde(tag = "type", content = "data")]
pub enum StreamEvent {
    Token(String),
    Done(String),
    Error(String),
}

/// 加载模型
#[tauri::command]
pub async fn load_model(model_path: String) -> Result<String, String> {
    if MODEL.get().is_some() {
        return Ok("模型已加载".into());
    }

    tauri::async_runtime::spawn_blocking(move || {
        let model = Model::load(&model_path).map_err(|e| e.to_string())?;
        MODEL.set(model).map_err(|_| "模型已被加载")?;
        Ok("模型加载成功".into())
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
        let agent = Qwen3Agent::create(model, sys_prompt, ctx_len.unwrap_or(8192))
            .map_err(|e| e.to_string())?;

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

        let result = agent.chat(&message, |token| {
            let _ = channel.send(StreamEvent::Token(token.to_string()));
        });

        match result {
            Ok(()) => {
                let _ = channel.send(StreamEvent::Done(String::new()));
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

/// 删除会话
#[tauri::command]
pub async fn remove_session(session_id: String) -> Result<(), String> {
    let mut map = sessions().lock().map_err(|e| e.to_string())?;
    map.remove(&session_id);
    Ok(())
}
