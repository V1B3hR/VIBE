#![allow(dead_code)]

use cpal::traits::{DeviceTrait, HostTrait};
use serde::{Deserialize, Serialize};

/// Audio device configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioDeviceConfig {
    pub host_name: String,
    pub device_name: String,
    pub sample_rate: u32,
    pub buffer_size: u32,
    pub input_channels: usize,
    pub output_channels: usize,
}

impl Default for AudioDeviceConfig {
    fn default() -> Self {
        Self {
            host_name: "Default".to_string(),
            device_name: "Default".to_string(),
            sample_rate: 48000,
            buffer_size: 512,
            input_channels: 2,
            output_channels: 2,
        }
    }
}

/// Audio device information for UI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioDeviceInfo {
    pub id: String,
    pub name: String,
    pub host: String,
    pub is_default: bool,
    pub supported_sample_rates: Vec<u32>,
    pub max_input_channels: usize,
    pub max_output_channels: usize,
}

/// Audio device manager
pub struct AudioDeviceManager;

impl AudioDeviceManager {
    /// Get all available audio hosts (ASIO, WASAPI, DirectSound, etc.)
    pub fn get_available_hosts() -> Vec<String> {
        let mut hosts = Vec::new();

        #[cfg(target_os = "windows")]
        {
            // Try ASIO first (professional audio standard)
            #[cfg(feature = "asio")]
            {
                // ASIO will be available if ASIO4ALL or interface driver is installed
                hosts.push("ASIO".to_string());
            }

            // WASAPI (Windows default)
            hosts.push("WASAPI".to_string());
        }

        #[cfg(target_os = "macos")]
        {
            hosts.push("CoreAudio".to_string());
        }

        #[cfg(target_os = "linux")]
        {
            hosts.push("ALSA".to_string());
            hosts.push("JACK".to_string());
        }

        hosts
    }

    /// Get all available audio devices for a specific host
    pub fn get_devices_for_host(host_name: &str) -> Result<Vec<AudioDeviceInfo>, String> {
        let host_id = Self::host_name_to_id(host_name)?;
        let host = cpal::host_from_id(host_id).map_err(|e| format!("Failed to get host: {}", e))?;

        let mut devices = Vec::new();

        // Get output devices
        if let Ok(output_devices) = host.output_devices() {
            for device in output_devices {
                if let Ok(name) = device.name() {
                    let info = Self::get_device_info(&device, &name, host_name)?;
                    devices.push(info);
                }
            }
        }

        Ok(devices)
    }

    /// Get detailed information about a device
    fn get_device_info(
        device: &cpal::Device,
        name: &str,
        host_name: &str,
    ) -> Result<AudioDeviceInfo, String> {
        // Output Channels & Sample Rates
        let mut max_output_channels = 0;
        let mut supported_rates_set = std::collections::HashSet::new();

        if let Ok(configs) = device.supported_output_configs() {
            for cfg in configs {
                max_output_channels = max_output_channels.max(cfg.channels() as usize);
                // Simple heuristic: check standard rates against the range
                for &rate in &[44100, 48000, 88200, 96000, 192000] {
                    if rate >= cfg.min_sample_rate().0 && rate <= cfg.max_sample_rate().0 {
                        supported_rates_set.insert(rate);
                    }
                }
            }
        }

        // Input Channels
        let mut max_input_channels = 0;
        if let Ok(configs) = device.supported_input_configs() {
            for cfg in configs {
                max_input_channels = max_input_channels.max(cfg.channels() as usize);
                for &rate in &[44100, 48000, 88200, 96000, 192000] {
                    if rate >= cfg.min_sample_rate().0 && rate <= cfg.max_sample_rate().0 {
                        supported_rates_set.insert(rate);
                    }
                }
            }
        }

        let mut supported_sample_rates: Vec<u32> = supported_rates_set.into_iter().collect();
        supported_sample_rates.sort();

        Ok(AudioDeviceInfo {
            id: format!("{}::{}", host_name, name),
            name: name.to_string(),
            host: host_name.to_string(),
            is_default: false, // TODO: Check if default
            supported_sample_rates,
            max_input_channels,
            max_output_channels,
        })
    }

    /// Convert host name to cpal HostId
    fn host_name_to_id(host_name: &str) -> Result<cpal::HostId, String> {
        match host_name {
            #[cfg(all(target_os = "windows", feature = "asio"))]
            "ASIO" => Ok(cpal::HostId::Asio),
            #[cfg(target_os = "windows")]
            "WASAPI" => Ok(cpal::HostId::Wasapi),
            #[cfg(target_os = "macos")]
            "CoreAudio" => Ok(cpal::HostId::CoreAudio),
            #[cfg(target_os = "linux")]
            "ALSA" => Ok(cpal::HostId::Alsa),
            #[cfg(target_os = "linux")]
            "JACK" => Ok(cpal::HostId::Jack),
            _ => Ok(cpal::default_host().id()), // Fallback to default
        }
    }

    /// Find a device by name and host
    pub fn find_device(host_name: &str, device_name: &str) -> Result<cpal::Device, String> {
        let host_id = Self::host_name_to_id(host_name)?;
        let host = cpal::host_from_id(host_id).map_err(|e| e.to_string())?;
        
        if device_name == "Default" {
             return host.default_output_device().ok_or("No default output device".to_string());
        }

        let mut devices = host.devices().map_err(|e| e.to_string())?;
        devices.find(|d| d.name().ok() == Some(device_name.to_string()))
            .ok_or(format!("Device not found: {}", device_name))
    }

    /// Get recommended buffer sizes
    pub fn get_recommended_buffer_sizes() -> Vec<u32> {
        vec![64, 128, 256, 512, 1024, 2048]
    }

    /// Get recommended sample rates
    pub fn get_recommended_sample_rates() -> Vec<u32> {
        vec![44100, 48000, 88200, 96000, 192000]
    }
}
