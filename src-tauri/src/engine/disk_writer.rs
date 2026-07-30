use hound::{WavSpec, WavWriter};
use rtrb::{RingBuffer, Producer};
use std::fs::File;
use std::io::BufWriter;
use std::path::PathBuf;
use std::thread;

/// DiskWriter handles writing audio streams directly to disk on a background thread.
/// This prevents memory overflow during long recording sessions.
pub struct DiskWriter {
    command_tx: crossbeam_channel::Sender<DiskCommand>,
}

enum DiskCommand {
    StartRecording(PathBuf, WavSpec),
    StopRecording,
    Terminate,
}

impl DiskWriter {
    pub fn new(buffer_size: usize) -> (Self, Producer<f32>) {
        let (command_tx, command_rx) = crossbeam_channel::unbounded();
        let (producer, mut consumer) = RingBuffer::<f32>::new(buffer_size);

        thread::Builder::new()
            .name("vibe-disk-writer".to_string())
            .spawn(move || {
                let mut writer: Option<WavWriter<BufWriter<File>>> = None;

                loop {
                    // 1. Handle commands
                    while let Ok(cmd) = command_rx.try_recv() {
                        match cmd {
                            DiskCommand::StartRecording(path, spec) => {
                                println!("VIBE: Starting Disk Recording to {:?}", path);
                                match WavWriter::create(&path, spec) {
                                    Ok(w) => {
                                        writer = Some(w);
                                    }
                                    Err(e) => eprintln!("VIBE: Failed to create WAV writer: {}", e),
                                }
                            }
                            DiskCommand::StopRecording => {
                                if let Some(w) = writer.take() {
                                    println!("VIBE: Stopping Disk Recording.");
                                    let _ = w.finalize();
                                }
                            }
                            DiskCommand::Terminate => return,
                        }
                    }

                    // 2. Process audio data
                    if let Some(ref mut w) = writer {
                        if let Ok(chunk) = consumer.read_chunk(4096) {
                            let (s1, s2) = chunk.as_slices();
                            for &sample in s1 {
                                    let _ = w.write_sample(sample);
                            }
                            for &sample in s2 {
                                    let _ = w.write_sample(sample);
                            }
                            chunk.commit_all();
                        } else {
                            // Nothing to read, sleep briefly
                            thread::sleep(std::time::Duration::from_millis(5));
                        }
                    } else {
                        // Not recording, clear the buffer if anything is left (shouldn't be much)
                        while let Ok(_) = consumer.pop() {}
                        thread::sleep(std::time::Duration::from_millis(50));
                    }
                }
            })
            .expect("Failed to spawn DiskWriter thread");

        (
            Self {
                command_tx,
            },
            producer,
        )
    }

    pub fn start_recording(&self, path: PathBuf, sample_rate: u32, channels: u16) {
        let spec = WavSpec {
            channels,
            sample_rate,
            bits_per_sample: 32,
            sample_format: hound::SampleFormat::Float,
        };
        let _ = self.command_tx.send(DiskCommand::StartRecording(path, spec));
    }

    pub fn stop_recording(&self) {
        let _ = self.command_tx.send(DiskCommand::StopRecording);
    }
}

impl Drop for DiskWriter {
    fn drop(&mut self) {
        let _ = self.command_tx.send(DiskCommand::Terminate);
    }
}
