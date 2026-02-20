use crossbeam_channel::Sender;

use crate::audio::mic_capture::AudioChunk;
use crate::audio::system_capture::{CaptureHandle, SystemAudioCapture};
use crate::transcription::types::AudioDeviceInfo;

pub struct WindowsSystemCapture;
struct WindowsCaptureHandle;

impl CaptureHandle for WindowsCaptureHandle {
    fn stop(&self) {}
}

impl WindowsSystemCapture {
    pub fn new() -> Self {
        Self
    }
}

impl SystemAudioCapture for WindowsSystemCapture {
    fn list_devices(&self) -> Vec<AudioDeviceInfo> {
        vec![AudioDeviceInfo {
            name: "Default System Audio".to_string(),
            id: "default".to_string(),
            is_default: true,
        }]
    }

    fn start_capture(
        &self,
        _device_id: Option<&str>,
        _sender: Sender<AudioChunk>,
    ) -> Result<Box<dyn CaptureHandle>, String> {
        Err("WASAPI Loopback capture not yet implemented".to_string())
    }
}
