#![allow(dead_code)]
//! Optimized parallel processing for audio tracks with CPU affinity and work-stealing.

#[allow(unused_imports)]
use rayon::prelude::*;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// Performance statistics for parallel processing
#[derive(Debug, Clone, Default)]
pub struct ParallelStats {
    /// Total tracks processed
    pub tracks_processed: Arc<AtomicU64>,
    /// Total samples processed
    pub samples_processed: Arc<AtomicU64>,
    /// Number of parallel batches
    pub batches_executed: Arc<AtomicU64>,
}

impl ParallelStats {
    pub fn new() -> Self {
        Self {
            tracks_processed: Arc::new(AtomicU64::new(0)),
            samples_processed: Arc::new(AtomicU64::new(0)),
            batches_executed: Arc::new(AtomicU64::new(0)),
        }
    }

    pub fn record_batch(&self, track_count: usize, sample_count: usize) {
        self.tracks_processed
            .fetch_add(track_count as u64, Ordering::Relaxed);
        self.samples_processed
            .fetch_add(sample_count as u64, Ordering::Relaxed);
        self.batches_executed.fetch_add(1, Ordering::Relaxed);
    }

    pub fn get_stats(&self) -> (u64, u64, u64) {
        (
            self.tracks_processed.load(Ordering::Relaxed),
            self.samples_processed.load(Ordering::Relaxed),
            self.batches_executed.load(Ordering::Relaxed),
        )
    }
}

/// Optimized parallel processing configuration
pub struct ParallelConfig {
    /// Minimum tracks to enable parallel processing
    pub min_tracks_for_parallel: usize,
    /// Chunk size for work-stealing
    pub chunk_size: usize,
    /// Enable CPU affinity (Windows-specific)
    pub enable_cpu_affinity: bool,
}

impl Default for ParallelConfig {
    fn default() -> Self {
        Self {
            min_tracks_for_parallel: 4, // Only parallelize if we have 4+ tracks
            chunk_size: 2,              // Process 2 tracks per work unit
            enable_cpu_affinity: cfg!(target_os = "windows"),
        }
    }
}

/// Set CPU affinity for the current thread (Windows-specific optimization)
#[cfg(target_os = "windows")]
pub fn set_audio_thread_affinity() -> Result<(), String> {
    use windows_sys::Win32::System::Threading::{GetCurrentThread, SetThreadAffinityMask};

    unsafe {
        let handle = GetCurrentThread();
        // Pin to performance cores (typically cores 0-3 on hybrid architectures)
        // Affinity mask: 0b1111 = cores 0,1,2,3
        let affinity_mask = 0b1111usize;

        let result = SetThreadAffinityMask(handle, affinity_mask);
        if result == 0 {
            return Err("Failed to set thread affinity".to_string());
        }
    }

    Ok(())
}

#[cfg(not(target_os = "windows"))]
pub fn set_audio_thread_affinity() -> Result<(), String> {
    // No-op on non-Windows platforms
    Ok(())
}

/// Set thread priority to real-time (Windows-specific)
#[cfg(target_os = "windows")]
pub fn set_realtime_priority() -> Result<(), String> {
    use windows_sys::Win32::System::Threading::{
        GetCurrentThread, SetThreadPriority, THREAD_PRIORITY_TIME_CRITICAL,
    };

    unsafe {
        let handle = GetCurrentThread();
        let result = SetThreadPriority(handle, THREAD_PRIORITY_TIME_CRITICAL);
        if result == 0 {
            return Err("Failed to set thread priority".to_string());
        }
    }

    Ok(())
}

#[cfg(not(target_os = "windows"))]
pub fn set_realtime_priority() -> Result<(), String> {
    // Use thread-priority crate for cross-platform support
    use thread_priority::{set_current_thread_priority, ThreadPriority};

    set_current_thread_priority(ThreadPriority::Max)
        .map_err(|e| format!("Failed to set thread priority: {}", e))
}

/// Initialize the Rayon thread pool with optimized settings for audio processing
pub fn init_audio_thread_pool(num_threads: Option<usize>) -> Result<(), String> {
    let num_cpus = num_threads.unwrap_or_else(|| {
        std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4)
    });

    rayon::ThreadPoolBuilder::new()
        .num_threads(num_cpus)
        .thread_name(|i| format!("vibe-audio-{}", i))
        .spawn_handler(|thread| {
            let builder =
                std::thread::Builder::new().name(thread.name().unwrap_or("vibe-audio").to_string());

            builder.spawn(|| {
                // Set real-time priority for audio threads
                let _ = set_realtime_priority();

                // Set CPU affinity if enabled
                #[cfg(target_os = "windows")]
                let _ = set_audio_thread_affinity();

                thread.run()
            })?;

            Ok(())
        })
        .build_global()
        .map_err(|e| format!("Failed to build thread pool: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parallel_stats() {
        let stats = ParallelStats::new();

        stats.record_batch(8, 512);
        stats.record_batch(8, 512);

        let (tracks, samples, batches) = stats.get_stats();
        assert_eq!(tracks, 16);
        assert_eq!(samples, 1024);
        assert_eq!(batches, 2);
    }

    #[test]
    fn test_parallel_config_default() {
        let config = ParallelConfig::default();
        assert_eq!(config.min_tracks_for_parallel, 4);
        assert_eq!(config.chunk_size, 2);
    }

    #[test]
    fn test_thread_priority() {
        // This test just verifies the function doesn't panic
        let result = set_realtime_priority();
        // May fail if not running with sufficient privileges, that's OK
        let _ = result;
    }
}
