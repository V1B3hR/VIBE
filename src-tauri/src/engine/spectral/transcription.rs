#![allow(dead_code)]
use super::types::MelFrame;
use crate::engine::graph::{MidiClip, MidiNote};
use uuid::Uuid;

pub struct Transcriber {
    threshold: f32,
}

impl Transcriber {
    pub fn new() -> Self {
        Self {
            threshold: -30.0, // dB
        }
    }

    /// Transcribe a sequence of MelFrames to a MidiClip.
    /// Baseline implementation: Peak tracking across time.
    pub fn transcribe(&self, frames: &[MelFrame], sample_rate: f64, hop_size: usize) -> MidiClip {
        let mut notes = Vec::new();
        let num_mels = if frames.is_empty() {
            0
        } else {
            frames[0].data.len()
        };

        // State for active notes: mel_index -> (start_sample, max_velocity)
        let mut active_notes: Vec<Option<(u64, f32)>> = vec![None; num_mels];

        for frame in frames.iter() {
            let current_sample = frame.timestamp_samples;

            for m in 0..num_mels {
                let val = frame.data[m];

                if val > self.threshold {
                    // Note is active
                    if active_notes[m].is_none() {
                        active_notes[m] = Some((current_sample, val));
                    } else {
                        // Update max velocity
                        let (start, max_v) = active_notes[m].unwrap();
                        if val > max_v {
                            active_notes[m] = Some((start, val));
                        }
                    }
                } else {
                    // Note became inactive
                    if let Some((start, max_v)) = active_notes[m] {
                        let duration = current_sample.saturating_sub(start);
                        if duration > (sample_rate * 0.05) as u64 {
                            // Min 50ms
                            notes.push(MidiNote {
                                start_sample: start,
                                length_samples: duration,
                                note: self.mel_to_midi(m),
                                velocity: ((self.db_to_velocity_f32(max_v) * 127.0) as u32) << 25,
                                channel: 0,
                                pitch_bend: None,
                                pressure: None,
                                timbre: None,
                                probability: 1.0,
                                velocity_random: 0,
                                timing_random: 0,
                            });
                        }
                        active_notes[m] = None;
                    }
                }
            }
        }

        // Close remaining notes
        for m in 0..num_mels {
            if let Some((start, max_v)) = active_notes[m] {
                let end_sample = frames.last().map(|f| f.timestamp_samples).unwrap_or(start);
                let duration = end_sample.saturating_sub(start);

                notes.push(MidiNote {
                    start_sample: start,
                    length_samples: duration,
                    note: self.mel_to_midi(m),
                    velocity: ((self.db_to_velocity_f32(max_v) * 127.0) as u32) << 25,
                    channel: 0,
                    pitch_bend: None,
                    pressure: None,
                    timbre: None,
                    probability: 1.0,
                    velocity_random: 0,
                    timing_random: 0,
                });
            }
        }

        MidiClip {
            id: Uuid::new_v4(),
            name: "Transcribed MIDI".to_string(),
            start_sample: frames.first().map(|f| f.timestamp_samples).unwrap_or(0),
            length_samples: (frames.len() * hop_size) as u64,
            notes,
            cc_events: Vec::new(),
            color: "#ffcc00".to_string(),
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

    fn mel_to_midi(&self, mel_idx: usize) -> u16 {
        // Mel scale approximation to MIDI
        let freq = 20.0 * (2.0f32.powf(mel_idx as f32 / 128.0 * 10.0));
        let midi = 12.0 * (freq / 440.0).log2() + 69.0;
        midi.round().clamp(0.0, 127.0) as u16
    }

    fn db_to_velocity_f32(&self, db: f32) -> f32 {
        (db + 60.0).max(0.0) / 60.0
    }

    /// Convert MelFrames to a Drum MIDI Clip (Phase 3.4)
    pub fn transcribe_drums(&self, frames: &[MelFrame], sample_rate: f64) -> MidiClip {
        let mut detector = super::drum_detector::DrumDetector::new();
        let mut notes = Vec::new();

        for frame in frames {
            let events = detector.process_frame(frame, sample_rate);
            for event in events {
                // Convert MidiEvent (real-time) to MidiNote (clip)
                // Note: Length is fixed for drums (e.g. 100ms)
                notes.push(MidiNote {
                    start_sample: frame.timestamp_samples + event.sample_offset as u64,
                    length_samples: (sample_rate * 0.1) as u64,
                    note: event.data1,
                    velocity: event.data2,
                    channel: 9, // Standard Drum Channel
                    pitch_bend: None,
                    pressure: None,
                    timbre: None,
                    probability: 1.0,
                    velocity_random: 0,
                    timing_random: 0,
                });
            }
        }

        MidiClip {
            id: Uuid::new_v4(),
            name: "Transcribed Drums".to_string(),
            start_sample: frames.first().map(|f| f.timestamp_samples).unwrap_or(0),
            length_samples: frames.last().map(|f| f.timestamp_samples).unwrap_or(0)
                + (sample_rate * 0.1) as u64,
            notes,
            cc_events: Vec::new(),
            color: "#ff3366".to_string(), // Drum Red/Pink
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transcribe_empty() {
        let transcriber = Transcriber::new();
        let clip = transcriber.transcribe(&[], 48000.0, 512);
        assert!(clip.notes.is_empty());
    }

    #[test]
    fn test_transcribe_single_note() {
        let transcriber = Transcriber::new();
        let mut frames = Vec::new();

        // Single note at mel index 50, lasting 10 frames
        for i in 0..20 {
            let mut data = vec![-90.0; 128];
            if i >= 5 && i < 15 {
                data[50] = -10.0; // Above threshold
            }
            frames.push(MelFrame {
                data,
                timestamp_samples: (i * 512) as u64,
            });
        }

        let clip = transcriber.transcribe(&frames, 48000.0, 512);
        assert_eq!(clip.notes.len(), 1);
        let note = &clip.notes[0];
        assert_eq!(note.start_sample, 5 * 512);
        assert_eq!(note.length_samples, (15 - 5) * 512); // Duration until first inactive frame
        assert!(note.note > 0);
    }
}
