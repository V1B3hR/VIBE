use super::config::MelSpectrogramConfig;
use super::drum_detector::DrumDetector;
use super::onset_detector::OnsetDetector;
use super::processor::MelProcessor;
use super::types::MelFrame;
use crate::engine::audio_commands::MidiEvent;
use crossbeam_channel::{Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;

pub struct SpectralWorker {
    pub config: MelSpectrogramConfig,
}

impl SpectralWorker {
    pub fn new(config: MelSpectrogramConfig) -> Self {
        Self { config }
    }

    pub fn spawn(
        &self,
        audio_receiver: Receiver<Vec<f32>>,
        frame_sender: Sender<MelFrame>,
        midi_sender: Arc<Mutex<rtrb::Producer<MidiEvent>>>,
    ) -> thread::JoinHandle<()> {
        let processor = MelProcessor::new(self.config.clone());
        let mut onset_detector = OnsetDetector::new(5.0); // Threshold 5.0
        let mut drum_detector = DrumDetector::new();
        let sr = self.config.sample_rate as f64;

        let hop_size = self.config.hop_size;
        let fft_size = self.config.fft_size;

        thread::spawn(move || {
            let mut buffer = Vec::new();
            let mut total_samples_processed: u64 = 0;

            while let Ok(samples) = audio_receiver.recv() {
                buffer.extend_from_slice(&samples);

                while buffer.len() >= fft_size {
                    let frame_samples = &buffer[..fft_size];
                    let frame = processor.process_frame(frame_samples, total_samples_processed);

                    // Stage 2: Drum Triggering Analysis (Spectral Flux & Band Energy)
                    let flux = onset_detector.process_frame(&frame);
                    if onset_detector.is_onset(flux) {
                        let drum_events = drum_detector.process_frame(&frame, sr);
                        if !drum_events.is_empty() {
                            if let Ok(mut midi) = midi_sender.lock() {
                                for event in drum_events {
                                    let _ = midi.push(event);
                                }
                            }
                        }
                    }

                    if frame_sender.send(frame).is_err() {
                        return; // Receiver closed
                    }

                    // Shift buffer by hop_size
                    buffer.drain(..hop_size);
                    total_samples_processed += hop_size as u64;
                }
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossbeam_channel::unbounded;
    use rtrb::RingBuffer;

    #[test]
    fn test_spectral_worker_integration() {
        let config = MelSpectrogramConfig::default();
        let worker = SpectralWorker::new(config);

        let (audio_tx, audio_rx) = unbounded();
        let (frame_tx, frame_rx) = unbounded();
        let (midi_prod, _midi_cons) = RingBuffer::new(100);
        let midi_sender = Arc::new(Mutex::new(midi_prod));

        let handle = worker.spawn(audio_rx, frame_tx, midi_sender);

        // Send a burst of audio (enough for a few frames)
        let samples = vec![0.0; 2048];
        audio_tx.send(samples).unwrap();

        // Wait a bit and check for frames
        let mut frames_received = 0;
        for _ in 0..10 {
            if let Ok(_) = frame_rx.try_recv() {
                frames_received += 1;
            }
            thread::sleep(std::time::Duration::from_millis(10));
        }

        assert!(frames_received > 0);

        // Stop worker
        drop(audio_tx);
        handle.join().unwrap();
    }
}
