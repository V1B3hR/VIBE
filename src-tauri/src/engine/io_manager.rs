#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use uuid::Uuid;

/// Logical input alias - abstraction over physical hardware channels
/// This ensures projects remain portable across different audio interfaces
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputAlias {
    pub id: Uuid,
    pub name: String, // "Vocal Mic", "Kick In", "Stereo Keys"
    pub is_stereo: bool,
    pub hardware_channels: Vec<usize>, // [3] for mono, [7,8] for stereo
    pub color: String,                 // For UI grouping (hex color)
}

impl InputAlias {
    pub fn new(name: String, is_stereo: bool, channels: Vec<usize>, color: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            is_stereo,
            hardware_channels: channels,
            color,
        }
    }

    /// Validate that stereo channels are consecutive
    pub fn validate(&self) -> Result<(), String> {
        if self.is_stereo {
            if self.hardware_channels.len() != 2 {
                return Err("Stereo alias must have exactly 2 channels".to_string());
            }
            let ch1 = self.hardware_channels[0];
            let ch2 = self.hardware_channels[1];
            if ch2 != ch1 + 1 {
                return Err(format!(
                    "Stereo channels must be consecutive (got {} and {})",
                    ch1, ch2
                ));
            }
        } else if self.hardware_channels.len() != 1 {
            return Err("Mono alias must have exactly 1 channel".to_string());
        }
        Ok(())
    }
}

/// Output alias for future sends/returns routing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputAlias {
    pub id: Uuid,
    pub name: String,
    pub is_stereo: bool,
    pub hardware_channels: Vec<usize>,
    pub color: String,
}

/// I/O Manager - central routing hub
/// Manages the 3-layer routing: Hardware → Logical Alias → Track
pub struct IoManager {
    /// Currently active audio device
    active_device: Option<String>,

    /// Logical input aliases (saved in project)
    input_aliases: Arc<Mutex<Vec<InputAlias>>>,

    /// Output aliases for sends/returns (future feature)
    output_aliases: Arc<Mutex<Vec<OutputAlias>>>,

    /// Real-time RMS metering for all physical channels (64 max)
    /// Stored as atomic f32 bits for lock-free access from audio thread
    channel_meters: Arc<Vec<AtomicU64>>,

    /// Maximum number of hardware channels supported
    max_channels: usize,
}

impl IoManager {
    pub fn new(max_channels: usize) -> Self {
        // Pre-allocate atomic meters for all channels
        let mut meters = Vec::with_capacity(max_channels);
        for _ in 0..max_channels {
            meters.push(AtomicU64::new(0)); // 0.0f32 as bits
        }

        Self {
            active_device: None,
            input_aliases: Arc::new(Mutex::new(Vec::new())),
            output_aliases: Arc::new(Mutex::new(Vec::new())),
            channel_meters: Arc::new(meters),
            max_channels,
        }
    }

    // ============ InputAlias CRUD ============

    /// Create new input alias
    pub fn create_input_alias(
        &self,
        name: String,
        is_stereo: bool,
        channels: Vec<usize>,
        color: String,
    ) -> Result<Uuid, String> {
        let alias = InputAlias::new(name, is_stereo, channels, color);

        // Validate
        alias.validate()?;

        // Check channel bounds
        for &ch in &alias.hardware_channels {
            if ch >= self.max_channels {
                return Err(format!(
                    "Channel {} exceeds max channels ({})",
                    ch, self.max_channels
                ));
            }
        }

        let id = alias.id;
        self.input_aliases.lock().unwrap().push(alias);
        Ok(id)
    }

    /// Add a pre-constructed input alias (used when ID provides by caller)
    pub fn add_input_alias(&self, alias: InputAlias) -> Result<(), String> {
        // Validate
        alias.validate()?;

        // Check channel bounds
        for &ch in &alias.hardware_channels {
            if ch >= self.max_channels {
                return Err(format!(
                    "Channel {} exceeds max channels ({})",
                    ch, self.max_channels
                ));
            }
        }

        self.input_aliases.lock().unwrap().push(alias);
        Ok(())
    }

    /// Update existing input alias
    pub fn update_input_alias(
        &self,
        id: Uuid,
        name: String,
        channels: Vec<usize>,
        color: String,
    ) -> Result<(), String> {
        let mut aliases = self.input_aliases.lock().unwrap();

        let alias = aliases
            .iter_mut()
            .find(|a| a.id == id)
            .ok_or_else(|| format!("InputAlias {} not found", id))?;

        // Validate new channels
        let is_stereo = alias.is_stereo;
        let temp_alias = InputAlias::new(name.clone(), is_stereo, channels.clone(), color.clone());
        temp_alias.validate()?;

        // Update
        alias.name = name;
        alias.hardware_channels = channels;
        alias.color = color;

        Ok(())
    }

    /// Delete input alias
    pub fn delete_input_alias(&self, id: Uuid) -> Result<(), String> {
        let mut aliases = self.input_aliases.lock().unwrap();

        let index = aliases
            .iter()
            .position(|a| a.id == id)
            .ok_or_else(|| format!("InputAlias {} not found", id))?;

        aliases.remove(index);
        Ok(())
    }

    /// Get all input aliases
    pub fn get_all_input_aliases(&self) -> Vec<InputAlias> {
        self.input_aliases.lock().unwrap().clone()
    }

    /// Get input alias by ID
    pub fn get_input_alias(&self, id: Uuid) -> Option<InputAlias> {
        self.input_aliases
            .lock()
            .unwrap()
            .iter()
            .find(|a| a.id == id)
            .cloned()
    }

    // ============ Real-Time Metering ============

    /// Update RMS meter for a specific channel (called from audio thread)
    /// This is lock-free and safe to call from real-time context
    pub fn update_channel_meter(&self, channel: usize, rms: f32) {
        if channel < self.max_channels {
            let bits = rms.to_bits() as u64;
            self.channel_meters[channel].store(bits, Ordering::Relaxed);
        }
    }

    /// Get all channel meters (for frontend)
    pub fn get_channel_meters(&self) -> Vec<f32> {
        self.channel_meters
            .iter()
            .map(|meter| {
                let bits = meter.load(Ordering::Relaxed) as u32;
                f32::from_bits(bits)
            })
            .collect()
    }

    /// Get meter for specific channel
    pub fn get_channel_meter(&self, channel: usize) -> f32 {
        if channel < self.max_channels {
            let bits = self.channel_meters[channel].load(Ordering::Relaxed) as u32;
            f32::from_bits(bits)
        } else {
            0.0
        }
    }

    // ============ Device Management ============

    pub fn set_active_device(&mut self, device_name: String) {
        self.active_device = Some(device_name);
    }

    pub fn get_active_device(&self) -> Option<String> {
        self.active_device.clone()
    }

    pub fn get_max_channels(&self) -> usize {
        self.max_channels
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_mono_alias() {
        let io_manager = IoManager::new(64);
        let id = io_manager
            .create_input_alias(
                "Vocal Mic".to_string(),
                false,
                vec![3],
                "#FFD700".to_string(),
            )
            .expect("Failed to create alias");

        let aliases = io_manager.get_all_input_aliases();
        assert_eq!(aliases.len(), 1);
        assert_eq!(aliases[0].id, id);
        assert_eq!(aliases[0].name, "Vocal Mic");
        assert!(!aliases[0].is_stereo);
        assert_eq!(aliases[0].hardware_channels, vec![3]);
    }

    #[test]
    fn test_create_stereo_alias() {
        let io_manager = IoManager::new(64);
        let id = io_manager
            .create_input_alias(
                "Stereo Keys".to_string(),
                true,
                vec![7, 8],
                "#00FF00".to_string(),
            )
            .expect("Failed to create alias");

        let alias = io_manager.get_input_alias(id).unwrap();
        assert!(alias.is_stereo);
        assert_eq!(alias.hardware_channels, vec![7, 8]);
    }

    #[test]
    fn test_stereo_validation_consecutive() {
        let io_manager = IoManager::new(64);

        // Should fail - non-consecutive channels
        let result = io_manager.create_input_alias(
            "Bad Stereo".to_string(),
            true,
            vec![5, 7],
            "#FF0000".to_string(),
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("consecutive"));
    }

    #[test]
    fn test_channel_metering() {
        let io_manager = IoManager::new(64);

        // Update meters
        io_manager.update_channel_meter(0, 0.5);
        io_manager.update_channel_meter(1, 0.8);
        io_manager.update_channel_meter(2, 0.3);

        // Read meters
        let meters = io_manager.get_channel_meters();
        assert_eq!(meters[0], 0.5);
        assert_eq!(meters[1], 0.8);
        assert_eq!(meters[2], 0.3);
    }

    #[test]
    fn test_update_alias() {
        let io_manager = IoManager::new(64);
        let id = io_manager
            .create_input_alias(
                "Old Name".to_string(),
                false,
                vec![5],
                "#FFFFFF".to_string(),
            )
            .unwrap();

        io_manager
            .update_input_alias(id, "New Name".to_string(), vec![10], "#000000".to_string())
            .expect("Failed to update");

        let alias = io_manager.get_input_alias(id).unwrap();
        assert_eq!(alias.name, "New Name");
        assert_eq!(alias.hardware_channels, vec![10]);
        assert_eq!(alias.color, "#000000");
    }

    #[test]
    fn test_delete_alias() {
        let io_manager = IoManager::new(64);
        let id = io_manager
            .create_input_alias(
                "To Delete".to_string(),
                false,
                vec![1],
                "#FFFFFF".to_string(),
            )
            .unwrap();

        assert_eq!(io_manager.get_all_input_aliases().len(), 1);

        io_manager.delete_input_alias(id).expect("Failed to delete");

        assert_eq!(io_manager.get_all_input_aliases().len(), 0);
    }
}
