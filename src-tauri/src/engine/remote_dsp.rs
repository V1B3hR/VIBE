#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// UDP/TCP Network Packet payload for remote DSP plugin offloading
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DspNetworkPacket {
    pub session_id: String,
    pub plugin_id: String,
    pub frame_index: u64,
    pub sample_rate: u32,
    pub channels: u16,
    pub block_size: u32,
    /// Interleaved PCM f32 samples
    pub pcm_data: Vec<f32>,
    /// Parameter change dictionary
    pub param_changes: HashMap<u32, f32>,
}

/// Remote DSP Offload Node Descriptor
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteDspNode {
    pub node_id: String,
    pub ip_address: String,
    pub port: u16,
    pub cpu_cores: u32,
    pub cpu_usage_pct: f32,
    pub network_latency_ms: f32,
    pub active_offloaded_plugins: usize,
    pub is_online: bool,
}

/// Client & Discovery Manager for Remote DSP Offloading across LAN
pub struct RemoteDspManager {
    nodes: HashMap<String, RemoteDspNode>,
    active_offloads: HashMap<String, String>, // plugin_id -> node_id
    total_offloaded_frames: u64,
}

impl RemoteDspManager {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            active_offloads: HashMap::new(),
            total_offloaded_frames: 0,
        }
    }

    /// Register or update a discovered LAN DSP node
    pub fn register_node(&mut self, node: RemoteDspNode) {
        self.nodes.insert(node.node_id.clone(), node);
    }

    /// Select the best available node for offloading based on CPU load and network latency
    pub fn select_best_node(&self) -> Option<RemoteDspNode> {
        self.nodes
            .values()
            .filter(|n| n.is_online && n.cpu_usage_pct < 80.0 && n.network_latency_ms < 10.0)
            .min_by(|a, b| {
                let score_a = a.cpu_usage_pct * 0.6 + a.network_latency_ms * 4.0;
                let score_b = b.cpu_usage_pct * 0.6 + b.network_latency_ms * 4.0;
                score_a.partial_cmp(&score_b).unwrap_or(std::cmp::Ordering::Equal)
            })
            .cloned()
    }

    /// Offload a plugin instance to a remote node
    pub fn assign_offload(&mut self, plugin_id: &str, node_id: &str) -> Result<(), String> {
        if !self.nodes.contains_key(node_id) {
            return Err(format!("Node {} not found", node_id));
        }
        self.active_offloads.insert(plugin_id.to_string(), node_id.to_string());
        if let Some(node) = self.nodes.get_mut(node_id) {
            node.active_offloaded_plugins += 1;
        }
        Ok(())
    }

    /// Unassign offload and return plugin execution to local engine
    pub fn remove_offload(&mut self, plugin_id: &str) {
        if let Some(node_id) = self.active_offloads.remove(plugin_id) {
            if let Some(node) = self.nodes.get_mut(&node_id) {
                if node.active_offloaded_plugins > 0 {
                    node.active_offloaded_plugins -= 1;
                }
            }
        }
    }

    /// Prepare a serialized network payload for offloaded processing
    pub fn prepare_packet(
        &mut self,
        session_id: &str,
        plugin_id: &str,
        samples: &[f32],
        channels: u16,
        sample_rate: u32,
        param_changes: HashMap<u32, f32>,
    ) -> DspNetworkPacket {
        self.total_offloaded_frames += 1;
        DspNetworkPacket {
            session_id: session_id.to_string(),
            plugin_id: plugin_id.to_string(),
            frame_index: self.total_offloaded_frames,
            sample_rate,
            channels,
            block_size: (samples.len() / channels as usize) as u32,
            pcm_data: samples.to_vec(),
            param_changes,
        }
    }

    /// Check if a plugin is currently offloaded
    pub fn is_offloaded(&self, plugin_id: &str) -> bool {
        self.active_offloads.contains_key(plugin_id)
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_remote_node_registration_and_selection() {
        let mut mgr = RemoteDspManager::new();

        let node1 = RemoteDspNode {
            node_id: "node_heavy".to_string(),
            ip_address: "192.168.1.50".to_string(),
            port: 9090,
            cpu_cores: 16,
            cpu_usage_pct: 75.0,
            network_latency_ms: 2.5,
            active_offloaded_plugins: 3,
            is_online: true,
        };

        let node2 = RemoteDspNode {
            node_id: "node_fast".to_string(),
            ip_address: "192.168.1.51".to_string(),
            port: 9090,
            cpu_cores: 32,
            cpu_usage_pct: 15.0,
            network_latency_ms: 1.2,
            active_offloaded_plugins: 0,
            is_online: true,
        };

        mgr.register_node(node1);
        mgr.register_node(node2);

        let best = mgr.select_best_node().expect("Should select node");
        assert_eq!(best.node_id, "node_fast");
    }

    #[test]
    fn test_offload_assignment_lifecycle() {
        let mut mgr = RemoteDspManager::new();
        let node = RemoteDspNode {
            node_id: "node_1".to_string(),
            ip_address: "127.0.0.1".to_string(),
            port: 9090,
            cpu_cores: 8,
            cpu_usage_pct: 20.0,
            network_latency_ms: 0.5,
            active_offloaded_plugins: 0,
            is_online: true,
        };
        mgr.register_node(node);

        assert!(mgr.assign_offload("reverb_vst3", "node_1").is_ok());
        assert!(mgr.is_offloaded("reverb_vst3"));

        mgr.remove_offload("reverb_vst3");
        assert!(!mgr.is_offloaded("reverb_vst3"));
    }
}
