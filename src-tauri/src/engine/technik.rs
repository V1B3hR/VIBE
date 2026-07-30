use super::graph::Track;
use std::sync::atomic::Ordering;
use uuid::Uuid;

pub struct Technik;

#[derive(Debug, Clone, serde::Serialize)]
pub struct ProjectHealth {
    pub cpu_percent: f32,
    pub memory_percent: f32,
    pub disk_status: String,
    pub buffer_health: f32,
}

impl Technik {
    pub fn get_track_cpu_percent(track: &Track, sample_rate: f64) -> f32 {
        let micros = track.cpu_usage.load(Ordering::Relaxed);
        // Assuming block size of 1024 frames for calculation
        let frames = 1024.0;
        let budget_us = (frames / sample_rate) * 1_000_000.0;
        if budget_us > 0.0 {
            (micros as f32 / budget_us as f32) * 100.0
        } else {
            0.0
        }
    }

    pub fn suggest_freeze(tracks: &[Track], sample_rate: f64) -> Option<(Uuid, String, f32)> {
        for track in tracks {
            if track.is_frozen
                || track.is_disabled
                || track.track_type == crate::engine::graph::TrackType::Folder
            {
                continue;
            }

            let cpu = Self::get_track_cpu_percent(track, sample_rate);
            if cpu > 15.0 {
                return Some((track.id, track.name.clone(), cpu));
            }
        }
        None
    }

    pub fn monitor_system_health(cpu_total: f32) -> ProjectHealth {
        // Simplified memory/disk monitoring for Level 3
        ProjectHealth {
            cpu_percent: cpu_total,
            memory_percent: 24.5, // Dummy
            disk_status: "Green".to_string(),
            buffer_health: 100.0,
        }
    }

    pub fn suggest_silence_sweep(track: &Track) -> Option<u64> {
        // Find tracks with audio clips but long gaps (simulated)
        if track.clips.is_empty() {
            return None;
        }
        // If track has processors but no audio for 30 seconds...
        // This is a placeholder for actual I/O silence detection
        None
    }

    pub fn check_project_integrity(tracks: &[Track]) -> Vec<String> {
        let mut issues = Vec::new();
        for track in tracks {
            for clip in &track.clips {
                if clip.path.as_ref().map(|p| p.is_empty()).unwrap_or(true) {
                    issues.push(format!(
                        "Missing audio path for clip '{}' on track '{}'",
                        clip.name, track.name
                    ));
                }
            }
        }
        issues
    }
}
