use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AudioSource {
    Mic,
    System,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptSegment {
    pub text: String,
    pub is_final: bool,
    pub timestamp: f64,
    pub confidence: f64,
    pub source: AudioSource,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ConnectionStatus {
    Disconnected,
    Connecting,
    Connected,
    Reconnecting,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionStatusEvent {
    pub source: AudioSource,
    pub status: ConnectionStatus,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioLevelEvent {
    pub source: AudioSource,
    pub level: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioDeviceInfo {
    pub name: String,
    pub id: String,
    pub is_default: bool,
}

/// Raw Deepgram WebSocket response
#[derive(Debug, Deserialize)]
pub struct DeepgramResponse {
    #[serde(rename = "type")]
    pub msg_type: String,
    pub channel: Option<DeepgramChannel>,
    pub is_final: Option<bool>,
    pub speech_final: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct DeepgramChannel {
    pub alternatives: Vec<DeepgramAlternative>,
}

#[derive(Debug, Deserialize)]
pub struct DeepgramAlternative {
    pub transcript: String,
    pub confidence: f64,
    pub words: Option<Vec<DeepgramWord>>,
}

#[derive(Debug, Deserialize)]
pub struct DeepgramWord {
    pub word: String,
    pub start: f64,
    pub end: f64,
    pub confidence: f64,
}

impl DeepgramResponse {
    /// Extract the transcript text, confidence, and finality from a Deepgram response.
    /// Returns None if the message is not a Results type, or if the transcript is empty.
    pub fn extract_transcript(&self) -> Option<(String, f64, bool)> {
        if self.msg_type != "Results" {
            return None;
        }
        let channel = self.channel.as_ref()?;
        let alt = channel.alternatives.first()?;
        if alt.transcript.is_empty() {
            return None;
        }
        let is_final = self.is_final.unwrap_or(false);
        Some((alt.transcript.clone(), alt.confidence, is_final))
    }

    /// Get the start timestamp from the first word in the response.
    /// Returns 0.0 if no words are present.
    pub fn start_timestamp(&self) -> f64 {
        self.channel
            .as_ref()
            .and_then(|c| c.alternatives.first())
            .and_then(|a| a.words.as_ref())
            .and_then(|w| w.first())
            .map(|w| w.start)
            .unwrap_or(0.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_response(is_final: bool) -> &'static str {
        if is_final {
            r#"{
                "type": "Results",
                "channel_index": [0, 1],
                "is_final": true,
                "speech_final": true,
                "channel": {
                    "alternatives": [{
                        "transcript": "Hello, how are you?",
                        "confidence": 0.98,
                        "words": [
                            {"word": "hello", "start": 0.5, "end": 0.85, "confidence": 0.99},
                            {"word": "how", "start": 0.9, "end": 1.1, "confidence": 0.97},
                            {"word": "are", "start": 1.15, "end": 1.3, "confidence": 0.96},
                            {"word": "you", "start": 1.35, "end": 1.5, "confidence": 0.98}
                        ]
                    }]
                }
            }"#
        } else {
            r#"{
                "type": "Results",
                "channel_index": [0, 1],
                "is_final": false,
                "speech_final": false,
                "channel": {
                    "alternatives": [{
                        "transcript": "Hello, how",
                        "confidence": 0.85,
                        "words": [
                            {"word": "hello", "start": 0.5, "end": 0.85, "confidence": 0.99},
                            {"word": "how", "start": 0.9, "end": 1.1, "confidence": 0.80}
                        ]
                    }]
                }
            }"#
        }
    }

    #[test]
    fn test_parse_final_transcript() {
        let resp: DeepgramResponse = serde_json::from_str(sample_response(true)).unwrap();
        let (text, confidence, is_final) = resp.extract_transcript().unwrap();
        assert_eq!(text, "Hello, how are you?");
        assert!((confidence - 0.98).abs() < 0.001);
        assert!(is_final);
    }

    #[test]
    fn test_parse_interim_transcript() {
        let resp: DeepgramResponse = serde_json::from_str(sample_response(false)).unwrap();
        let (text, _, is_final) = resp.extract_transcript().unwrap();
        assert_eq!(text, "Hello, how");
        assert!(!is_final);
    }

    #[test]
    fn test_start_timestamp() {
        let resp: DeepgramResponse = serde_json::from_str(sample_response(true)).unwrap();
        assert!((resp.start_timestamp() - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_empty_transcript_returns_none() {
        let json = r#"{"type": "Results", "is_final": true, "channel": {"alternatives": [{"transcript": "", "confidence": 0.0}]}}"#;
        let resp: DeepgramResponse = serde_json::from_str(json).unwrap();
        assert!(resp.extract_transcript().is_none());
    }

    #[test]
    fn test_metadata_message_returns_none() {
        let json = r#"{"type": "Metadata"}"#;
        let resp: DeepgramResponse = serde_json::from_str(json).unwrap();
        assert!(resp.extract_transcript().is_none());
    }
}
