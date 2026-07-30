use crate::engine::graph::MidiClip;
use crate::engine::spectral::{MelProcessor, MelSpectrogramConfig};

pub struct AudioToMidiConverter {
    sample_rate: f64,
}

impl AudioToMidiConverter {
    pub fn new(sample_rate: f64) -> Self {
        Self { sample_rate }
    }

    /// Converts an audio buffer into a MidiClip using Quantum Level 4 spectral polyphonic detection.
    pub fn convert_polyphonic(&self, samples: &[f32]) -> MidiClip {
        let config = MelSpectrogramConfig::default();
        let processor = MelProcessor::new(config.clone());
        let mut poly_converter =
            crate::engine::spectral::PolyphonicConverter::new(self.sample_rate, config.hop_size);

        let mut frames = Vec::new();
        let mut offset = 0;
        let fft_size = config.fft_size;
        let hop_size = config.hop_size;

        while offset + fft_size <= samples.len() {
            let frame_samples = &samples[offset..offset + fft_size];
            let frame = processor.process_frame(frame_samples, offset as u64);
            frames.push(frame);
            offset += hop_size;
        }

        let mut clip = poly_converter.convert(&frames);
        clip.name = "Quantum Polyphonic MIDI".to_string();
        clip
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_convert_polyphonic_simple() {
        let converter = AudioToMidiConverter::new(48000.0);
        let samples = vec![0.0; 4096]; // Silence
        let clip = converter.convert_polyphonic(&samples);
        assert_eq!(clip.notes.len(), 0);
    }
}
