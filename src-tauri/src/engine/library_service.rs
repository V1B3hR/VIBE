use notify::{Event, RecursiveMode, Watcher};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::errors::Error;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

/// Metadata for an audio file in the library
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioFileMetadata {
    pub id: String,
    pub path: PathBuf,
    pub filename: String,
    pub size_bytes: u64,
    pub duration_seconds: f64,
    pub sample_rate: u32,
    pub channels: u16,
    pub bpm: Option<f32>,
    pub key: Option<String>,
    pub tags: Vec<String>,
    pub category: AudioCategory,
    pub waveform_peaks: Vec<f32>, // For preview rendering
    pub last_modified: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AudioCategory {
    Kick,
    Snare,
    Hat,
    Percussion,
    Bass,
    Synth,
    Vocal,
    Loop,
    OneShot,
    FX,
    Unknown,
}

/// Plugin metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMetadata {
    pub id: String,
    pub name: String,
    pub vendor: String,
    pub path: PathBuf,
    pub plugin_type: PluginType,
    pub thumbnail_path: Option<PathBuf>,
    pub is_blacklisted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PluginType {
    Instrument,
    Effect,
}

/// Main library service managing indexing and search
pub struct LibraryService {
    /// In-memory index for fast search
    audio_files: Arc<Mutex<HashMap<String, AudioFileMetadata>>>,
    #[allow(dead_code)]
    plugins: Arc<Mutex<HashMap<String, PluginMetadata>>>,

    /// Watched directories
    watched_dirs: Arc<Mutex<Vec<PathBuf>>>,

    /// File system watcher
    watcher: Option<notify::RecommendedWatcher>,

    /// Event sender for library updates
    watcher_sender: Option<std::sync::mpsc::Sender<()>>,
}

impl LibraryService {
    pub fn new() -> Self {
        Self {
            audio_files: Arc::new(Mutex::new(HashMap::new())),
            plugins: Arc::new(Mutex::new(HashMap::new())),
            watched_dirs: Arc::new(Mutex::new(Vec::new())),
            watcher: None,
            watcher_sender: None,
        }
    }

    /// Add a directory to watch for audio files
    pub fn add_watch_directory(&mut self, path: PathBuf) -> Result<(), String> {
        {
            let mut dirs = self.watched_dirs.lock().unwrap();
            if !dirs.contains(&path) {
                dirs.push(path.clone());
            }
        } // Release lock before scan

        // Initial scan
        self.scan_directory(&path)?;

        // Setup file watcher
        self.setup_watcher()?;

        Ok(())
    }

    /// Scan a directory recursively for audio files
    fn scan_directory(&self, path: &Path) -> Result<(), String> {
        use walkdir::WalkDir;

        for entry in WalkDir::new(path)
            .follow_links(true)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            if let Some(ext) = path.extension() {
                let ext_str = ext.to_string_lossy().to_lowercase();
                if matches!(ext_str.as_str(), "wav" | "mp3" | "flac" | "ogg" | "aiff") {
                    self.index_audio_file(path)?;
                }
            }
        }

        Ok(())
    }

    /// Index a single audio file
    fn index_audio_file(&self, path: &Path) -> Result<(), String> {
        use std::fs::metadata;
        use uuid::Uuid;

        let meta = metadata(path).map_err(|e| e.to_string())?;
        let filename = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        // Extract BPM and key from filename (common convention: "Kick_128bpm_Cm.wav")
        let (bpm, key) = Self::extract_metadata_from_filename(&filename);

        // Auto-categorize based on filename
        let category = Self::categorize_from_filename(&filename);

        // Generate tags using AI-assisted heuristics and acoustic analysis
        let (duration, sample_rate, channels, peaks) = match Self::analyze_file(path) {
            Ok(res) => res,
            Err(e) => {
                eprintln!("VIBE: Failed to analyze {}: {}", filename, e);
                (0.0, 44100, 2, vec![])
            }
        };

        let mut tags = Self::generate_tags(&filename, &category);
        
        // Acoustic Tagging (AI Heuristics)
        if !peaks.is_empty() {
            // 1. Detect "Punchy" (High peak at start, fast decay)
            let first_quarter = (peaks.len() / 4).max(1);
            let avg_start = peaks[..first_quarter].iter().sum::<f32>() / first_quarter as f32;
            let avg_tail = peaks[first_quarter..].iter().sum::<f32>() / (peaks.len() - first_quarter).max(1) as f32;
            if avg_start > avg_tail * 3.0 {
                tags.push("punchy".to_string());
                tags.push("impact".to_string());
            }

            // 2. Detect "Long" vs "Short"
            if duration > 2.0 {
                tags.push("long".to_string());
                tags.push("sustained".to_string());
            } else if duration < 0.5 {
                tags.push("short".to_string());
                tags.push("transient".to_string());
            }

            // 3. Detect "Loud" / "Compressed"
            let avg_peak: f32 = peaks.iter().sum::<f32>() / peaks.len() as f32;
            if avg_peak > 0.7 {
                tags.push("loud".to_string());
                tags.push("compressed".to_string());
            }
        }

        let file_meta = AudioFileMetadata {
            id: Uuid::new_v4().to_string(),
            path: path.to_path_buf(),
            filename: filename.clone(),
            size_bytes: meta.len(),
            duration_seconds: duration,
            sample_rate,
            channels,
            bpm,
            key,
            tags,
            category,
            waveform_peaks: peaks,
            last_modified: meta
                .modified()
                .map(|t| t.duration_since(std::time::UNIX_EPOCH).unwrap().as_secs())
                .unwrap_or(0),
        };

        let mut files = self.audio_files.lock().unwrap();
        files.insert(file_meta.id.clone(), file_meta);

        Ok(())
    }

    /// Extract BPM and key from filename
    fn extract_metadata_from_filename(filename: &str) -> (Option<f32>, Option<String>) {
        let lower = filename.to_lowercase();

        // Extract BPM (e.g., "128bpm", "120_bpm")
        let bpm = if let Some(pos) = lower.find("bpm") {
            let s = &lower[..pos].trim_end_matches(|c: char| !c.is_numeric() && c != '.');
            let start = s
                .rfind(|c: char| !c.is_numeric() && c != '.')
                .map(|i| i + 1)
                .unwrap_or(0);
            s[start..].parse::<f32>().ok()
        } else {
            None
        };

        // Extract key (e.g., "Cm", "F#m", "Db")
        let key = None; // TODO: Implement key detection

        (bpm, key)
    }

    /// Auto-categorize based on filename keywords
    fn categorize_from_filename(filename: &str) -> AudioCategory {
        let lower = filename.to_lowercase();

        if lower.contains("kick") || lower.contains("bd") {
            AudioCategory::Kick
        } else if lower.contains("snare") || lower.contains("sd") {
            AudioCategory::Snare
        } else if lower.contains("hat") || lower.contains("hh") || lower.contains("hihat") {
            AudioCategory::Hat
        } else if lower.contains("perc") || lower.contains("shaker") || lower.contains("clap") {
            AudioCategory::Percussion
        } else if lower.contains("bass") || lower.contains("sub") {
            AudioCategory::Bass
        } else if lower.contains("synth") || lower.contains("lead") || lower.contains("pad") {
            AudioCategory::Synth
        } else if lower.contains("vocal") || lower.contains("vox") {
            AudioCategory::Vocal
        } else if lower.contains("loop") {
            AudioCategory::Loop
        } else if lower.contains("fx") || lower.contains("sfx") {
            AudioCategory::FX
        } else {
            AudioCategory::OneShot
        }
    }

    /// Generate searchable tags
    pub(crate) fn generate_tags(filename: &str, category: &AudioCategory) -> Vec<String> {
        let mut tags = vec![format!("{:?}", category).to_lowercase()];

        // Add filename parts as tags
        let parts: Vec<String> = filename
            .split(|c: char| !c.is_alphanumeric())
            .filter(|s| !s.is_empty() && s.len() > 2)
            .map(|s| s.to_lowercase())
            .collect();

        tags.extend(parts);
        tags
    }

    /// Fuzzy search across audio files
    pub fn fuzzy_search(&self, query: &str, max_results: usize) -> Vec<AudioFileMetadata> {
        use fuzzy_matcher::skim::SkimMatcherV2;
        use fuzzy_matcher::FuzzyMatcher;

        let matcher = SkimMatcherV2::default();
        let files = self.audio_files.lock().unwrap();
        let query_lower = query.to_lowercase();

        let mut results: Vec<(i64, AudioFileMetadata)> = files
            .values()
            .filter_map(|file| {
                // Try matching filename
                let filename_score = matcher.fuzzy_match(&file.filename, &query_lower);

                // Try matching tags
                let tag_score = file
                    .tags
                    .iter()
                    .filter_map(|tag| matcher.fuzzy_match(tag, &query_lower))
                    .max();

                // Use best score
                let score = filename_score.or(tag_score)?;
                Some((score, file.clone()))
            })
            .collect();

        // Sort by score (descending)
        results.sort_by(|a, b| b.0.cmp(&a.0));

        results
            .into_iter()
            .take(max_results)
            .map(|(_, file)| file)
            .collect()
    }

    /// Tag-based filtering (e.g., "Kick 128bpm Fm")
    #[allow(dead_code)]
    pub fn filter_by_tags(&self, filters: &[String]) -> Vec<AudioFileMetadata> {
        let files = self.audio_files.lock().unwrap();

        files
            .values()
            .filter(|file| {
                filters.iter().all(|filter| {
                    let filter_lower = filter.to_lowercase();

                    // Check filename
                    if file.filename.to_lowercase().contains(&filter_lower) {
                        return true;
                    }

                    // Check tags
                    if file.tags.iter().any(|tag| tag.contains(&filter_lower)) {
                        return true;
                    }

                    // Check BPM
                    if let Some(bpm) = file.bpm {
                        if filter_lower.contains(&bpm.to_string()) {
                            return true;
                        }
                    }

                    // Check key
                    if let Some(ref key) = file.key {
                        if key.to_lowercase().contains(&filter_lower) {
                            return true;
                        }
                    }

                    false
                })
            })
            .cloned()
            .collect()
    }

    /// Set the event sender for library updates
    #[allow(dead_code)]
    pub fn set_event_sender(&mut self, sender: std::sync::mpsc::Sender<()>) {
        self.watcher_sender = Some(sender);
    }

    /// Setup file system watcher for auto-indexing
    fn setup_watcher(&mut self) -> Result<(), String> {
        let _audio_files = Arc::clone(&self.audio_files);
        let sender = self.watcher_sender.clone();

        let mut watcher = notify::recommended_watcher(move |res: Result<Event, notify::Error>| {
            if let Ok(event) = res {
                // Handle file system events
                match event.kind {
                    notify::EventKind::Create(_)
                    | notify::EventKind::Modify(_)
                    | notify::EventKind::Remove(_) => {
                        // Notify frontend/main thread
                        if let Some(ref tx) = sender {
                            let _ = tx.send(());
                        }
                    }
                    _ => {}
                }
            }
        })
        .map_err(|e| e.to_string())?;

        // Watch all directories
        let dirs = self.watched_dirs.lock().unwrap();
        for dir in dirs.iter() {
            watcher
                .watch(dir, RecursiveMode::Recursive)
                .map_err(|e| e.to_string())?;
        }

        self.watcher = Some(watcher);
        Ok(())
    }

    /// Get all indexed files
    #[allow(dead_code)]
    pub fn get_all_files(&self) -> Vec<AudioFileMetadata> {
        self.audio_files.lock().unwrap().values().cloned().collect()
    }

    /// Get files by category
    #[allow(dead_code)]
    pub fn get_by_category(&self, category: AudioCategory) -> Vec<AudioFileMetadata> {
        self.audio_files
            .lock()
            .unwrap()
            .values()
            .filter(|f| f.category == category)
            .cloned()
            .collect()
    }

    /// Analyze audio file to extract metadata and waveform peaks
    fn analyze_file(path: &Path) -> Result<(f64, u32, u16, Vec<f32>), String> {
        let src = File::open(path).map_err(|e| e.to_string())?;
        let mss = MediaSourceStream::new(Box::new(src), Default::default());
        let mut hint = Hint::new();
        if let Some(ext) = path.extension() {
            if let Some(ext_str) = ext.to_str() {
                hint.with_extension(ext_str);
            }
        }

        let probed = symphonia::default::get_probe()
            .format(
                &hint,
                mss,
                &FormatOptions::default(),
                &MetadataOptions::default(),
            )
            .map_err(|e| e.to_string())?;

        let mut format = probed.format;
        let track = format.default_track().ok_or("No track found")?;
        let track_id = track.id;

        let mut decoder = symphonia::default::get_codecs()
            .make(&track.codec_params, &DecoderOptions::default())
            .map_err(|e| e.to_string())?;

        let mut samples: Vec<f32> = Vec::new();
        let mut sample_rate = track.codec_params.sample_rate.unwrap_or(0);
        let mut channels = track
            .codec_params
            .channels
            .map(|c| c.count() as u16)
            .unwrap_or(0);

        while let Ok(packet) = format.next_packet() {
            if packet.track_id() != track_id {
                continue;
            }
            match decoder.decode(&packet) {
                Ok(audio_buf) => {
                    if sample_rate == 0 {
                        sample_rate = audio_buf.spec().rate;
                    }
                    if channels == 0 {
                        channels = audio_buf.spec().channels.count() as u16;
                    }

                    use symphonia::core::audio::SampleBuffer;
                    let mut sample_buf =
                        SampleBuffer::<f32>::new(audio_buf.capacity() as u64, *audio_buf.spec());
                    sample_buf.copy_interleaved_ref(audio_buf);
                    samples.extend_from_slice(sample_buf.samples());
                }
                Err(Error::IoError(_)) => break,
                Err(Error::DecodeError(_)) => continue,
                Err(_) => continue,
            }
        }

        if sample_rate == 0 || channels == 0 {
            return Err("Invalid audio format".to_string());
        }

        let total_frames = samples.len() / (channels as usize).max(1);
        if total_frames == 0 {
            return Ok((0.0, sample_rate, channels, vec![]));
        }

        let duration = total_frames as f64 / sample_rate as f64;

        // Downsample to 100 peaks
        let target_peaks = 100;
        let chunk_size = (total_frames / target_peaks).max(1);
        let mut peaks = Vec::with_capacity(target_peaks);

        for i in 0..target_peaks {
            let start = i * chunk_size * channels as usize;
            let end = (start + chunk_size * channels as usize).min(samples.len());
            if start >= samples.len() {
                break;
            }

            let mut max_val = 0.0f32;
            for j in start..end {
                let abs = samples[j].abs();
                if abs > max_val {
                    max_val = abs;
                }
            }
            peaks.push(max_val);
        }

        Ok((duration, sample_rate, channels, peaks))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bpm_extraction() {
        let (bpm, _) = LibraryService::extract_metadata_from_filename("Kick_128bpm.wav");
        assert_eq!(bpm, Some(128.0));

        let (bpm, _) = LibraryService::extract_metadata_from_filename("Loop_140_bpm_Cm.wav");
        assert_eq!(bpm, Some(140.0));
    }

    #[test]
    fn test_categorization() {
        assert_eq!(
            LibraryService::categorize_from_filename("Kick_Heavy.wav"),
            AudioCategory::Kick
        );
        assert_eq!(
            LibraryService::categorize_from_filename("Snare_808.wav"),
            AudioCategory::Snare
        );
    }

    #[test]
    fn test_fuzzy_search() {
        let _service = LibraryService::new();
        // TODO: Add test files and verify search
    }
}
