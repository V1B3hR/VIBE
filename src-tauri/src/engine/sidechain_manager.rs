use uuid::Uuid;

/// Represents a sidechain connection between two nodes in the audio graph.
#[allow(dead_code)]
pub struct SidechainConnection {
    pub source_node_id: Uuid,
    pub target_node_id: Uuid,
    pub amount_db: f32,
    pub enabled: bool,
}

/// Manages the complex sidechain routing matrix.
#[allow(dead_code)]
pub struct SidechainRouter {
    pub connections: Vec<SidechainConnection>,
}

#[allow(dead_code)]
impl SidechainRouter {
    pub fn new() -> Self {
        Self {
            connections: Vec::new(),
        }
    }

    /// Adds a sidechain route (e.g. Kick -> Bass Compressor).
    pub fn add_connection(&mut self, source: Uuid, target: Uuid) {
        self.connections.push(SidechainConnection {
            source_node_id: source,
            target_node_id: target,
            amount_db: 0.0,
            enabled: true,
        });
    }

    /// Returns all sidechain inputs for a given target processor.
    pub fn get_inputs_for(&self, target_id: Uuid) -> Vec<Uuid> {
        self.connections
            .iter()
            .filter(|c| c.target_node_id == target_id && c.enabled)
            .map(|c| c.source_node_id)
            .collect()
    }
}
