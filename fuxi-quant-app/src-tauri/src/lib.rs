use fuxi_quant_agent::model::{ChatSession, Qwen3Llama};
use std::sync::{Mutex, OnceLock};
use tauri::ipc::Channel;

/// 全局模型实例（加载一次，复用）
static MODEL: OnceLock<Qwen3Llama> = OnceLock::new();

/// 全局会话实例（使用 Mutex 保证线程安全）
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
    /// 收到 token
    Token(String),
    /// 完成
    Done(String),
    /// 错误
    Error(String),
}

/// 加载模型
#[tauri::command]
fn load_model(model_path: &str) -> Result<String, String> {
    if MODEL.get().is_some() {
        return Ok("模型已加载".into());
    }

    let llama = Qwen3Llama::load(model_path).map_err(|e| e.to_string())?;

    MODEL.set(llama).map_err(|_| "模型已被加载")?;

    // 初始化会话
    let model = MODEL.get().unwrap();
    let session = ChatSession::with_system(model, "你是阿强，一个专业的量化交易助手。")
        .map_err(|e| e.to_string())?;

    // Safety: MODEL 是 'static，所以 session 也可以安全地保存
    let wrapper = ChatSessionWrapper {
        inner: unsafe { std::mem::transmute::<ChatSession<'_>, ChatSession<'static>>(session) },
    };

    SESSION.get_or_init(|| Mutex::new(Some(wrapper)));

    Ok("模型加载成功".into())
}

/// 检查模型是否已加载
#[tauri::command]
fn is_model_loaded() -> bool {
    MODEL.get().is_some()
}

/// 发送消息（流式响应，异步执行避免阻塞主线程）
#[tauri::command]
async fn chat(
    message: String,
    max_tokens: usize,
    channel: Channel<StreamEvent>,
) -> Result<(), String> {
    // 在后台线程执行模型推理
    tauri::async_runtime::spawn_blocking(move || {
        let session_mutex = SESSION.get().ok_or("模型未加载")?;

        let mut guard = session_mutex.lock().map_err(|e| e.to_string())?;
        let session = guard.as_mut().ok_or("会话未初始化")?;

        // 流式生成
        let result = session.inner.send_stream(&message, max_tokens, |token| {
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

/// 清空对话历史
#[tauri::command]
fn clear_chat() -> Result<(), String> {
    let session_mutex = SESSION.get().ok_or("模型未加载")?;

    let mut guard = session_mutex.lock().map_err(|e| e.to_string())?;
    let session = guard.as_mut().ok_or("会话未初始化")?;

    session.inner.clear().map_err(|e| e.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            load_model,
            is_model_loaded,
            chat,
            clear_chat
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
