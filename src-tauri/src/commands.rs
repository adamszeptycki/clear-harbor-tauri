use std::sync::Mutex;
use tauri::State;

use crate::audio::mic_capture::MicCapture;
use crate::audio::system_capture;
use crate::settings::AppSettings;
use crate::transcription::stream_manager::StreamManager;
use crate::transcription::types::AudioDeviceInfo;

pub struct AppState {
    pub stream_manager: Mutex<StreamManager>,
    pub settings: Mutex<AppSettings>,
}

#[tauri::command]
pub fn list_input_devices() -> Vec<AudioDeviceInfo> {
    let mic = MicCapture::new();
    mic.list_devices()
}

#[tauri::command]
pub fn list_output_devices() -> Vec<AudioDeviceInfo> {
    let system = system_capture::create_system_capture();
    system.list_devices()
}

#[tauri::command]
pub fn start_transcription(
    app_handle: tauri::AppHandle,
    state: State<'_, AppState>,
    api_key: String,
    language: String,
    mic_device_id: Option<String>,
    system_device_id: Option<String>,
) -> Result<(), String> {
    let mut manager = state.stream_manager.lock().map_err(|e| e.to_string())?;
    manager.start(app_handle, api_key, language, mic_device_id, system_device_id)
}

#[tauri::command]
pub fn stop_transcription(state: State<'_, AppState>) -> Result<(), String> {
    let mut manager = state.stream_manager.lock().map_err(|e| e.to_string())?;
    manager.stop();
    Ok(())
}

#[tauri::command]
pub fn export_transcript(
    segments: Vec<crate::transcription::types::TranscriptSegment>,
    format: String,
    show_timestamps: bool,
) -> Result<String, String> {
    match format.as_str() {
        "markdown" => Ok(crate::export::export_markdown(&segments, show_timestamps)),
        "text" => Ok(crate::export::export_plain_text(&segments, show_timestamps)),
        "json" => crate::export::export_json(&segments),
        _ => Err(format!("Unknown format: {}", format)),
    }
}

#[tauri::command]
pub fn auto_save_transcript(
    app_handle: tauri::AppHandle,
    segments: Vec<crate::transcription::types::TranscriptSegment>,
) -> Result<(), String> {
    use tauri::Manager;
    let path = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?
        .join("autosave.json");
    // Ensure the directory exists
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string(&segments).map_err(|e| e.to_string())?;
    std::fs::write(&path, json).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn check_autosave(
    app_handle: tauri::AppHandle,
) -> Result<Option<Vec<crate::transcription::types::TranscriptSegment>>, String> {
    use tauri::Manager;
    let path = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?
        .join("autosave.json");
    if path.exists() {
        let json = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
        let segments = serde_json::from_str(&json).map_err(|e| e.to_string())?;
        Ok(Some(segments))
    } else {
        Ok(None)
    }
}

#[tauri::command]
pub fn clear_autosave(app_handle: tauri::AppHandle) -> Result<(), String> {
    use tauri::Manager;
    let path = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?
        .join("autosave.json");
    if path.exists() {
        std::fs::remove_file(&path).map_err(|e| e.to_string())?;
    }
    Ok(())
}
