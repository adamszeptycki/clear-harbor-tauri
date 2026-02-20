pub mod mic_capture;
pub mod resampler;
pub mod system_capture;

#[cfg(target_os = "linux")]
pub mod system_linux;
#[cfg(target_os = "windows")]
pub mod system_windows;
#[cfg(target_os = "macos")]
pub mod system_macos;
