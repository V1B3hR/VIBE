use petgraph::graph::DiGraph;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// The core graph structure driving the audio engine signal flow.
pub type AudioGraph = DiGraph<GraphNode, GraphEdge>;

/// A node in the audio graph.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GraphNode {
    pub id: Uuid,
    pub name: String,
    pub kind: NodeKind,
    /// Gain in dB applied at this node's output
    pub gain_db: f64,
    /// Pan (-1.0 to 1.0)
    pub pan: f64,
    pub is_muted: bool,
    pub is_solo: bool,
    /// Latency introduced by this node (for PDC)
    pub latency_samples: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum NodeKind {
    /// Audio Interface Input (Hardware)
    Input { channel_index: usize },
    /// A sequenced Audio/MIDI Track
    Track { track_id: Uuid },
    /// A Submix Group / Bus
    Group { group_id: Uuid },
    /// An Aux Send bus (often post-fader)
    Aux { aux_id: Uuid },
    /// An Effect Return
    Return { return_id: Uuid },
    /// Master Output Bus
    Master,
    /// A specific Plugin instance (if we route node-to-node per plugin)
    Plugin { plugin_id: Uuid },
    /// Sidechain Source (tap point)
    SidechainSource { source_id: Uuid },
}

/// A connection between two nodes.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GraphEdge {
    /// Source channel (0=L, 1=R)
    pub from_port: u32,
    /// Destination channel (0=L, 1=R)
    pub to_port: u32,
    /// Gain applied to this specific connection (dB)
    pub gain_db: f64,
    /// Signal type (Audio or Sidechain Control)
    pub signal_type: SignalType,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum SignalType {
    Audio { pre_fader: bool },
    Sidechain,
}

/// Helper to manage reusable buffers for zero-allocation processing.
pub struct BufferPool {
    /// Available buffers (each is a vector of samples)
    free_buffers: Vec<Vec<f64>>,
    /// Standard size for buffers
    #[allow(dead_code)]
    buffer_size: usize,
}

impl BufferPool {
    pub fn new(buffer_size: usize, initial_capacity: usize) -> Self {
        let mut pool = Self {
            free_buffers: Vec::with_capacity(initial_capacity),
            buffer_size,
        };
        for _ in 0..initial_capacity {
            pool.free_buffers.push(vec![0.0; buffer_size]);
        }
        pool
    }

    /// specific resizing if buffer size changes (e.g. sample rate change)
    #[allow(dead_code)]
    pub fn resize(&mut self, new_size: usize) {
        self.buffer_size = new_size;
        for buf in &mut self.free_buffers {
            buf.resize(new_size, 0.0);
        }
    }

    #[allow(dead_code)]
    pub fn acquire(&mut self) -> Vec<f64> {
        if let Some(mut buf) = self.free_buffers.pop() {
            // Ensure size is correct (in case it was returned from somewhere else,
            // though we should strictly manage this)
            if buf.len() != self.buffer_size {
                buf.resize(self.buffer_size, 0.0);
            } else {
                // Determine if we need to clear. Usually the consumer clears.
                // But for safety let's fill 0.0
                buf.fill(0.0);
            }
            buf
        } else {
            // Allocate new if empty (should be rare if sized correctly)
            vec![0.0; self.buffer_size]
        }
    }

    #[allow(dead_code)]
    pub fn release(&mut self, buffer: Vec<f64>) {
        self.free_buffers.push(buffer);
    }
}

/// Execution context for the audio graph
#[allow(dead_code)]
pub struct GraphExecutor {
    sorting_cache: Vec<petgraph::graph::NodeIndex>,
    dirty: bool,
}

#[allow(dead_code)]
impl GraphExecutor {
    pub fn new() -> Self {
        Self {
            sorting_cache: Vec::new(),
            dirty: true,
        }
    }

    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    /// Primary Processing Entry Point
    /// uses 2 buffers per edge? No, usually nodes have buffers.
    /// In a zero-copy system, we pass buffer references.
    /// Simplified for VIBE Architecture:
    /// 1. Sort Nodes
    /// 2. For each node/track:
    ///    a. Sum inputs from dependencies (Parents in graph)
    ///    b. Process
    ///    c. Expose output buffer
    pub fn execute(
        &mut self,
        graph: &mut AudioGraph,
        _tracks: &mut std::collections::HashMap<Uuid, &mut super::graph::Track>,
    ) -> Result<(), String> {
        // 1. Topo Sort (Cached)
        if self.dirty {
            match petgraph::algo::toposort(&*graph, None) {
                Ok(sorted) => {
                    self.sorting_cache = sorted;
                    self.dirty = false;
                }
                Err(_) => return Err("Graph Cycle Detected".to_string()),
            }
        }

        // 2. Traverse
        // We need a way to map GraphNode -> Track/Bus Processing
        // And manage buffers between them.

        // This acts as a proof-of-concept executor.
        // In the real engine, this loop replaces "tracks.par_iter_mut()"

        // Mock execution for validation
        for _node_idx in &self.sorting_cache {
            // let node = &graph[*node_idx];
            // Match node.kind -> Lookup Track -> Process
        }

        Ok(())
    }
}
