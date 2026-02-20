use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, Host, SampleFormat, Stream, StreamConfig};
use crossbeam_channel::Sender;
use log::{error, info};

use crate::transcription::types::AudioDeviceInfo;

/// Raw audio chunk from capture thread
pub struct AudioChunk {
    pub samples: Vec<f32>, // interleaved f32 samples
    pub sample_rate: u32,
    pub channels: u16,
}

pub struct MicCapture {
    host: Host,
}

impl MicCapture {
    pub fn new() -> Self {
        let host = cpal::default_host();
        Self { host }
    }

    /// List available input (microphone) devices
    pub fn list_devices(&self) -> Vec<AudioDeviceInfo> {
        let default_name = self
            .host
            .default_input_device()
            .and_then(|d| d.name().ok())
            .unwrap_or_default();

        match self.host.input_devices() {
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
                error!("Failed to enumerate input devices: {}", e);
                vec![]
            }
        }
    }

    /// Get a device by name, or the default input device
    pub fn get_device(&self, device_id: Option<&str>) -> Result<Device, String> {
        match device_id {
            Some(id) => {
                let devices = self
                    .host
                    .input_devices()
                    .map_err(|e| format!("Failed to get devices: {}", e))?;
                for device in devices {
                    if device.name().ok().as_deref() == Some(id) {
                        return Ok(device);
                    }
                }
                Err(format!("Device '{}' not found", id))
            }
            None => self
                .host
                .default_input_device()
                .ok_or_else(|| "No default input device found".to_string()),
        }
    }

    /// Start capturing from the given device. Sends AudioChunks over the channel.
    /// Returns the Stream handle -- capture stops when this is dropped.
    pub fn start_capture(
        &self,
        device: &Device,
        sender: Sender<AudioChunk>,
    ) -> Result<(Stream, StreamConfig), String> {
        let config = device
            .default_input_config()
            .map_err(|e| format!("Failed to get input config: {}", e))?;

        let sample_rate = config.sample_rate().0;
        let channels = config.channels();
        let sample_format = config.sample_format();

        info!(
            "Mic capture: {} Hz, {} ch, {:?}",
            sample_rate, channels, sample_format
        );

        let stream_config: StreamConfig = config.into();
        let sr = sample_rate;
        let ch = channels;

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
                    |err| error!("Mic stream error: {}", err),
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
                    |err| error!("Mic stream error: {}", err),
                    None,
                )
            }
            _ => return Err(format!("Unsupported sample format: {:?}", sample_format)),
        }
        .map_err(|e| format!("Failed to build input stream: {}", e))?;

        stream
            .play()
            .map_err(|e| format!("Failed to start stream: {}", e))?;

        Ok((stream, stream_config))
    }
}
