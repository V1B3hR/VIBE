use super::types::MelFrame;
use crate::engine::audio_commands::MidiEvent;

pub struct DrumDetector {
    kick_threshold: f32,
    snare_threshold: f32,
    hat_threshold: f32,

    // State for triggering
    last_trigger_samples: [u64; 3], // 0: Kick, 1: Snare, 2: Hat
    prev_energies: [f32; 3],
    triggered: [bool; 3],
}

impl DrumDetector {
    pub fn new() -> Self {
        Self {
            kick_threshold: -18.0, // dB (Higher = less sensitive)
            snare_threshold: -25.0,
            hat_threshold: -35.0,
            last_trigger_samples: [0; 3],
            prev_energies: [-100.0; 3],
            triggered: [false; 3],
        }
    }

    pub fn process_frame(&mut self, frame: &MelFrame, sample_rate: f64) -> Vec<MidiEvent> {
        let mut events = Vec::new();
        let timestamp = frame.timestamp_samples;

        // Band Energies (Refined for Mel scale bins)
        let kick_energy = self.get_band_energy(&frame.data, 0, 8); // Sub-bass
        let snare_energy = self.get_band_energy(&frame.data, 35, 65); // Mids
        let hat_energy = self.get_band_energy(&frame.data, 80, 128); // Highs

        // 1. KICK (C1 - 36)
        if self.should_trigger(
            0,
            kick_energy,
            self.kick_threshold,
            timestamp,
            sample_rate,
            0.1,
        ) {
            events.push(MidiEvent {
                sample_offset: 0,
                status: 0x99,
                data1: 36,
                data2: (self.energy_to_velocity(kick_energy) as u32) << 25,
            });
            self.last_trigger_samples[0] = timestamp;
            self.triggered[0] = true;
        }

        // 2. SNARE (D1 - 38)
        if self.should_trigger(
            1,
            snare_energy,
            self.snare_threshold,
            timestamp,
            sample_rate,
            0.1,
        ) {
            events.push(MidiEvent {
                sample_offset: 0,
                status: 0x99,
                data1: 38,
                data2: (self.energy_to_velocity(snare_energy) as u32) << 25,
            });
            self.last_trigger_samples[1] = timestamp;
            self.triggered[1] = true;
        }

        // 3. HAT (F#1 - 42)
        if self.should_trigger(
            2,
            hat_energy,
            self.hat_threshold,
            timestamp,
            sample_rate,
            0.05,
        ) {
            events.push(MidiEvent {
                sample_offset: 0,
                status: 0x99,
                data1: 42,
                data2: (self.energy_to_velocity(hat_energy) as u32) << 25,
            });
            self.last_trigger_samples[2] = timestamp;
            self.triggered[2] = true;
        }

        // Update state
        self.prev_energies[0] = kick_energy;
        self.prev_energies[1] = snare_energy;
        self.prev_energies[2] = hat_energy;

        events
    }

    /// Logic for detecting a trigger:
    /// - Must be above threshold
    /// - Must be a local peak (rising energy)
    /// - Must respect refractory period (debouncing)
    fn should_trigger(
        &self,
        idx: usize,
        energy: f32,
        threshold: f32,
        timestamp: u64,
        sample_rate: f64,
        refractory_sec: f64,
    ) -> bool {
        // Threshold check
        if energy < threshold {
            return false;
        }

        // Debounce check
        if self.triggered[idx] {
            let diff_samples = timestamp.saturating_sub(self.last_trigger_samples[idx]);
            let diff_sec = diff_samples as f64 / sample_rate;
            if diff_sec < refractory_sec {
                return false;
            }
        }

        // Peak Detection (Energy must be significantly higher than previous)
        if energy < self.prev_energies[idx] + 2.0 {
            return false;
        }

        true
    }

    fn get_band_energy(&self, data: &[f32], start: usize, end: usize) -> f32 {
        let sub = &data[start.min(data.len())..end.min(data.len())];
        if sub.is_empty() {
            return -100.0;
        }
        sub.iter().sum::<f32>() / sub.len() as f32
    }

    fn energy_to_velocity(&self, db: f32) -> u8 {
        let vol = (db + 50.0).max(0.0) / 50.0; // Dynamic range mapping
        (vol * 127.0).min(127.0) as u8
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_drum_detection_kick() {
        let mut detector = DrumDetector::new();
        let mut data = vec![-100.0; 128];

        // 1. Send first frame above threshold (should trigger)
        for i in 0..8 {
            data[i] = -10.0;
        }
        let frame1 = MelFrame {
            data: data.clone(),
            timestamp_samples: 0,
        };
        let events1 = detector.process_frame(&frame1, 48000.0);
        assert!(events1.iter().any(|e| e.data1 == 36));

        // 2. Send second frame (same energy, should NOT trigger due to peak detection)
        let frame2 = MelFrame {
            data: data.clone(),
            timestamp_samples: 512,
        };
        let events2 = detector.process_frame(&frame2, 48000.0);
        assert!(events2.is_empty());

        // 3. Send third frame (higher energy, should trigger again)
        for i in 0..8 {
            data[i] = -5.0;
        }
        let frame3 = MelFrame {
            data: data.clone(),
            timestamp_samples: 10000,
        };
        let events3 = detector.process_frame(&frame3, 48000.0);
        assert!(events3.iter().any(|e| e.data1 == 36));
    }

    #[test]
    fn test_drum_detection_debounce() {
        let mut detector = DrumDetector::new();
        let mut data = vec![-100.0; 128];
        for i in 80..128 {
            data[i] = -20.0;
        }

        // Trigger 1
        let f1 = MelFrame {
            data: data.clone(),
            timestamp_samples: 0,
        };
        let e1 = detector.process_frame(&f1, 48000.0);
        assert!(!e1.is_empty());

        // Trigger 2 (Too soon - 10ms later)
        for i in 80..128 {
            data[i] = -10.0;
        } // Higher energy but too soon
        let f2 = MelFrame {
            data: data.clone(),
            timestamp_samples: 480,
        };
        let e2 = detector.process_frame(&f2, 48000.0);
        assert!(e2.is_empty());
    }
}
