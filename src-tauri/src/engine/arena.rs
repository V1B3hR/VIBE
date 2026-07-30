#![allow(dead_code)]

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

const ARENA_SIZE: usize = 64 * 1024 * 1024; // 64MB pre-allocated pool
const BLOCK_SIZE: usize = 8192; // 8KB blocks

/// Lock-Free Arena Allocator for audio buffers
/// Pre-allocates a large chunk of memory and issues blocks atomically
pub struct LockFreeArena {
    memory: Vec<f64>,
    next_offset: Arc<AtomicUsize>,
    block_size: usize,
}

impl LockFreeArena {
    pub fn new() -> Self {
        let num_samples = ARENA_SIZE / std::mem::size_of::<f64>();
        Self {
            memory: vec![0.0; num_samples],
            next_offset: Arc::new(AtomicUsize::new(0)),
            block_size: BLOCK_SIZE / std::mem::size_of::<f64>(),
        }
    }

    /// Allocate a buffer of given size
    /// Returns None if arena is full
    pub fn alloc<'a>(&'a self, size: usize) -> Option<ArenaBuffer<'a>> {
        let aligned_size = (size + 7) & !7; // 8-byte alignment
        let old_offset = self.next_offset.fetch_add(aligned_size, Ordering::Relaxed);

        if old_offset + aligned_size > self.memory.len() {
            // Arena exhausted
            return None;
        }

        Some(ArenaBuffer {
            offset: old_offset,
            size: aligned_size,
            arena: self,
        })
    }

    /// Reset arena (call when no buffers are in use)
    pub fn reset(&self) {
        self.next_offset.store(0, Ordering::Release);
    }

    /// Get usage statistics
    pub fn usage(&self) -> (usize, usize) {
        let used = self.next_offset.load(Ordering::Acquire);
        (used, self.memory.len())
    }
}

/// Buffer allocated from LockFreeArena
pub struct ArenaBuffer<'a> {
    offset: usize,
    size: usize,
    arena: &'a LockFreeArena,
}

impl<'a> ArenaBuffer<'a> {
    /// Get mutable slice to buffer data
    pub fn as_mut_slice(&mut self) -> &mut [f64] {
        unsafe {
            let ptr = self.arena.memory.as_ptr().add(self.offset) as *mut f64;
            std::slice::from_raw_parts_mut(ptr, self.size)
        }
    }

    /// Get immutable slice to buffer data
    pub fn as_slice(&self) -> &[f64] {
        &self.arena.memory[self.offset..self.offset + self.size]
    }
}

/// Buffer pool for reusable audio buffers
pub struct BufferPool {
    arena: Arc<LockFreeArena>,
    buffer_size: usize,
}

impl BufferPool {
    pub fn new(buffer_size: usize) -> Self {
        Self {
            arena: Arc::new(LockFreeArena::new()),
            buffer_size,
        }
    }

    /// Get a buffer from the pool
    pub fn get<'a>(&'a self) -> Option<ArenaBuffer<'a>> {
        self.arena.alloc(self.buffer_size)
    }

    /// Reset pool (call between processing cycles)
    pub fn reset(&self) {
        self.arena.reset();
    }

    /// Get pool statistics
    pub fn stats(&self) -> (usize, usize, f64) {
        let (used, total) = self.arena.usage();
        let utilization = (used as f64 / total as f64) * 100.0;
        (used, total, utilization)
    }
}
