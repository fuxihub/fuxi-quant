use fuxi_quant_agent::model::{ChatSession, Qwen3Llama};
use std::sync::{Mutex, OnceLock};
use tauri::ipc::Channel;

/// 全局模型实例
static MODEL: OnceLock<Qwen3Llama> = OnceLock::new();

/// 全局会话实例
static SESSION: OnceLock<Mutex<Option<ChatSessionWrapper>>> = OnceLock::new();

/// ChatSession 包装器（处理生命周期）
struct ChatSessionWrapper {
    inner: ChatSession<'static>,
}

// Safety: ChatSession 内部已经是线程安全的
unsafe impl Send for ChatSessionWrapper {}
unsafe impl Sync for ChatSessionWrapper {}

/// 流式响应事件
#[derive(Clone, serde::Serialize)]
#[serde(tag = "type", content = "data")]
pub enum StreamEvent {
    Token(String),
    Done(String),
    Error(String),
}

/// 加载模型（异步，避免阻塞主线程）
#[tauri::command]
pub async fn load_model(model_path: String) -> Result<String, String> {
    if MODEL.get().is_some() {
        return Ok("模型已加载".into());
    }

    // 在后台线程加载模型
    tauri::async_runtime::spawn_blocking(move || {
        let llama = Qwen3Llama::load(&model_path).map_err(|e| e.to_string())?;
        MODEL.set(llama).map_err(|_| "模型已被加载")?;

        // 初始化会话
        let model = MODEL.get().unwrap();
        let session = ChatSession::new(model, Some("你是阿强，一个专业的量化交易助手。"))
            .map_err(|e| e.to_string())?;

        // Safety: MODEL 是 'static，所以 session 也可以安全地保存
        let wrapper = ChatSessionWrapper {
            inner: unsafe { std::mem::transmute::<ChatSession<'_>, ChatSession<'static>>(session) },
        };

        SESSION.get_or_init(|| Mutex::new(Some(wrapper)));
        Ok("模型加载成功".into())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// 检查模型是否已加载
#[tauri::command]
pub fn is_model_loaded() -> bool {
    MODEL.get().is_some()
}

/// 发送消息（流式响应）
#[tauri::command]
pub async fn chat(message: String, channel: Channel<StreamEvent>) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        let session_mutex = SESSION.get().ok_or("模型未加载")?;
        let mut guard = session_mutex.lock().map_err(|e| e.to_string())?;
        let session = guard.as_mut().ok_or("会话未初始化")?;

        let result = session.inner.chat(&message, |token| {
            let _ = channel.send(StreamEvent::Token(token.to_string()));
        });

        match result {
            Ok(response) => {
                let _ = channel.send(StreamEvent::Done(response));
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

/// 重置会话
#[tauri::command]
pub fn reset_chat() -> Result<(), String> {
    let model = MODEL.get().ok_or("模型未加载")?;

    let session = ChatSession::new(model, Some("你是阿强，一个专业的量化交易助手。"))
        .map_err(|e| e.to_string())?;

    let wrapper = ChatSessionWrapper {
        inner: unsafe { std::mem::transmute::<ChatSession<'_>, ChatSession<'static>>(session) },
    };

    let session_mutex = SESSION.get().ok_or("会话未初始化")?;
    let mut guard = session_mutex.lock().map_err(|e| e.to_string())?;
    *guard = Some(wrapper);

    Ok(())
}
