use super::types::MelFrame;
use crate::engine::graph::{MidiClip, MidiNote};
use uuid::Uuid;

pub struct PolyphonicConverter {
    sample_rate: f64,
    hop_size: usize,
    adaptive_threshold: MovingAverage,
    prev_magnitude: Option<Vec<f32>>,
}

struct MovingAverage {
    buffer: Vec<f32>,
    size: usize,
}

impl MovingAverage {
    fn new(size: usize) -> Self {
        Self {
            buffer: Vec::with_capacity(size),
            size,
        }
    }

    fn update(&mut self, val: f32) {
        if self.buffer.len() >= self.size {
            self.buffer.remove(0);
        }
        self.buffer.push(val);
    }

    fn current(&self) -> f32 {
        if self.buffer.is_empty() {
            return -100.0;
        }
        self.buffer.iter().sum::<f32>() / self.buffer.len() as f32
    }
}

impl PolyphonicConverter {
    pub fn new(sample_rate: f64, hop_size: usize) -> Self {
        Self {
            sample_rate,
            hop_size,
            adaptive_threshold: MovingAverage::new(100),
            prev_magnitude: None,
        }
    }

    /// Entry point for Quantum Level 4 conversion
    pub fn convert(&mut self, frames: &[MelFrame]) -> MidiClip {
        let mut notes = Vec::new();
        let num_mels = if frames.is_empty() {
            0
        } else {
            frames[0].data.len()
        };
        let mut active_voices: Vec<Option<(u64, f32)>> = vec![None; num_mels];

        for frame in frames {
            let timestamp = frame.timestamp_samples;

            // 1. Hybrid Detection: Compute Spectral Flux
            let flux = self.compute_flux(&frame.data);
            let is_onset = flux > 8.0; // Quantum Level 1 threshold

            // 2. Adaptive Threshold (Quantum Level 2)
            let avg_energy = frame.data.iter().sum::<f32>() / num_mels as f32;
            self.adaptive_threshold.update(avg_energy);
            let threshold = self.adaptive_threshold.current() + 12.0;

            // 3. Harmonic Masking (Quantum Level 3 - Suppression)
            let mut cleaned_data = frame.data.clone();
            self.suppress_harmonics(&mut cleaned_data);

            // 4. Peak Picking & Tracking
            for m in 0..num_mels {
                let energy = cleaned_data[m];
                let is_peak = self.is_spectral_peak(&cleaned_data, m);

                if is_peak && energy > threshold {
                    if active_voices[m].is_none() {
                        // Start note only if it's a transient OR a very strong new peak
                        if is_onset || energy > threshold + 10.0 {
                            active_voices[m] = Some((timestamp, energy));
                        }
                    } else {
                        // Update max energy for velocity mapping (Quantum Level 4)
                        let (start, max_e) = active_voices[m].unwrap();
                        if energy > max_e {
                            active_voices[m] = Some((start, energy));
                        }
                    }
                } else if let Some((start, max_e)) = active_voices[m] {
                    let duration = timestamp.saturating_sub(start);
                    if duration > (self.sample_rate * 0.04) as u64 {
                        // Min 40ms
                        notes.push(self.create_note(m, start, duration, max_e));
                    }
                    active_voices[m] = None;
                }
            }
            self.prev_magnitude = Some(frame.data.clone());
        }

        // Close remaining notes
        if let Some(last_frame) = frames.last() {
            for m in 0..num_mels {
                if let Some((start, max_e)) = active_voices[m] {
                    let duration = last_frame.timestamp_samples.saturating_sub(start);
                    notes.push(self.create_note(m, start, duration, max_e));
                }
            }
        }

        MidiClip {
            id: Uuid::new_v4(),
            name: "Quantum Polyphonic MIDI".to_string(),
            start_sample: frames.first().map(|f| f.timestamp_samples).unwrap_or(0),
            length_samples: (frames.len() * self.hop_size) as u64,
            notes,
            cc_events: Vec::new(),
            color: "#00eeff".to_string(),
            is_muted: false,
            is_looped: false,
            scale: None,
            chord_markers: Vec::new(),
            groove_template: None,
            pattern_id: None,
            tuning_steps: None,
            time_signature_num: None,
            time_signature_den: None,
            reference_clip_id: None,
        }
    }

    fn compute_flux(&self, current: &[f32]) -> f32 {
        if let Some(prev) = &self.prev_magnitude {
            let mut flux = 0.0;
            for (&c, &p) in current.iter().zip(prev.iter()) {
                let d = c - p;
                if d > 0.0 {
                    flux += d;
                }
            }
            flux / current.len() as f32
        } else {
            0.0
        }
    }

    fn suppress_harmonics(&self, data: &mut [f32]) {
        let n = data.len();
        // Simple Harmonic Product Spectrum (HPS) style suppression
        // If bin i is strong, bin 2*i and 3*i are likely harmonics.
        // Since Mel is logarithmic, i+12 (approx an octave) is a multiple.
        for i in 0..n {
            if data[i] > -15.0 {
                // Suppress likely harmonics in higher bins
                for k in 2..4 {
                    let harmonic_idx = i + (12 * (k - 1)); // Rough approximation for 12 bins per octave
                    if harmonic_idx < n {
                        data[harmonic_idx] -= 6.0; // Dampen by 6dB
                    }
                }
            }
        }
    }

    fn is_spectral_peak(&self, data: &[f32], idx: usize) -> bool {
        let val = data[idx];
        let left = if idx > 0 { data[idx - 1] } else { -100.0 };
        let right = if idx < data.len() - 1 {
            data[idx + 1]
        } else {
            -100.0
        };
        val > left && val > right
    }

    fn create_note(&self, mel_idx: usize, start: u64, duration: u64, energy: f32) -> MidiNote {
        let (note, bend) = self.mel_to_precise_midi(mel_idx);
        MidiNote {
            start_sample: start,
            length_samples: duration,
            note,
            velocity: self.estimate_velocity(energy, mel_idx),
            channel: 0,
            pitch_bend: bend,
            pressure: None,
            timbre: None,
            probability: 1.0,
            velocity_random: 0,
            timing_random: 0,
        }
    }

    fn mel_to_precise_midi(&self, mel_idx: usize) -> (u16, Option<i16>) {
        // Mel scale approximation
        let f_min = 20.0f32;
        let f_max = 20000.0f32;
        let m_min = 2595.0f32 * (1.0f32 + f_min / 700.0f32).log10();
        let m_max = 2595.0f32 * (1.0f32 + f_max / 700.0f32).log10();
        let mel = m_min + (mel_idx as f32 / 128.0f32) * (m_max - m_min);
        let freq = 700.0f32 * (10.0f32.powf(mel / 2595.0f32) - 1.0f32);

        let midi_raw = 12.0f32 * (freq / 440.0f32).log2() + 69.0f32;
        let note = midi_raw.round() as u16;
        let detune = midi_raw - note as f32; // -0.5 to 0.5 semitones

        // Convert detune to MIDI Pitch Bend (-8192 to 8191)
        // Standard range is +/- 2 semitones.
        // 1 semitone = 4096 units.
        let bend = (detune * 4096.0) as i16;

        (
            note.clamp(0, 127),
            if bend.abs() > 50 { Some(bend) } else { None },
        )
    }

    fn estimate_velocity(&self, energy: f32, _mel_idx: usize) -> u32 {
        // Quantum Level 4: Spectral Energy Mapping
        let vol = (energy + 50.0).max(0.0) / 50.0;
        ((vol * 127.0).min(127.0) as u32) << 25
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quantum_polyphony() {
        let mut converter = PolyphonicConverter::new(48000.0, 512);
        let mut frame = MelFrame {
            data: vec![-100.0; 128],
            timestamp_samples: 0,
        };

        // Silence
        let clip1 = converter.convert(&[frame.clone()]);
        assert_eq!(clip1.notes.len(), 0);

        // A single peak
        frame.data[60] = -5.0; // Strong Peak
        let f2 = frame.clone();
        let mut f3 = f2.clone();
        f3.timestamp_samples = 10000;
        f3.data[60] = -90.0; // End note

        let clip2 = converter.convert(&[f2, f3]);
        assert!(clip2.notes.len() >= 1);
    }
}
