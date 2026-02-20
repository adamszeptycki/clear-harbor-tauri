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
