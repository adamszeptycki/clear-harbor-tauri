use crate::transcription::types::{AudioSource, TranscriptSegment};

pub fn format_timestamp(seconds: f64) -> String {
    let total_secs = seconds as u64;
    let hours = total_secs / 3600;
    let mins = (total_secs % 3600) / 60;
    let secs = total_secs % 60;
    if hours > 0 {
        format!("{:02}:{:02}:{:02}", hours, mins, secs)
    } else {
        format!("{:02}:{:02}", mins, secs)
    }
}

pub fn export_markdown(segments: &[TranscriptSegment], show_timestamps: bool) -> String {
    let mut output = String::from("# DualScribe Transcript\n\n");
    let mic_segments: Vec<_> = segments
        .iter()
        .filter(|s| s.source == AudioSource::Mic && s.is_final)
        .collect();
    let sys_segments: Vec<_> = segments
        .iter()
        .filter(|s| s.source == AudioSource::System && s.is_final)
        .collect();

    if !mic_segments.is_empty() {
        output.push_str("## You\n\n");
        for seg in &mic_segments {
            if show_timestamps {
                output.push_str(&format!(
                    "**[{}]** {}\n\n",
                    format_timestamp(seg.timestamp),
                    seg.text
                ));
            } else {
                output.push_str(&format!("{}\n\n", seg.text));
            }
        }
    }
    if !sys_segments.is_empty() {
        output.push_str("## System Audio\n\n");
        for seg in &sys_segments {
            if show_timestamps {
                output.push_str(&format!(
                    "**[{}]** {}\n\n",
                    format_timestamp(seg.timestamp),
                    seg.text
                ));
            } else {
                output.push_str(&format!("{}\n\n", seg.text));
            }
        }
    }
    output
}

pub fn export_plain_text(segments: &[TranscriptSegment], show_timestamps: bool) -> String {
    let mut output = String::from("DualScribe Transcript\n\n");
    let mic_segments: Vec<_> = segments
        .iter()
        .filter(|s| s.source == AudioSource::Mic && s.is_final)
        .collect();
    let sys_segments: Vec<_> = segments
        .iter()
        .filter(|s| s.source == AudioSource::System && s.is_final)
        .collect();

    if !mic_segments.is_empty() {
        output.push_str("--- You ---\n\n");
        for seg in &mic_segments {
            if show_timestamps {
                output.push_str(&format!(
                    "[{}] {}\n",
                    format_timestamp(seg.timestamp),
                    seg.text
                ));
            } else {
                output.push_str(&format!("{}\n", seg.text));
            }
        }
        output.push('\n');
    }
    if !sys_segments.is_empty() {
        output.push_str("--- System Audio ---\n\n");
        for seg in &sys_segments {
            if show_timestamps {
                output.push_str(&format!(
                    "[{}] {}\n",
                    format_timestamp(seg.timestamp),
                    seg.text
                ));
            } else {
                output.push_str(&format!("{}\n", seg.text));
            }
        }
    }
    output
}

pub fn export_json(segments: &[TranscriptSegment]) -> Result<String, String> {
    let final_segments: Vec<_> = segments.iter().filter(|s| s.is_final).collect();
    serde_json::to_string_pretty(&final_segments).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transcription::types::AudioSource;

    fn sample_segments() -> Vec<TranscriptSegment> {
        vec![
            TranscriptSegment {
                text: "Hello there.".to_string(),
                is_final: true,
                timestamp: 1.5,
                confidence: 0.98,
                source: AudioSource::Mic,
            },
            TranscriptSegment {
                text: "Welcome to the meeting.".to_string(),
                is_final: true,
                timestamp: 2.0,
                confidence: 0.95,
                source: AudioSource::System,
            },
            TranscriptSegment {
                text: "partial".to_string(),
                is_final: false,
                timestamp: 3.0,
                confidence: 0.5,
                source: AudioSource::Mic,
            },
        ]
    }

    #[test]
    fn test_format_timestamp_minutes() {
        assert_eq!(format_timestamp(65.0), "01:05");
    }

    #[test]
    fn test_format_timestamp_hours() {
        assert_eq!(format_timestamp(3661.0), "01:01:01");
    }

    #[test]
    fn test_markdown_with_timestamps() {
        let md = export_markdown(&sample_segments(), true);
        assert!(md.contains("## You"));
        assert!(md.contains("**[00:01]** Hello there."));
        assert!(md.contains("## System Audio"));
        assert!(md.contains("**[00:02]** Welcome to the meeting."));
        assert!(!md.contains("partial"));
    }

    #[test]
    fn test_plain_text_without_timestamps() {
        let txt = export_plain_text(&sample_segments(), false);
        assert!(txt.contains("--- You ---"));
        assert!(txt.contains("Hello there."));
        assert!(!txt.contains("[00:01]"));
        assert!(!txt.contains("partial"));
    }

    #[test]
    fn test_json_export() {
        let json = export_json(&sample_segments()).unwrap();
        let parsed: Vec<TranscriptSegment> = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.len(), 2); // only finals
        assert_eq!(parsed[0].text, "Hello there.");
        assert_eq!(parsed[1].text, "Welcome to the meeting.");
    }
}
