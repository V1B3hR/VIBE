#![allow(dead_code)]
use super::persistence::TrackSnapshot;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ProjectSnapshot {
    pub id: Uuid,
    pub timestamp: u64,
    pub action_name: String,
    pub tracks: Vec<TrackSnapshot>,
    pub bpm: f32,
    pub parent_id: Option<Uuid>,
}

pub struct HistoryManager {
    pub nodes: HashMap<Uuid, ProjectSnapshot>,
    pub current_node: Uuid,
    pub branches: HashMap<String, Uuid>, // Branch Name -> Node ID
    pub active_branch: String,           // Tracks the currently active branch
}

impl HistoryManager {
    pub fn new(initial_state: ProjectSnapshot) -> Self {
        let root_id = initial_state.id;
        let mut nodes = HashMap::new();
        nodes.insert(root_id, initial_state);

        let mut branches = HashMap::new();
        branches.insert("main".to_string(), root_id);

        Self {
            nodes,
            current_node: root_id,
            branches,
            active_branch: "main".to_string(),
        }
    }

    pub fn commit(&mut self, mut new_state: ProjectSnapshot) {
        new_state.parent_id = Some(self.current_node);
        let new_id = new_state.id;
        self.nodes.insert(new_id, new_state);
        self.current_node = new_id;

        // Update active branch only
        if let Some(node_id) = self.branches.get_mut(&self.active_branch) {
            *node_id = new_id;
        } else {
            // Should not happen if logic is correct, but safe fallback
            self.branches.insert(self.active_branch.clone(), new_id);
        }
    }

    pub fn checkout(&mut self, node_id: Uuid) -> Option<ProjectSnapshot> {
        if let Some(node) = self.nodes.get(&node_id) {
            self.current_node = node_id;
            Some(node.clone())
        } else {
            None
        }
    }

    pub fn undo(&mut self) -> Option<ProjectSnapshot> {
        let current = self.nodes.get(&self.current_node)?;
        if let Some(parent_id) = current.parent_id {
            self.checkout(parent_id)
        } else {
            None
        }
    }

    pub fn redo(&mut self) -> Option<ProjectSnapshot> {
        // Find children of current node
        let children: Vec<Uuid> = self
            .nodes
            .values()
            .filter(|n| n.parent_id == Some(self.current_node))
            .map(|n| n.id)
            .collect();

        if let Some(child_id) = children.first() {
            self.checkout(*child_id)
        } else {
            None
        }
    }

    pub fn create_branch(&mut self, branch_name: String) {
        self.branches.insert(branch_name.clone(), self.current_node);
        self.active_branch = branch_name;
    }

    pub fn get_history_graph(&self) -> Vec<(Uuid, Option<Uuid>, String)> {
        // Returns (node_id, parent_id, action_name) triplets for visualization
        self.nodes
            .values()
            .map(|n| (n.id, n.parent_id, n.action_name.clone()))
            .collect()
    }
}
