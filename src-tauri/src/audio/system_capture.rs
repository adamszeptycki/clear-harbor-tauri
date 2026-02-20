use crossbeam_channel::Sender;

use crate::audio::mic_capture::AudioChunk;
use crate::transcription::types::AudioDeviceInfo;

pub trait SystemAudioCapture: Send {
    fn list_devices(&self) -> Vec<AudioDeviceInfo>;
    fn start_capture(
        &self,
        device_id: Option<&str>,
        sender: Sender<AudioChunk>,
    ) -> Result<Box<dyn CaptureHandle>, String>;
}

pub trait CaptureHandle: Send {
    fn stop(&self);
}

pub fn create_system_capture() -> Box<dyn SystemAudioCapture> {
    #[cfg(target_os = "linux")]
    {
        Box::new(super::system_linux::LinuxSystemCapture::new())
    }
    #[cfg(target_os = "windows")]
    {
        Box::new(super::system_windows::WindowsSystemCapture::new())
    }
    #[cfg(target_os = "macos")]
    {
        Box::new(super::system_macos::MacosSystemCapture::new())
    }
}
