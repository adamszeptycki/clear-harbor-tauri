use crossbeam_channel::Sender;
use libpulse_binding as pulse;
use libpulse_simple_binding as psimple;
use log::{error, info};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;

use crate::audio::mic_capture::AudioChunk;
use crate::audio::system_capture::{CaptureHandle, SystemAudioCapture};
use crate::transcription::types::AudioDeviceInfo;

pub struct LinuxSystemCapture;

impl LinuxSystemCapture {
    pub fn new() -> Self {
        Self
    }
}

struct LinuxCaptureHandle {
    stop_flag: Arc<AtomicBool>,
}

impl CaptureHandle for LinuxCaptureHandle {
    fn stop(&self) {
        self.stop_flag.store(true, Ordering::Relaxed);
    }
}

impl SystemAudioCapture for LinuxSystemCapture {
    fn list_devices(&self) -> Vec<AudioDeviceInfo> {
        vec![AudioDeviceInfo {
            name: "Default System Audio".to_string(),
            id: "default_monitor".to_string(),
            is_default: true,
        }]
    }

    fn start_capture(
        &self,
        _device_id: Option<&str>,
        sender: Sender<AudioChunk>,
    ) -> Result<Box<dyn CaptureHandle>, String> {
        let stop_flag = Arc::new(AtomicBool::new(false));
        let stop_clone = stop_flag.clone();

        let spec = pulse::sample::Spec {
            format: pulse::sample::Format::F32le,
            channels: 1,
            rate: 44100,
        };

        let attr = pulse::def::BufferAttr {
            maxlength: u32::MAX,
            tlength: u32::MAX,
            prebuf: u32::MAX,
            minreq: u32::MAX,
            fragsize: 4410 * 4, // 100ms at 44100Hz, 4 bytes per f32
        };

        thread::spawn(move || {
            let simple = match psimple::Simple::new(
                None,
                "DualScribe",
                pulse::stream::Direction::Record,
                Some("@DEFAULT_MONITOR@"),
                "system-audio-capture",
                &spec,
                None,
                Some(&attr),
            ) {
                Ok(s) => s,
                Err(e) => {
                    error!("Failed to open PulseAudio monitor: {}", e);
                    return;
                }
            };

            info!("Linux system audio capture started");

            let chunk_samples = 4410; // 100ms at 44100Hz mono
            let mut buf = vec![0u8; chunk_samples * 4]; // f32 = 4 bytes

            while !stop_clone.load(Ordering::Relaxed) {
                match simple.read(&mut buf) {
                    Ok(()) => {
                        let samples: Vec<f32> = buf
                            .chunks_exact(4)
                            .map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
                            .collect();

                        let _ = sender.try_send(AudioChunk {
                            samples,
                            sample_rate: 44100,
                            channels: 1,
                        });
                    }
                    Err(e) => {
                        error!("PulseAudio read error: {}", e);
                        break;
                    }
                }
            }

            info!("Linux system audio capture stopped");
        });

        Ok(Box::new(LinuxCaptureHandle { stop_flag }))
    }
}
