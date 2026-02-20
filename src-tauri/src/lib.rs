mod audio;
mod commands;
mod export;
mod settings;
mod transcription;

use commands::AppState;
use transcription::stream_manager::StreamManager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .manage(AppState {
            stream_manager: std::sync::Mutex::new(StreamManager::new()),
            settings: std::sync::Mutex::new(settings::AppSettings::default()),
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_input_devices,
            commands::list_output_devices,
            commands::start_transcription,
            commands::stop_transcription,
            commands::export_transcript,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
