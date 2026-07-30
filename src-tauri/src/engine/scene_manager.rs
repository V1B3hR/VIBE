#![allow(dead_code)]
use std::collections::HashMap;
use uuid::Uuid;

// Scene Manager: Grouping and launching scenes of clips
pub struct Scene {
    pub id: Uuid,
    pub name: String,
    pub clips: HashMap<usize, Uuid>, // track_index -> clip_id
}

pub struct SceneManager {
    pub scenes: Vec<Scene>,
    pub queued_scene_id: Option<Uuid>,
}

impl SceneManager {
    pub fn new() -> Self {
        Self {
            scenes: Vec::new(),
            queued_scene_id: None,
        }
    }

    pub fn launch_scene(&mut self, scene_id: Uuid) {
        self.queued_scene_id = Some(scene_id);
    }
}
