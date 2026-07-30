#![allow(dead_code)]
use crossbeam_channel::{unbounded, Sender};
use memmap2::Mmap;
use rtrb::{Consumer, Producer, RingBuffer};
use std::fs::File;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::thread;

pub struct StreamingSource {
    pub path: String,
    pub mmap: Option<Arc<Mmap>>,
}

impl StreamingSource {
    pub fn new(path: &str) -> Result<Self, String> {
        let file = File::open(path).map_err(|e| e.to_string())?;
        let mmap = unsafe { Mmap::map(&file).ok().map(Arc::new) };
        Ok(Self {
            path: path.to_string(),
            mmap,
        })
    }
}

pub struct SmartClip {
    pub id: uuid::Uuid,
    pub head_buffer: Arc<Vec<f32>>, // First ~1s in RAM
    pub source: Arc<StreamingSource>,
    pub is_tail_cached: AtomicBool,
    pub sample_rate: u32,
    pub channels: u16,
    pub total_samples: u64,
}

impl SmartClip {
    pub fn new(path: &str, head: Vec<f32>, total_samples: u64) -> Result<Self, String> {
        let source = Arc::new(StreamingSource::new(path)?);
        Ok(Self {
            id: uuid::Uuid::new_v4(),
            head_buffer: Arc::new(head),
            source,
            is_tail_cached: AtomicBool::new(false),
            sample_rate: 44100,
            channels: 2,
            total_samples,
        })
    }
}

pub enum DiskTask {
    StreamTail {
        clip: Arc<SmartClip>,
        producer: Producer<f32>,
        start_pos: u64,
    },
}

pub struct VelocityEngine {
    task_tx: Sender<DiskTask>,
}

impl VelocityEngine {
    pub fn new() -> Self {
        let (tx, rx) = unbounded::<DiskTask>();

        thread::spawn(move || {
            while let Ok(task) = rx.recv() {
                match task {
                    DiskTask::StreamTail {
                        clip,
                        producer: _producer,
                        start_pos,
                    } => {
                        let samples_to_stream = clip.total_samples.saturating_sub(start_pos);
                        println!(
                            "VIBE: Velocity Engine streaming {} samples for clip {} from pos {}",
                            samples_to_stream, clip.id, start_pos
                        );
                        // Future: Symphonia batch decoding into producer
                    }
                }
            }
        });

        Self { task_tx: tx }
    }

    pub fn request_stream(&self, task: DiskTask) {
        let _ = self.task_tx.send(task);
    }
}

pub struct StreamingVoice {
    pub clip: Arc<SmartClip>,
    pub consumer: Consumer<f32>,
    pub current_sample: u64,
}

impl StreamingVoice {
    pub fn new(clip: Arc<SmartClip>, buffer_size: usize) -> (Self, Producer<f32>) {
        let (producer, consumer) = RingBuffer::new(buffer_size);
        (
            Self {
                clip,
                consumer,
                current_sample: 0,
            },
            producer,
        )
    }

    pub fn get_next_sample(&mut self) -> f32 {
        if self.current_sample < self.clip.head_buffer.len() as u64 {
            let s = self.clip.head_buffer[self.current_sample as usize];
            self.current_sample += 1;
            s
        } else {
            self.consumer.pop().unwrap_or(0.0)
        }
    }
}
