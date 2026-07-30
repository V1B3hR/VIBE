use crate::engine::streamer::pool::{AudioBlock, GlobalBufferPool, BLOCK_SIZE_SAMPLES};
use crate::engine::streamer::windows_streamer::{StreamRequest, WindowsAsyncStreamer};
use crossbeam_channel::{bounded, Receiver, Sender};
use std::os::windows::io::AsRawHandle;
use std::sync::Arc;
use uuid::Uuid;
use winapi::um::winnt::HANDLE;

#[allow(dead_code)]
pub struct HyperStreamReader {
    pub clip_id: Uuid,
    pub head_data: Arc<Vec<f32>>,
    pub head_samples: u64,
    pub total_samples: u64,
    pub file: Arc<std::fs::File>,
    pub clip_sample_rate: u32,
    pub offset_samples: u64,

    // Streaming state
    pub current_block: Option<Box<AudioBlock>>,
    pub next_block_rx: Receiver<Box<AudioBlock>>,
    pub next_block_tx: Sender<Box<AudioBlock>>,
    pub next_block_pending: bool,
    pub block_start_sample: u64,
    pub offline: bool,
}

impl HyperStreamReader {
    pub fn new(
        clip_id: Uuid,
        head_data: Arc<Vec<f32>>,
        total_samples: u64,
        file: Arc<std::fs::File>,
        clip_sample_rate: u32,
        offset_samples: u64,
        offline: bool,
    ) -> Self {
        let (tx, rx) = bounded(1);
        Self {
            clip_id,
            head_samples: head_data.len() as u64 / 2, // Stereo
            head_data,
            total_samples,
            file,
            clip_sample_rate,
            offset_samples,
            current_block: None,
            next_block_rx: rx,
            next_block_tx: tx,
            next_block_pending: false,
            block_start_sample: 0,
            offline,
        }
    }

    /// Retrieve stereo samples for the current process bufffer.
    /// Writes directly to the provided output buffers to avoid allocation.
    pub fn read_samples(
        &mut self,
        start_sample: u64,
        out_l: &mut [f32],
        out_r: &mut [f32],
        pool: &GlobalBufferPool,
        streamer: &WindowsAsyncStreamer,
    ) {
        let num_samples = out_l.len().min(out_r.len());

        for i in 0..num_samples {
            let s = start_sample + i as u64;
            let p = s + self.offset_samples;

            if p < self.head_samples {
                // READ FROM RAM HEAD
                let idx = (p * 2) as usize;
                if idx + 1 < self.head_data.len() {
                    out_l[i] = self.head_data[idx];
                    out_r[i] = self.head_data[idx + 1];
                }
            } else if s < self.total_samples {
                // READ FROM DISK TAIL (via Blocks)

                // 1. Rotation Logic: If we surpassed the current block, rotate.
                if self.current_block.is_none()
                    || s >= self.block_start_sample + (BLOCK_SIZE_SAMPLES as u64 / 2)
                {
                    let result = if self.offline {
                        self.next_block_rx.recv().map_err(|_| ())
                    } else {
                        self.next_block_rx.try_recv().map_err(|_| ())
                    };

                    if let Ok(next) = result {
                        if let Some(old) = self.current_block.take() {
                            pool.release(old);
                        }
                        self.current_block = Some(next);
                        // Start of the new block on the clip timeline
                        if self.block_start_sample == 0 {
                            // First block starts where head ends?
                            // Or where we are now?
                            // Logic: request_next_block decides offset.
                            // We need block_start_sample to match logical s.
                            // If we just loaded a block, it corresponds to what we requested.
                            // Simplification: align to block size if possible?
                            // But we just increment by block size usually.

                            // If block_start_sample is 0, we set it to (p aligned?)
                            // Let's assume sequential:
                            if self.block_start_sample == 0 {
                                // This is tricky. Let's assume request logic set next pending offset correct
                                // relative to start?
                                // Let's set it to 's' roughly?
                                // Better: reader assumes linear.
                                self.block_start_sample =
                                    self.head_samples.saturating_sub(self.offset_samples);
                                // If offset > head, then start at 0?
                                // Ideally we need to track what 'next_block' contains.
                                // For now, simple linear increment:
                                // If offset > head:
                            }
                            // Actually, let's keep it simple:
                            self.block_start_sample += BLOCK_SIZE_SAMPLES as u64 / 2;
                        } else {
                            self.block_start_sample += BLOCK_SIZE_SAMPLES as u64 / 2;
                        }
                        self.next_block_pending = false;
                    }
                }

                // 2. Read from current block if available
                if let Some(ref block) = self.current_block {
                    // Mapping p relative to file?
                    // Or s relative to block_start (logical)?
                    // block_start is logical.
                    let offset_in_block = (s.saturating_sub(self.block_start_sample)) as usize * 2;
                    if offset_in_block + 1 < BLOCK_SIZE_SAMPLES {
                        out_l[i] = block.data[offset_in_block];
                        out_r[i] = block.data[offset_in_block + 1];
                    }
                }

                // 3. Pre-fetch Logic
                if !self.next_block_pending
                    && s > self.block_start_sample + (BLOCK_SIZE_SAMPLES as u64 / 4)
                {
                    self.request_next_block(pool, streamer);
                }
            }
        }
    }

    fn request_next_block(&mut self, pool: &GlobalBufferPool, streamer: &WindowsAsyncStreamer) {
        if pool.available_blocks() > 0 {
            let block = pool.lease();
            let next_offset = if self.block_start_sample == 0 {
                // Start of disk reading (physical offset)
                // Head ends at head_samples.
                // File position = (head_samples + offset_samples) * 8
                (self.head_samples + self.offset_samples) * 8
            } else {
                // Next block is logical block_start + block_size
                // Physical = (logical + offset) * 8?
                // Wait, block_start_sample is logical.
                (self.block_start_sample + (BLOCK_SIZE_SAMPLES as u64 / 2) + self.offset_samples)
                    * 8
            };

            let req = StreamRequest {
                file_handle: self.file.as_raw_handle() as HANDLE,
                offset: next_offset,
                target_block: block,
                callback_tx: self.next_block_tx.clone(),
            };

            if streamer.read_at(req).is_ok() {
                self.next_block_pending = true;
            }
        }
    }
}
