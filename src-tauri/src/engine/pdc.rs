#![allow(dead_code)]

use super::graph::Track;

/// PDC (Plugin Delay Compensation) Manager
/// Calculates and applies delay compensation across all tracks
pub struct PdcManager {
    max_latency_samples: usize,
}

impl PdcManager {
    pub fn new() -> Self {
        Self {
            max_latency_samples: 0,
        }
    }

    /// Calculate maximum latency across all processors in a track
    pub fn calculate_track_latency(processors: &[Box<dyn super::graph::AudioProcessor>]) -> usize {
        processors.iter().map(|p| p.latency_samples()).sum()
    }

    /// Recalculate PDC for all tracks in a standard linear mixer setup
    pub fn recalculate_project_pdc(tracks: &mut [Track]) {
        if tracks.is_empty() {
            return;
        }

        // 1. Get latency for each track
        let track_latencies: Vec<usize> = tracks
            .iter()
            .map(|t| Self::calculate_track_latency(&t.processors))
            .collect();

        // 2. Find max latency
        let max_latency = track_latencies.iter().cloned().max().unwrap_or(0);

        // 3. Assign compensation to each track
        for (i, track) in tracks.iter_mut().enumerate() {
            let compensation = max_latency - track_latencies[i];
            track.pdc_delay_samples = compensation;

            // 4. Ensure buffer is large enough
            // Buffer should be enough to hold the delay + current block
            let needed_size = compensation + super::graph::MAX_BUFFER_SIZE;
            if track.pdc_delay_buffer.is_empty() {
                track.pdc_delay_buffer = vec![vec![0.0; needed_size]; super::graph::MAX_CHANNELS];
            } else {
                for ch in 0..track.pdc_delay_buffer.len() {
                    if track.pdc_delay_buffer[ch].len() < needed_size {
                        track.pdc_delay_buffer[ch].resize(needed_size, 0.0);
                    }
                }
            }
        }
    }

    /// Apply delay compensation to a track's output using a circular buffer approach
    pub fn apply_compensation(
        delay_buffer: &mut Vec<Vec<f64>>,
        channels_data: &mut [&mut [f64]],
        delay_samples: usize,
        write_cursor: &mut usize,
        frames: usize,
    ) {
        if delay_samples == 0 || delay_buffer.is_empty() {
            return;
        }

        let buffer_len = delay_buffer[0].len();
        let num_chans = channels_data.len();

        // Process each channel
        for ch in 0..num_chans {
            if ch >= delay_buffer.len() {
                break;
            }

            let mut local_w = *write_cursor;
            // Read cursor follows write cursor by `delay_samples` (modulo buffer length)
            let mut local_r = (local_w + buffer_len - delay_samples) % buffer_len;

            let buf = &mut delay_buffer[ch];
            let io_buf = &mut *channels_data[ch];

            // 1. Write Input to Circular Buffer
            let mut frames_written = 0;
            while frames_written < frames {
                let chunk = (buffer_len - local_w).min(frames - frames_written);
                buf[local_w..local_w + chunk]
                    .copy_from_slice(&io_buf[frames_written..frames_written + chunk]);

                local_w = (local_w + chunk) % buffer_len;
                frames_written += chunk;
            }

            // 2. Read Delayed Data from Circular Buffer to Output
            let mut frames_read = 0;
            while frames_read < frames {
                let chunk = (buffer_len - local_r).min(frames - frames_read);
                io_buf[frames_read..frames_read + chunk]
                    .copy_from_slice(&buf[local_r..local_r + chunk]);

                local_r = (local_r + chunk) % buffer_len;
                frames_read += chunk;
            }
        }

        // Update global write cursor
        *write_cursor = (*write_cursor + frames) % buffer_len;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::graph::{AudioBuffer, AudioProcessor, ProcessingContext, Track};
    use uuid::Uuid;

    struct LatencyProcessor {
        latency: usize,
        id: Uuid,
    }

    impl AudioProcessor for LatencyProcessor {
        fn name(&self) -> String {
            "Latency".to_string()
        }
        fn process(&mut self, _buffer: &mut AudioBuffer, _context: &ProcessingContext) {}
        fn latency_samples(&self) -> usize {
            self.latency
        }
        fn id(&self) -> Uuid {
            self.id
        }
        fn clone_box(&self) -> Box<dyn AudioProcessor> {
            Box::new(LatencyProcessor {
                latency: self.latency,
                id: self.id,
            })
        }
    }

    #[test]
    fn test_pdc_calculation() {
        let mut track1 = Track::new("Track 1".to_string());
        let track2 = Track::new("Track 2".to_string());

        // Track 1 has 100 samples latency
        track1.processors.push(Box::new(LatencyProcessor {
            latency: 100,
            id: Uuid::new_v4(),
        }));

        // Track 2 has 0 samples latency

        let mut tracks = vec![track1, track2];
        PdcManager::recalculate_project_pdc(&mut tracks);

        // Track 1 should have 0 compensation (it's the bottleneck)
        assert_eq!(tracks[0].pdc_delay_samples, 0);
        // Track 2 should have 100 samples compensation
        assert_eq!(tracks[1].pdc_delay_samples, 100);
        // Track 2's buffer should be initialized
        assert!(!tracks[1].pdc_delay_buffer.is_empty());
        assert!(tracks[1].pdc_delay_buffer[0].len() >= 100);
    }

    #[test]
    fn test_pdc_compensation_logic() {
        let mut delay_buffer = vec![vec![0.0; 1000]; 2];
        let mut output_l = vec![1.0, 2.0, 3.0, 4.0];
        let mut output_r = vec![1.0, 2.0, 3.0, 4.0];

        {
            let mut chans = [output_l.as_mut_slice(), output_r.as_mut_slice()];
            // Block 1: Input [1,2,3,4]. Delay 2.
            let mut write_idx = 0;
            PdcManager::apply_compensation(&mut delay_buffer, &mut chans, 2, &mut write_idx, 4);
        }

        // Output should be [0,0,1,2]
        assert_eq!(output_l, vec![0.0, 0.0, 1.0, 2.0]);

        // Block 2: Input [5,6,7,8]
        output_l = vec![5.0, 6.0, 7.0, 8.0];
        output_r = vec![5.0, 6.0, 7.0, 8.0];

        {
            let mut chans = [output_l.as_mut_slice(), output_r.as_mut_slice()];
            let mut write_idx = 4; // Follow from previous block
            PdcManager::apply_compensation(&mut delay_buffer, &mut chans, 2, &mut write_idx, 4);
        }

        assert_eq!(output_l, vec![3.0, 4.0, 5.0, 6.0]);
    }

    #[test]
    fn test_pdc_stress_test() {
        let mut tracks = Vec::new();
        // create 100 tracks with random latencies
        for i in 0..100 {
            let mut track = Track::new(format!("Track {}", i));
            track.processors.push(Box::new(LatencyProcessor {
                latency: i * 10,
                id: Uuid::new_v4(),
            }));
            tracks.push(track);
        }

        let start = std::time::Instant::now();
        PdcManager::recalculate_project_pdc(&mut tracks);
        let duration = start.elapsed();

        println!("PDC Recalculation for 100 tracks took {:?}", duration);

        // Max latency should be 990 (100th track index 99 * 10)
        // track 0 should have 990 compensation
        // track 99 should have 0 compensation
        assert_eq!(tracks[0].pdc_delay_samples, 990);
        assert_eq!(tracks[99].pdc_delay_samples, 0);

        // Verification of buffers
        assert!(tracks[0].pdc_delay_buffer[0].len() >= 990);
    }
}
