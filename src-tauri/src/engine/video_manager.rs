use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoState {
    pub path: Option<PathBuf>,
    pub filename: Option<String>,
    pub framerate: f64,
    pub offset_samples: i64,
    pub is_active: bool,
}

pub struct VideoManager {
    state: Arc<Mutex<VideoState>>,
}

impl VideoManager {
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(VideoState {
                path: None,
                filename: None,
                framerate: 24.0,
                offset_samples: 0,
                is_active: false,
            })),
        }
    }

    pub fn load_video(&self, path: PathBuf) -> Result<VideoState, String> {
        let mut state = self.state.lock().unwrap();
        state.filename = path.file_name().map(|n| n.to_string_lossy().to_string());
        state.path = Some(path);
        state.is_active = true;
        Ok(state.clone())
    }

    pub fn unload_video(&self) -> Result<(), String> {
        let mut state = self.state.lock().unwrap();
        state.path = None;
        state.filename = None;
        state.is_active = false;
        Ok(())
    }

    pub fn set_offset(&self, offset_samples: i64) -> Result<(), String> {
        self.state.lock().unwrap().offset_samples = offset_samples;
        Ok(())
    }

    pub fn set_framerate(&self, fps: f64) -> Result<(), String> {
        self.state.lock().unwrap().framerate = fps;
        Ok(())
    }

    pub fn get_state(&self) -> VideoState {
        self.state.lock().unwrap().clone()
    }
}

impl Default for VideoManager {
    fn default() -> Self {
        Self::new()
    }
}
