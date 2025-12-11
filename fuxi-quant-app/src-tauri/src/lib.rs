mod agent;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            agent::init_agent,
            agent::create_session,
            agent::chat,
            agent::remove_session
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
