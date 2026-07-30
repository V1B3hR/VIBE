#![allow(dead_code)]

use super::audio_graph::{AudioGraph, GraphEdge, GraphNode};
use petgraph::algo::is_cyclic_directed;
use petgraph::graph::{EdgeIndex, NodeIndex};
use std::sync::{Arc, Mutex};
use uuid::Uuid;

#[derive(Debug)]
pub enum RoutingError {
    NodeNotFound,
    CycleDetected,
    InvalidPort,
    GraphLocked,
    EdgeNotFound,
}

impl std::fmt::Display for RoutingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for RoutingError {}

pub struct GraphManager {
    pub graph: Arc<Mutex<AudioGraph>>,
}

impl GraphManager {
    pub fn new(graph: Arc<Mutex<AudioGraph>>) -> Self {
        Self { graph }
    }

    pub fn add_node(&self, node: GraphNode) -> NodeIndex {
        let mut g = self.graph.lock().unwrap();
        g.add_node(node)
    }

    pub fn remove_node(&self, id: Uuid) -> Result<(), RoutingError> {
        let mut g = self.graph.lock().unwrap();
        let idx = g
            .node_indices()
            .find(|i| g[*i].id == id)
            .ok_or(RoutingError::NodeNotFound)?;
        g.remove_node(idx);
        Ok(())
    }

    pub fn connect(
        &self,
        from: NodeIndex,
        to: NodeIndex,
        edge: GraphEdge,
    ) -> Result<EdgeIndex, RoutingError> {
        let mut g = self.graph.lock().unwrap();

        // 1. Temporary add edge
        let edge_idx = g.add_edge(from, to, edge);

        // 2. Check for cycles
        if is_cyclic_directed(&*g) {
            // Revert
            g.remove_edge(edge_idx);
            return Err(RoutingError::CycleDetected);
        }

        Ok(edge_idx)
    }

    pub fn disconnect(&self, edge_index: EdgeIndex) -> Result<(), RoutingError> {
        let mut g = self.graph.lock().unwrap();
        if g.remove_edge(edge_index).is_none() {
            return Err(RoutingError::EdgeNotFound);
        }
        Ok(())
    }

    pub fn find_node_by_id(&self, id: Uuid) -> Option<NodeIndex> {
        let g = self.graph.lock().unwrap();
        g.node_indices().find(|i| g[*i].id == id)
    }
}
