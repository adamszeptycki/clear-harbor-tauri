use futures_util::{SinkExt, StreamExt};
use log::{error, info, warn};
use std::collections::VecDeque;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite;

use crate::transcription::types::{
    AudioSource, ConnectionStatus, ConnectionStatusEvent, DeepgramResponse, TranscriptSegment,
};

const MAX_RECONNECT_ATTEMPTS: u32 = 5;
const KEEPALIVE_INTERVAL_SECS: u64 = 10;

pub struct DeepgramConfig {
    pub api_key: String,
    pub language: String,
    pub model: String,
    pub sample_rate: u32,
}

impl Default for DeepgramConfig {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            language: "en".to_string(),
            model: "nova-2".to_string(),
            sample_rate: 16000,
        }
    }
}

fn build_ws_url(config: &DeepgramConfig) -> String {
    format!(
        "wss://api.deepgram.com/v1/listen?\
         encoding=linear16&\
         sample_rate={}&\
         channels=1&\
         punctuate=true&\
         smart_format=true&\
         interim_results=true&\
         endpointing=300&\
         vad_events=true&\
         language={}&\
         model={}",
        config.sample_rate, config.language, config.model,
    )
}

pub async fn run_deepgram_stream(
    source: AudioSource,
    config: DeepgramConfig,
    mut audio_rx: mpsc::Receiver<Vec<i16>>,
    transcript_tx: mpsc::Sender<TranscriptSegment>,
    status_tx: mpsc::Sender<ConnectionStatusEvent>,
    mut shutdown_rx: mpsc::Receiver<()>,
) {
    let url_str = build_ws_url(&config);
    let mut reconnect_attempts: u32 = 0;
    let mut audio_buffer: VecDeque<Vec<i16>> = VecDeque::new();
    let max_buffer_chunks = 30 * 1000 / 200; // ~150 chunks of 200ms

    'outer: loop {
        let _ = status_tx
            .send(ConnectionStatusEvent {
                source,
                status: if reconnect_attempts > 0 {
                    ConnectionStatus::Reconnecting
                } else {
                    ConnectionStatus::Connecting
                },
                error: None,
            })
            .await;

        // Build WebSocket request with auth header using ClientRequestBuilder.
        // The URI is parsed from the URL string; ClientRequestBuilder handles
        // WebSocket key generation and upgrade headers internally.
        let uri: tungstenite::http::Uri = match url_str.parse() {
            Ok(u) => u,
            Err(e) => {
                error!("Invalid Deepgram URL: {}", e);
                let _ = status_tx
                    .send(ConnectionStatusEvent {
                        source,
                        status: ConnectionStatus::Failed,
                        error: Some(format!("Invalid URL: {}", e)),
                    })
                    .await;
                return;
            }
        };

        let request = tungstenite::ClientRequestBuilder::new(uri)
            .with_header("Authorization", format!("Token {}", config.api_key));

        let ws_stream = match connect_async(request).await {
            Ok((stream, _)) => {
                info!("{:?} connected to Deepgram", source);
                reconnect_attempts = 0;
                let _ = status_tx
                    .send(ConnectionStatusEvent {
                        source,
                        status: ConnectionStatus::Connected,
                        error: None,
                    })
                    .await;
                stream
            }
            Err(e) => {
                error!("{:?} Deepgram connection failed: {}", source, e);
                reconnect_attempts += 1;
                if reconnect_attempts >= MAX_RECONNECT_ATTEMPTS {
                    let _ = status_tx
                        .send(ConnectionStatusEvent {
                            source,
                            status: ConnectionStatus::Failed,
                            error: Some(format!(
                                "Failed after {} attempts: {}",
                                MAX_RECONNECT_ATTEMPTS, e
                            )),
                        })
                        .await;
                    return;
                }
                let backoff = Duration::from_secs(
                    2u64.pow(reconnect_attempts.min(5)).min(30),
                );
                warn!(
                    "{:?} reconnecting in {:?} (attempt {})",
                    source, backoff, reconnect_attempts
                );
                time::sleep(backoff).await;
                continue;
            }
        };

        let (mut ws_sender, mut ws_receiver) = ws_stream.split();

        // Flush buffered audio
        while let Some(chunk) = audio_buffer.pop_front() {
            let bytes: Vec<u8> = chunk.iter().flat_map(|s| s.to_le_bytes()).collect();
            if ws_sender
                .send(tungstenite::Message::binary(bytes))
                .await
                .is_err()
            {
                break;
            }
        }

        let mut keepalive_interval = time::interval(Duration::from_secs(KEEPALIVE_INTERVAL_SECS));
        let mut last_audio_sent = time::Instant::now();

        loop {
            tokio::select! {
                audio = audio_rx.recv() => {
                    match audio {
                        Some(pcm) => {
                            let bytes: Vec<u8> = pcm.iter().flat_map(|s| s.to_le_bytes()).collect();
                            if let Err(e) = ws_sender.send(tungstenite::Message::binary(bytes)).await {
                                warn!("{:?} WS send error: {}", source, e);
                                audio_buffer.push_back(pcm);
                                if audio_buffer.len() > max_buffer_chunks {
                                    audio_buffer.pop_front();
                                }
                                continue 'outer;
                            }
                            last_audio_sent = time::Instant::now();
                        }
                        None => {
                            // Audio channel closed — send CloseStream and exit
                            let close_msg = serde_json::json!({"type": "CloseStream"});
                            let _ = ws_sender.send(tungstenite::Message::text(close_msg.to_string())).await;
                            time::sleep(Duration::from_secs(2)).await;
                            break 'outer;
                        }
                    }
                }
                msg = ws_receiver.next() => {
                    match msg {
                        Some(Ok(tungstenite::Message::Text(text))) => {
                            match serde_json::from_str::<DeepgramResponse>(&text) {
                                Ok(resp) => {
                                    if let Some((text, confidence, is_final)) = resp.extract_transcript() {
                                        let segment = TranscriptSegment {
                                            text,
                                            is_final,
                                            timestamp: resp.start_timestamp(),
                                            confidence,
                                            source,
                                        };
                                        let _ = transcript_tx.send(segment).await;
                                    }
                                }
                                Err(e) => warn!("{:?} parse error: {}", source, e),
                            }
                        }
                        Some(Ok(tungstenite::Message::Close(_))) | None => {
                            warn!("{:?} Deepgram WS closed", source);
                            continue 'outer;
                        }
                        Some(Err(e)) => {
                            error!("{:?} Deepgram WS error: {}", source, e);
                            continue 'outer;
                        }
                        _ => {}
                    }
                }
                _ = keepalive_interval.tick() => {
                    if last_audio_sent.elapsed() > Duration::from_secs(KEEPALIVE_INTERVAL_SECS) {
                        let keepalive = serde_json::json!({"type": "KeepAlive"});
                        let _ = ws_sender.send(tungstenite::Message::text(keepalive.to_string())).await;
                    }
                }
                _ = shutdown_rx.recv() => {
                    let close_msg = serde_json::json!({"type": "CloseStream"});
                    let _ = ws_sender.send(tungstenite::Message::text(close_msg.to_string())).await;
                    time::sleep(Duration::from_secs(2)).await;
                    break 'outer;
                }
            }
        }
    }

    let _ = status_tx
        .send(ConnectionStatusEvent {
            source,
            status: ConnectionStatus::Disconnected,
            error: None,
        })
        .await;
}
