#![allow(dead_code)]
//! Lock-free parameter updates for real-time audio processing.
//!
//! This module provides wait-free parameter updates that can be safely
//! used from both the UI thread and the real-time audio thread without
//! blocking or allocating memory.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// Lock-free parameter value using atomic operations.
/// Stores f64 as u64 bits for atomic access.
#[derive(Debug)]
pub struct AtomicParameter {
    /// Current value stored as u64 bits
    value_bits: AtomicU64,
    /// Minimum allowed value
    min_value: f64,
    /// Maximum allowed value
    max_value: f64,
}

impl AtomicParameter {
    /// Create a new atomic parameter with the given initial value and range
    pub fn new(initial_value: f64, min_value: f64, max_value: f64) -> Self {
        let clamped = initial_value.clamp(min_value, max_value);
        Self {
            value_bits: AtomicU64::new(clamped.to_bits()),
            min_value,
            max_value,
        }
    }

    /// Get the current value (wait-free read)
    #[inline(always)]
    pub fn get(&self) -> f64 {
        f64::from_bits(self.value_bits.load(Ordering::Relaxed))
    }

    /// Set a new value (wait-free write)
    /// The value will be clamped to [min_value, max_value]
    #[inline(always)]
    pub fn set(&self, value: f64) {
        let clamped = value.clamp(self.min_value, self.max_value);
        self.value_bits.store(clamped.to_bits(), Ordering::Relaxed);
    }

    /// Atomically add a delta to the current value
    /// Returns the new value after the operation
    pub fn add(&self, delta: f64) -> f64 {
        loop {
            let current_bits = self.value_bits.load(Ordering::Relaxed);
            let current = f64::from_bits(current_bits);
            let new_value = (current + delta).clamp(self.min_value, self.max_value);
            let new_bits = new_value.to_bits();

            // Try to swap the value
            match self.value_bits.compare_exchange_weak(
                current_bits,
                new_bits,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => return new_value,
                Err(_) => continue, // Retry if another thread modified it
            }
        }
    }

    /// Get the parameter range
    pub fn range(&self) -> (f64, f64) {
        (self.min_value, self.max_value)
    }
}

/// Collection of lock-free parameters for a track or effect
#[derive(Debug)]
pub struct ParameterBank {
    /// Named parameters
    parameters: Vec<(String, Arc<AtomicParameter>)>,
}

impl ParameterBank {
    /// Create a new empty parameter bank
    pub fn new() -> Self {
        Self {
            parameters: Vec::new(),
        }
    }

    /// Add a parameter to the bank
    pub fn add_parameter(
        &mut self,
        name: String,
        initial_value: f64,
        min_value: f64,
        max_value: f64,
    ) -> Arc<AtomicParameter> {
        let param = Arc::new(AtomicParameter::new(initial_value, min_value, max_value));
        self.parameters.push((name, param.clone()));
        param
    }

    /// Get a parameter by name
    pub fn get_parameter(&self, name: &str) -> Option<Arc<AtomicParameter>> {
        self.parameters
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, p)| p.clone())
    }

    /// Get all parameter names
    pub fn parameter_names(&self) -> Vec<String> {
        self.parameters.iter().map(|(n, _)| n.clone()).collect()
    }

    /// Get the number of parameters
    pub fn len(&self) -> usize {
        self.parameters.len()
    }

    /// Check if the bank is empty
    pub fn is_empty(&self) -> bool {
        self.parameters.is_empty()
    }
}

impl Default for ParameterBank {
    fn default() -> Self {
        Self::new()
    }
}

/// Smoothed parameter that interpolates changes over time
/// to avoid zipper noise in audio processing
#[derive(Debug)]
pub struct SmoothedParameter {
    /// Target value
    target: Arc<AtomicParameter>,
    /// Current smoothed value
    current: f64,
    /// Smoothing coefficient (0.0 = instant, 1.0 = never changes)
    smoothing: f64,
}

impl SmoothedParameter {
    /// Create a new smoothed parameter
    ///
    /// # Arguments
    /// * `initial_value` - Starting value
    /// * `min_value` - Minimum allowed value
    /// * `max_value` - Maximum allowed value
    /// * `smoothing_ms` - Smoothing time in milliseconds
    /// * `sample_rate` - Audio sample rate
    pub fn new(
        initial_value: f64,
        min_value: f64,
        max_value: f64,
        smoothing_ms: f64,
        sample_rate: f64,
    ) -> Self {
        // Calculate smoothing coefficient for exponential smoothing
        // tau = smoothing_ms / 1000.0 (convert to seconds)
        // coefficient = exp(-1.0 / (tau * sample_rate))
        let tau = smoothing_ms / 1000.0;
        let smoothing = (-1.0 / (tau * sample_rate)).exp();

        Self {
            target: Arc::new(AtomicParameter::new(initial_value, min_value, max_value)),
            current: initial_value,
            smoothing,
        }
    }

    /// Set the target value (will be smoothed over time)
    pub fn set_target(&self, value: f64) {
        self.target.set(value);
    }

    /// Get the current smoothed value and advance the smoother
    /// Call this once per audio sample
    #[inline(always)]
    pub fn next(&mut self) -> f64 {
        let target = self.target.get();
        // Exponential smoothing: current = current * smoothing + target * (1 - smoothing)
        self.current = self.current * self.smoothing + target * (1.0 - self.smoothing);
        self.current
    }

    /// Get the target parameter for external access
    pub fn target_parameter(&self) -> Arc<AtomicParameter> {
        self.target.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_atomic_parameter_basic() {
        let param = AtomicParameter::new(0.5, 0.0, 1.0);
        assert_eq!(param.get(), 0.5);

        param.set(0.75);
        assert_eq!(param.get(), 0.75);
    }

    #[test]
    fn test_atomic_parameter_clamping() {
        let param = AtomicParameter::new(0.5, 0.0, 1.0);

        param.set(1.5); // Should clamp to 1.0
        assert_eq!(param.get(), 1.0);

        param.set(-0.5); // Should clamp to 0.0
        assert_eq!(param.get(), 0.0);
    }

    #[test]
    fn test_atomic_parameter_add() {
        let param = AtomicParameter::new(0.5, 0.0, 1.0);

        let new_value = param.add(0.3);
        assert_eq!(new_value, 0.8);
        assert_eq!(param.get(), 0.8);

        // Test clamping on add
        param.add(0.5); // Should clamp to 1.0
        assert_eq!(param.get(), 1.0);
    }

    #[test]
    fn test_parameter_bank() {
        let mut bank = ParameterBank::new();

        let volume = bank.add_parameter("volume".to_string(), 0.8, 0.0, 1.0);
        let pan = bank.add_parameter("pan".to_string(), 0.5, 0.0, 1.0);

        assert_eq!(bank.len(), 2);
        assert_eq!(volume.get(), 0.8);
        assert_eq!(pan.get(), 0.5);

        let retrieved = bank.get_parameter("volume").unwrap();
        assert_eq!(retrieved.get(), 0.8);
    }

    #[test]
    fn test_smoothed_parameter() {
        let mut param = SmoothedParameter::new(0.0, 0.0, 1.0, 10.0, 44100.0);

        param.set_target(1.0);

        // After a few samples, should be moving towards target
        let first = param.next();
        let second = param.next();
        let third = param.next();

        assert!(first > 0.0 && first < 1.0);
        assert!(second > first);
        assert!(third > second);
    }
}
