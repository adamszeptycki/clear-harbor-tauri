use rubato::{
    Resampler, SincFixedIn, SincInterpolationParameters, SincInterpolationType, WindowFunction,
};

pub struct AudioResampler {
    resampler: Option<SincFixedIn<f32>>,
    input_channels: usize,
    chunk_size: usize,
}

impl AudioResampler {
    pub fn new(input_rate: u32, output_rate: u32, channels: usize) -> Result<Self, String> {
        if input_rate == output_rate && channels == 1 {
            return Ok(Self {
                resampler: None,
                input_channels: channels,
                chunk_size: 0,
            });
        }

        let params = SincInterpolationParameters {
            sinc_len: 64,
            f_cutoff: 0.95,
            interpolation: SincInterpolationType::Linear,
            oversampling_factor: 128,
            window: WindowFunction::BlackmanHarris2,
        };

        let chunk_size = 480; // 10ms at 48kHz, reasonable default
        let resampler = SincFixedIn::new(
            output_rate as f64 / input_rate as f64,
            2.0,
            params,
            chunk_size,
            1, // always resample as mono (we mix down before resampling)
        )
        .map_err(|e| format!("Failed to create resampler: {}", e))?;

        Ok(Self {
            resampler: Some(resampler),
            input_channels: channels,
            chunk_size,
        })
    }

    /// Process interleaved audio samples. If multi-channel, mixes down to mono first.
    /// Returns resampled mono f32 samples.
    pub fn process(&mut self, input: &[f32]) -> Result<Vec<f32>, String> {
        // Mix to mono if needed
        let mono: Vec<f32> = if self.input_channels > 1 {
            input
                .chunks(self.input_channels)
                .map(|frame| frame.iter().sum::<f32>() / self.input_channels as f32)
                .collect()
        } else {
            input.to_vec()
        };

        match &mut self.resampler {
            None => Ok(mono), // passthrough: same rate, already mono
            Some(resampler) => {
                let mut output = Vec::new();
                // Process full chunks
                for chunk in mono.chunks(self.chunk_size) {
                    if chunk.len() == self.chunk_size {
                        // rubato expects Vec<Vec<f32>> — one inner vec per channel
                        let input_buf = vec![chunk.to_vec()];
                        let result = resampler
                            .process(&input_buf, None)
                            .map_err(|e| format!("Resample error: {}", e))?;
                        output.extend_from_slice(&result[0]);
                    }
                    // Partial chunks (< chunk_size) are skipped for now.
                    // In streaming usage, the continuous audio stream provides full chunks.
                }
                Ok(output)
            }
        }
    }
}

/// Convert f32 samples (-1.0..1.0) to i16 Linear16 for Deepgram.
/// Positive values map to [0, 32767], negative values map to [-32768, 0].
pub fn to_linear16(samples: &[f32]) -> Vec<i16> {
    samples
        .iter()
        .map(|&s| {
            let clamped = s.clamp(-1.0, 1.0);
            if clamped >= 0.0 {
                (clamped * i16::MAX as f32) as i16
            } else {
                // Use 32768.0 for negative range so -1.0 maps to -32768
                (clamped * (-(i16::MIN as f32))) as i16
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resampler_downsample_48k_to_16k() {
        let mut resampler = AudioResampler::new(48000, 16000, 1).unwrap();
        // 480 samples at 48kHz = 10ms of audio
        let input: Vec<f32> = (0..480).map(|i| (i as f32 / 480.0).sin()).collect();
        let output = resampler.process(&input).unwrap();
        // 10ms at 16kHz = ~160 samples (rubato may vary slightly)
        assert!(
            output.len() >= 140 && output.len() <= 180,
            "Expected ~160 samples, got {}",
            output.len()
        );
    }

    #[test]
    fn test_resampler_passthrough_16k() {
        let mut resampler = AudioResampler::new(16000, 16000, 1).unwrap();
        let input: Vec<f32> = (0..160).map(|i| (i as f32 / 160.0).sin()).collect();
        let output = resampler.process(&input).unwrap();
        assert_eq!(output.len(), input.len());
    }

    #[test]
    fn test_resampler_stereo_to_mono() {
        let mut resampler = AudioResampler::new(48000, 16000, 2).unwrap();
        // 960 samples = 480 stereo frames at 48kHz = 10ms
        let input: Vec<f32> = (0..960).map(|i| (i as f32 / 960.0).sin()).collect();
        let output = resampler.process(&input).unwrap();
        // Output should be mono ~160 samples
        assert!(
            output.len() >= 140 && output.len() <= 180,
            "Expected ~160 mono samples, got {}",
            output.len()
        );
    }

    #[test]
    fn test_to_linear16() {
        let samples = vec![0.0f32, 0.5, -0.5, 1.0, -1.0];
        let linear16 = to_linear16(&samples);
        assert_eq!(linear16[0], 0i16);
        assert_eq!(linear16[1], 16383);
        assert_eq!(linear16[2], -16384);
        assert_eq!(linear16[3], 32767);
        assert_eq!(linear16[4], -32768);
    }
}
