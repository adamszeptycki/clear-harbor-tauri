use crossbeam_channel::Receiver as CbReceiver;
use log::{error, info};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use tokio::sync::mpsc;

use crate::audio::mic_capture::{AudioChunk, MicCapture};
use crate::audio::resampler::{to_linear16, AudioResampler};
use crate::audio::system_capture;
use crate::transcription::deepgram_client::{self, DeepgramConfig};
use crate::transcription::types::{
    AudioLevelEvent, AudioSource, ConnectionStatus, ConnectionStatusEvent, TranscriptSegment,
};

/// Manages the lifecycle of mic and system audio capture streams,
/// resampling, and Deepgram WebSocket connections.
///
/// Because `cpal::Stream` is !Send on most platforms, the mic capture
/// stream handle is kept on the thread that created it. We use an
/// `AtomicBool` stop flag to signal the capture thread to drop it.
pub struct StreamManager {
    mic_shutdown_tx: Option<mpsc::Sender<()>>,
    system_shutdown_tx: Option<mpsc::Sender<()>>,
    mic_stop_flag: Option<Arc<AtomicBool>>,
    system_capture_handle: Option<Box<dyn system_capture::CaptureHandle>>,
}

impl StreamManager {
    pub fn new() -> Self {
        Self {
            mic_shutdown_tx: None,
            system_shutdown_tx: None,
            mic_stop_flag: None,
            system_capture_handle: None,
        }
    }

    pub fn start(
        &mut self,
        app_handle: AppHandle,
        api_key: String,
        language: String,
        mic_device_id: Option<String>,
        _system_device_id: Option<String>,
    ) -> Result<(), String> {
        // --- Mic stream ---
        // cpal::Stream is !Send, so we create and hold the capture on a
        // dedicated std::thread. The thread exits when the stop flag is set,
        // which also drops the Stream.
        let (audio_cb_tx, audio_cb_rx) = crossbeam_channel::bounded::<AudioChunk>(50);
        let mic_stop_flag = Arc::new(AtomicBool::new(false));
        let stop_flag_clone = mic_stop_flag.clone();

        // We need sample_rate and channels from the mic config for the resampler.
        // These are sent back via a oneshot-like crossbeam channel.
        let (config_tx, config_rx) =
            crossbeam_channel::bounded::<Result<(u32, u16), String>>(1);

        let mic_device_id_clone = mic_device_id.clone();
        std::thread::spawn(move || {
            let mic_capture = MicCapture::new();
            let mic_device = match mic_capture.get_device(mic_device_id_clone.as_deref()) {
                Ok(d) => d,
                Err(e) => {
                    let _ = config_tx.send(Err(e));
                    return;
                }
            };

            let (_stream, stream_config) =
                match mic_capture.start_capture(&mic_device, audio_cb_tx) {
                    Ok(s) => s,
                    Err(e) => {
                        let _ = config_tx.send(Err(e));
                        return;
                    }
                };

            let sample_rate = stream_config.sample_rate.0;
            let channels = stream_config.channels;
            let _ = config_tx.send(Ok((sample_rate, channels)));

            // Keep _stream alive (it captures audio via its callback).
            // Wait until stop flag is set, then drop.
            while !stop_flag_clone.load(Ordering::Relaxed) {
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
            // _stream is dropped here, stopping capture.
        });

        let (mic_sample_rate, mic_channels) = config_rx
            .recv()
            .map_err(|e| format!("Mic config channel error: {}", e))?
            .map_err(|e| format!("Mic capture error: {}", e))?;

        self.mic_stop_flag = Some(mic_stop_flag);

        let (mic_shutdown_tx, mic_shutdown_rx) = mpsc::channel::<()>(1);
        self.mic_shutdown_tx = Some(mic_shutdown_tx);

        let config1 = DeepgramConfig {
            api_key: api_key.clone(),
            language: language.clone(),
            ..Default::default()
        };
        spawn_stream_pipeline(
            app_handle.clone(),
            AudioSource::Mic,
            config1,
            audio_cb_rx,
            mic_sample_rate,
            mic_channels as usize,
            mic_shutdown_rx,
        );

        // --- System stream ---
        let sys_capture = system_capture::create_system_capture();
        let (sys_cb_tx, sys_cb_rx) = crossbeam_channel::bounded::<AudioChunk>(50);
        match sys_capture.start_capture(_system_device_id.as_deref(), sys_cb_tx) {
            Ok(handle) => {
                self.system_capture_handle = Some(handle);
                let (sys_shutdown_tx, sys_shutdown_rx) = mpsc::channel::<()>(1);
                self.system_shutdown_tx = Some(sys_shutdown_tx);

                let config2 = DeepgramConfig {
                    api_key,
                    language,
                    ..Default::default()
                };
                spawn_stream_pipeline(
                    app_handle.clone(),
                    AudioSource::System,
                    config2,
                    sys_cb_rx,
                    44100,
                    1,
                    sys_shutdown_rx,
                );
            }
            Err(e) => {
                error!("System audio capture failed: {}", e);
                let _ = app_handle.emit(
                    "connection-status",
                    ConnectionStatusEvent {
                        source: AudioSource::System,
                        status: ConnectionStatus::Failed,
                        error: Some(e),
                    },
                );
            }
        }

        info!("Stream manager started");
        Ok(())
    }

    pub fn stop(&mut self) {
        // Signal Deepgram WS tasks to shut down
        if let Some(tx) = self.mic_shutdown_tx.take() {
            let _ = tx.try_send(());
        }
        if let Some(tx) = self.system_shutdown_tx.take() {
            let _ = tx.try_send(());
        }

        // Signal the mic capture thread to stop and drop the cpal::Stream
        if let Some(flag) = self.mic_stop_flag.take() {
            flag.store(true, Ordering::Relaxed);
        }

        // Stop system capture
        if let Some(handle) = self.system_capture_handle.take() {
            handle.stop();
        }

        info!("Stream manager stopped");
    }
}

fn spawn_stream_pipeline(
    app_handle: AppHandle,
    source: AudioSource,
    config: DeepgramConfig,
    audio_rx: CbReceiver<AudioChunk>,
    input_sample_rate: u32,
    input_channels: usize,
    shutdown_rx: mpsc::Receiver<()>,
) {
    let (pcm_tx, pcm_rx) = mpsc::channel::<Vec<i16>>(100);
    let (transcript_tx, mut transcript_rx) = mpsc::channel::<TranscriptSegment>(100);
    let (status_tx, mut status_rx) = mpsc::channel::<ConnectionStatusEvent>(10);

    // Thread: crossbeam -> resample -> compute level -> tokio channel
    let app_for_level = app_handle.clone();
    std::thread::spawn(move || {
        let mut resampler = match AudioResampler::new(input_sample_rate, 16000, input_channels) {
            Ok(r) => r,
            Err(e) => {
                error!("{:?} resampler init failed: {}", source, e);
                return;
            }
        };

        while let Ok(chunk) = audio_rx.recv() {
            // Compute RMS audio level
            let rms = (chunk
                .samples
                .iter()
                .map(|s| s * s)
                .sum::<f32>()
                / chunk.samples.len() as f32)
                .sqrt();
            let _ = app_for_level.emit(
                "audio-level",
                AudioLevelEvent {
                    source,
                    level: rms.min(1.0),
                },
            );

            match resampler.process(&chunk.samples) {
                Ok(resampled) => {
                    if !resampled.is_empty() {
                        let linear16 = to_linear16(&resampled);
                        if pcm_tx.blocking_send(linear16).is_err() {
                            break;
                        }
                    }
                }
                Err(e) => error!("{:?} resample error: {}", source, e),
            }
        }
    });

    // Tokio task: Deepgram WS
    tauri::async_runtime::spawn(async move {
        deepgram_client::run_deepgram_stream(
            source,
            config,
            pcm_rx,
            transcript_tx,
            status_tx,
            shutdown_rx,
        )
        .await;
    });

    // Tokio task: forward transcripts to Tauri events
    let app_for_transcripts = app_handle.clone();
    tauri::async_runtime::spawn(async move {
        let event_name = match source {
            AudioSource::Mic => "mic-transcript",
            AudioSource::System => "system-transcript",
        };
        while let Some(segment) = transcript_rx.recv().await {
            let _ = app_for_transcripts.emit(event_name, &segment);
        }
    });

    // Tokio task: forward connection status to Tauri events
    tauri::async_runtime::spawn(async move {
        while let Some(status) = status_rx.recv().await {
            let _ = app_handle.emit("connection-status", &status);
        }
    });
}
