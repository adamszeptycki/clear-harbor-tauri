use crossbeam_channel::Sender;
use log::info;
use screencapturekit::prelude::*;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use crate::audio::mic_capture::AudioChunk;
use crate::audio::system_capture::{CaptureHandle, SystemAudioCapture};
use crate::transcription::types::AudioDeviceInfo;

const SAMPLE_RATE: u32 = 48000;
const CHANNELS: u16 = 1;

pub struct MacosSystemCapture;

struct MacosCaptureHandle {
    stop_flag: Arc<AtomicBool>,
}

impl CaptureHandle for MacosCaptureHandle {
    fn stop(&self) {
        self.stop_flag.store(true, Ordering::Relaxed);
    }
}

impl MacosSystemCapture {
    pub fn new() -> Self {
        Self
    }
}

struct AudioHandler {
    sender: Sender<AudioChunk>,
}

impl SCStreamOutputTrait for AudioHandler {
    fn did_output_sample_buffer(&self, sample: CMSampleBuffer, of_type: SCStreamOutputType) {
        if of_type != SCStreamOutputType::Audio {
            return;
        }

        let audio_buffers = match sample.get_audio_buffer_list() {
            Some(bufs) => bufs,
            None => return,
        };

        // Extract f32 PCM samples from the audio buffer list.
        // SCK delivers Float32 PCM in native endian when configured for it.
        let mut all_samples = Vec::new();
        for buffer in audio_buffers.iter() {
            let data = buffer.data();
            // Each sample is 4 bytes (f32)
            for chunk in data.chunks_exact(4) {
                all_samples.push(f32::from_ne_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]));
            }
        }

        if !all_samples.is_empty() {
            let _ = self.sender.try_send(AudioChunk {
                samples: all_samples,
                sample_rate: SAMPLE_RATE,
                channels: CHANNELS,
            });
        }
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
        sender: Sender<AudioChunk>,
    ) -> Result<Box<dyn CaptureHandle>, String> {
        let stop_flag = Arc::new(AtomicBool::new(false));
        let stop_clone = stop_flag.clone();

        // Report startup success/failure back to the caller
        let (result_tx, result_rx) = crossbeam_channel::bounded::<Result<(), String>>(1);

        std::thread::spawn(move || {
            // Get available displays (requires Screen Recording permission)
            let content = match SCShareableContent::get() {
                Ok(c) => c,
                Err(e) => {
                    let _ = result_tx.send(Err(format!(
                        "Failed to get shareable content (Screen Recording permission required): {}",
                        e
                    )));
                    return;
                }
            };

            let displays = content.displays();
            if displays.is_empty() {
                let _ = result_tx.send(Err("No displays found".to_string()));
                return;
            }

            // Capture entire display audio (no per-app filtering)
            let filter = SCContentFilter::builder()
                .display(&displays[0])
                .exclude_windows(&[])
                .build();

            // Minimal 2x2 video resolution — SCK requires a display filter even for
            // audio-only capture, so we minimize the video overhead.
            let mut config = SCStreamConfiguration::default();
            config.set_width(2);
            config.set_height(2);
            config.set_captures_audio(true);
            config.set_sample_rate(SAMPLE_RATE as i32);
            config.set_channel_count(CHANNELS as i32);

            let mut stream = SCStream::new(&filter, &config);

            let handler = AudioHandler { sender };
            stream.add_output_handler(handler, SCStreamOutputType::Audio);

            if let Err(e) = stream.start_capture() {
                let _ = result_tx.send(Err(format!("Failed to start SCStream: {}", e)));
                return;
            }

            info!(
                "macOS system audio capture started ({}Hz, {}ch)",
                SAMPLE_RATE, CHANNELS
            );
            let _ = result_tx.send(Ok(()));

            // Keep stream alive until stop flag is set
            while !stop_clone.load(Ordering::Relaxed) {
                std::thread::sleep(std::time::Duration::from_millis(100));
            }

            let _ = stream.stop_capture();
            info!("macOS system audio capture stopped");
        });

        result_rx
            .recv()
            .map_err(|e| format!("SCK channel error: {}", e))?
            .map_err(|e| e)?;

        Ok(Box::new(MacosCaptureHandle { stop_flag }))
    }
}
