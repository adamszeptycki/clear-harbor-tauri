use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::SampleFormat;
use crossbeam_channel::Sender;
use log::{error, info};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use crate::audio::mic_capture::AudioChunk;
use crate::audio::system_capture::{CaptureHandle, SystemAudioCapture};
use crate::transcription::types::AudioDeviceInfo;

pub struct WindowsSystemCapture;

struct WindowsCaptureHandle {
    stop_flag: Arc<AtomicBool>,
}

impl CaptureHandle for WindowsCaptureHandle {
    fn stop(&self) {
        self.stop_flag.store(true, Ordering::Relaxed);
    }
}

impl WindowsSystemCapture {
    pub fn new() -> Self {
        Self
    }
}

impl SystemAudioCapture for WindowsSystemCapture {
    fn list_devices(&self) -> Vec<AudioDeviceInfo> {
        let host = cpal::default_host();
        let default_name = host
            .default_output_device()
            .and_then(|d| d.name().ok())
            .unwrap_or_default();

        match host.output_devices() {
            Ok(devices) => devices
                .filter_map(|d| {
                    let name = d.name().ok()?;
                    Some(AudioDeviceInfo {
                        id: name.clone(),
                        is_default: name == default_name,
                        name,
                    })
                })
                .collect(),
            Err(e) => {
                error!("Failed to enumerate output devices: {}", e);
                vec![]
            }
        }
    }

    fn start_capture(
        &self,
        device_id: Option<&str>,
        sender: Sender<AudioChunk>,
    ) -> Result<Box<dyn CaptureHandle>, String> {
        let host = cpal::default_host();

        let device = match device_id {
            Some(id) => {
                let devices = host
                    .output_devices()
                    .map_err(|e| format!("Failed to get output devices: {}", e))?;
                let mut found = None;
                for d in devices {
                    if d.name().ok().as_deref() == Some(id) {
                        found = Some(d);
                        break;
                    }
                }
                found.ok_or_else(|| format!("Output device '{}' not found", id))?
            }
            None => host
                .default_output_device()
                .ok_or_else(|| "No default output device found".to_string())?,
        };

        let config = device
            .default_output_config()
            .map_err(|e| format!("Failed to get output config: {}", e))?;

        let sample_rate = config.sample_rate().0;
        let channels = config.channels();
        let sample_format = config.sample_format();

        info!(
            "Windows system audio (WASAPI loopback): {} Hz, {} ch, {:?}",
            sample_rate, channels, sample_format
        );

        let stop_flag = Arc::new(AtomicBool::new(false));
        let stop_clone = stop_flag.clone();

        // Report startup success/failure back to the caller
        let (result_tx, result_rx) = crossbeam_channel::bounded::<Result<(), String>>(1);

        std::thread::spawn(move || {
            let stream_config: cpal::StreamConfig = config.into();
            let sr = sample_rate;
            let ch = channels;

            // build_input_stream on an output device enables WASAPI loopback
            let stream = match sample_format {
                SampleFormat::F32 => {
                    let sender = sender.clone();
                    device.build_input_stream(
                        &stream_config,
                        move |data: &[f32], _: &cpal::InputCallbackInfo| {
                            let _ = sender.try_send(AudioChunk {
                                samples: data.to_vec(),
                                sample_rate: sr,
                                channels: ch,
                            });
                        },
                        |err| error!("System audio stream error: {}", err),
                        None,
                    )
                }
                SampleFormat::I16 => {
                    let sender = sender.clone();
                    device.build_input_stream(
                        &stream_config,
                        move |data: &[i16], _: &cpal::InputCallbackInfo| {
                            let samples: Vec<f32> = data
                                .iter()
                                .map(|&s| s as f32 / i16::MAX as f32)
                                .collect();
                            let _ = sender.try_send(AudioChunk {
                                samples,
                                sample_rate: sr,
                                channels: ch,
                            });
                        },
                        |err| error!("System audio stream error: {}", err),
                        None,
                    )
                }
                _ => {
                    let _ = result_tx.send(Err(format!(
                        "Unsupported sample format: {:?}",
                        sample_format
                    )));
                    return;
                }
            };

            let stream = match stream {
                Ok(s) => s,
                Err(e) => {
                    let _ = result_tx.send(Err(format!("Failed to build loopback stream: {}", e)));
                    return;
                }
            };

            if let Err(e) = stream.play() {
                let _ = result_tx.send(Err(format!("Failed to start loopback stream: {}", e)));
                return;
            }

            let _ = result_tx.send(Ok(()));

            // Keep stream alive until stop flag is set
            while !stop_clone.load(Ordering::Relaxed) {
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
            // stream is dropped here, stopping capture
            info!("Windows system audio capture stopped");
        });

        result_rx
            .recv()
            .map_err(|e| format!("Loopback channel error: {}", e))?
            .map_err(|e| e)?;

        Ok(Box::new(WindowsCaptureHandle { stop_flag }))
    }
}
