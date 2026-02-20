use crossbeam_channel::Sender;

use crate::audio::mic_capture::AudioChunk;
use crate::audio::system_capture::{CaptureHandle, SystemAudioCapture};
use crate::transcription::types::AudioDeviceInfo;

pub struct MacosSystemCapture;
struct MacosCaptureHandle;

impl CaptureHandle for MacosCaptureHandle {
    fn stop(&self) {}
}

impl MacosSystemCapture {
    pub fn new() -> Self {
        Self
    }
}

impl SystemAudioCapture for MacosSystemCapture {
    fn list_devices(&self) -> Vec<AudioDeviceInfo> {
        vec![AudioDeviceInfo {
            name: "System Audio".to_string(),
            id: "screencapturekit".to_string(),
            is_default: true,
        }]
    }

    fn start_capture(
        &self,
        _device_id: Option<&str>,
        _sender: Sender<AudioChunk>,
    ) -> Result<Box<dyn CaptureHandle>, String> {
        Err("ScreenCaptureKit capture not yet implemented".to_string())
    }
}
