use crossbeam_channel::{bounded, Receiver, Sender};
use std::sync::Arc;

/// A fixed-size block of audio memory (256KB = 64k f32 samples).
/// This aligns well with SSD page sizes and provides enough lookahead.
pub const BLOCK_SIZE_SAMPLES: usize = 65536;
pub const BLOCK_SIZE_BYTES: usize = BLOCK_SIZE_SAMPLES * 4;

pub struct AudioBlock {
    pub data: Vec<f32>,
}

impl AudioBlock {
    pub fn new() -> Self {
        Self {
            data: vec![0.0; BLOCK_SIZE_SAMPLES],
        }
    }
}

/// Global Buffer Pool for Zero-Allocation Streaming.
/// Pre-allocates a pool of audio blocks to be recycled between I/O and Audio threads.
pub struct GlobalBufferPool {
    free_blocks_rx: Receiver<Box<AudioBlock>>,
    free_blocks_tx: Sender<Box<AudioBlock>>,
    #[allow(dead_code)]
    total_blocks: usize,
}

impl GlobalBufferPool {
    /// Create a new pool with the specified total memory size (in MB).
    pub fn new(total_memory_mb: usize) -> Arc<Self> {
        let block_bytes = BLOCK_SIZE_BYTES;
        let num_blocks = (total_memory_mb * 1024 * 1024) / block_bytes;

        let (tx, rx) = bounded(num_blocks);

        for _ in 0..num_blocks {
            tx.send(Box::new(AudioBlock::new())).unwrap();
        }

        Arc::new(Self {
            free_blocks_rx: rx,
            free_blocks_tx: tx,
            total_blocks: num_blocks,
        })
    }

    /// Lease a block from the pool. Blocks if none are available.
    pub fn lease(&self) -> Box<AudioBlock> {
        self.free_blocks_rx
            .recv()
            .expect("Buffer pool exhausted or closed")
    }

    /// Return a block to the pool.
    pub fn release(&self, block: Box<AudioBlock>) {
        let _ = self.free_blocks_tx.send(block);
    }

    pub fn available_blocks(&self) -> usize {
        self.free_blocks_rx.len()
    }

    #[allow(dead_code)]
    pub fn total_blocks(&self) -> usize {
        self.total_blocks
    }
}
