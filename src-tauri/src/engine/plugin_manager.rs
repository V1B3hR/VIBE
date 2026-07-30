use notify::{Event, RecursiveMode, Watcher};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};

/// Plugin metadata with safety information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginInfo {
    pub id: String,
    pub name: String,
    pub vendor: String,
    pub path: PathBuf,
    pub plugin_type: PluginType,
    pub is_blacklisted: bool,
    pub blacklist_reason: Option<String>,
    pub thumbnail_path: Option<PathBuf>,
    pub last_scanned: u64,
    // Phase 1 extensions
    pub category: PluginCategory,
    pub is_favorite: bool,
    pub tags: Vec<String>,
    pub last_used: Option<u64>,
    pub custom_folder: Option<String>,
    // Phase 3 extensions
    pub hidden: bool,
    pub deprecated: bool,
    pub duplicate_of: Option<String>,
    pub cpu_usage_avg: Option<f32>,
    pub latency_samples: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PluginType {
    VST2,
    VST3,
    CLAP,
    Native,
    WASM,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PluginCategory {
    Dynamics,   // Compressor, Limiter, Gate, Expander
    EQ,         // Equalizer, Filter
    Reverb,     // Reverb, Room, Hall
    Delay,      // Delay, Echo
    Distortion, // Saturation, Overdrive, Distortion
    Modulation, // Chorus, Flanger, Phaser
    Instrument, // Synth, Sampler, Drum Machine
    Utility,    // Analyzer, Meter, Tuner, Gain
    MidiFX,     // MIDI processors
    Other,      // Uncategorized
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChainRouting {
    Serial,
    Parallel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginChain {
    pub id: String,
    pub name: String,
    pub plugins: Vec<String>, // Plugin IDs
    pub routing: ChainRouting,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanLogEntry {
    pub timestamp: u64,
    pub path: String,
    pub status: String, // "Success", "Failed", "Skipped"
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct PluginDiagnostics {
    pub load_time_ms: u32,
    pub memory_usage_bytes: u64,
    pub error_count: u32,
    pub last_crash: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct PluginPreset {
    pub name: String,
    pub category: String,
    pub author: Option<String>,
    pub path: String,
}

/// Plugin scanner with sandboxed scanning and auto-watch
pub struct PluginManager {
    /// Indexed plugins
    plugins: Arc<Mutex<HashMap<String, PluginInfo>>>,

    /// Watched directories for auto-discovery
    watched_dirs: Arc<Mutex<Vec<PathBuf>>>,

    /// File system watcher
    watcher: Option<notify::RecommendedWatcher>,

    /// Blacklisted plugins (crashed during scan)
    blacklist: Arc<Mutex<Vec<String>>>,

    /// Event sender to notify frontend
    event_sender: Option<Sender<()>>,

    /// Plugin chains (Phase 3)
    chains: Arc<Mutex<HashMap<String, PluginChain>>>,

    /// Expert Phase 4 extensions
    search_paths: Arc<Mutex<Vec<PathBuf>>>,
    excluded_paths: Arc<Mutex<Vec<PathBuf>>>,
    scan_log: Arc<Mutex<Vec<ScanLogEntry>>>,
}

impl PluginManager {
    pub fn new() -> Self {
        let s = Self {
            plugins: Arc::new(Mutex::new(HashMap::new())),
            watched_dirs: Arc::new(Mutex::new(Vec::new())),
            watcher: None,
            blacklist: Arc::new(Mutex::new(Vec::new())),
            event_sender: None,
            chains: Arc::new(Mutex::new(HashMap::new())),
            search_paths: Arc::new(Mutex::new(Vec::new())),
            excluded_paths: Arc::new(Mutex::new(Vec::new())),
            scan_log: Arc::new(Mutex::new(Vec::new())),
        };

        // Register Native Effects
        s.register_native_effect(
            "Convolution Reverb",
            "VIBE",
            "convolution",
            "Reverb",
            &["IR", "Uniform", "Pro"],
        );
        s.register_native_effect(
            "Multiband Dynamics",
            "VIBE",
            "multiband",
            "Dynamics",
            &["4-Band", "Ott", "Pro"],
        );
        s.register_native_effect(
            "Spectral Gate",
            "VIBE",
            "spectralgate",
            "Dynamics",
            &["FFT", "Noise", "Pro"],
        );
        s.register_native_effect(
            "Stereo Imager",
            "VIBE",
            "stereoimager",
            "Utility",
            &["M/S", "Width", "Pro"],
        );

        s
    }

    fn register_native_effect(
        &self,
        name: &str,
        vendor: &str,
        id: &str,
        category_str: &str,
        tags: &[&str],
    ) {
        let mut plugins = self.plugins.lock().unwrap();
        let cat = match category_str {
            "Dynamics" => PluginCategory::Dynamics,
            "EQ" => PluginCategory::EQ,
            "Reverb" => PluginCategory::Reverb,
            "Delay" => PluginCategory::Delay,
            "Utility" => PluginCategory::Utility,
            "Instrument" => PluginCategory::Instrument,
            _ => PluginCategory::Other,
        };

        let info = PluginInfo {
            id: format!("native:{}", name.to_lowercase().replace(' ', "_")),
            name: name.to_string(),
            vendor: vendor.to_string(),
            path: PathBuf::from(format!("native://{}", id)),
            plugin_type: PluginType::Native,
            is_blacklisted: false,
            blacklist_reason: None,
            thumbnail_path: None,
            last_scanned: 0,
            category: cat,
            is_favorite: false,
            tags: tags.iter().map(|&t| t.to_string()).collect(),
            last_used: None,
            custom_folder: None,
            hidden: false,
            deprecated: false,
            duplicate_of: None,
            cpu_usage_avg: None,
            latency_samples: None,
        };
        plugins.insert(info.id.clone(), info);
    }

    #[allow(dead_code)]
    pub fn set_event_sender(&mut self, sender: Sender<()>) {
        self.event_sender = Some(sender);
    }

    /// Add a directory to watch for plugins
    pub fn add_watch_directory(&mut self, path: PathBuf) -> Result<(), String> {
        {
            let mut dirs = self.watched_dirs.lock().unwrap();
            if !dirs.contains(&path) {
                dirs.push(path.clone());
            }
        }

        // Initial scan
        self.scan_directory(&path)?;

        // Setup watcher
        self.setup_watcher()?;

        Ok(())
    }

    /// Scan a directory for plugins (sandboxed)
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

                // Check for plugin files
                if matches!(ext_str.as_str(), "dll" | "vst3" | "clap" | "wasm") {
                    // Sandboxed scan (in separate process to prevent crashes)
                    self.scan_plugin_sandboxed(path)?;
                }
            }
        }

        Ok(())
    }

    /// Scan a single plugin in sandboxed environment
    fn scan_plugin_sandboxed(&self, path: &Path) -> Result<(), String> {
        use uuid::Uuid;

        let path_str = path.to_string_lossy().to_string();

        // Check exclusions (Phase 4)
        {
            let exclusions = self.excluded_paths.lock().unwrap();
            if exclusions.iter().any(|p| path.starts_with(p)) {
                self.log_scan(ScanLogEntry {
                    timestamp: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs(),
                    path: path_str.clone(),
                    status: "Skipped".into(),
                    error: Some("Excluded directory".into()),
                });
                return Ok(());
            }
        }

        // 1. Check duplicate by path
        let mut plugins = self.plugins.lock().unwrap();
        if plugins
            .values()
            .any(|p| p.path.to_string_lossy() == path_str)
        {
            return Ok(()); // Already scanned
        }

        // 2. Check blacklist
        let blacklist = self.blacklist.lock().unwrap();
        if blacklist.contains(&path_str) {
            return Ok(()); // Skip blacklisted plugins
        }
        drop(blacklist);

        // TODO: Implement actual sandboxed scanning (separate process)
        // Actual VST3 Probing (Simplified version of bridge logic)
        let plugin_type = Self::detect_plugin_type(path);
        let vendor = "Unknown".to_string();
        let mut plugin_name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Unknown")
            .to_string();

        if plugin_type == PluginType::VST3 {
            if let Ok(lib) = unsafe { libloading::Library::new(path) } {
                unsafe {
                    if let Ok(get_factory) = lib.get::<unsafe extern "system" fn() -> *mut std::ffi::c_void>(b"GetPluginFactory\0") {
                        let factory = get_factory();
                        if !factory.is_null() {
                            // Minimal COM query for ClassInfo
                            type GetClassInfoFn = unsafe extern "system" fn(*mut std::ffi::c_void, i32, *mut crate::engine::vst3_bridge::PClassInfo) -> i32;
                            // Offset of get_class_info in Vtable is 3 (IUnknown: 0,1,2 + Factory: 1,2,3) -> No, check vst3_bridge
                            // In IPluginFactoryVtbl: base(3) + get_factory_info(0) + count_classes(1) + get_class_info(2) -> index 5
                            let vtable = *(factory as *mut *mut *const std::ffi::c_void);
                            let count_classes_ptr = *vtable.add(4);
                            let get_class_info_ptr = *vtable.add(5);
                            
                            let count_classes: unsafe extern "system" fn(*mut std::ffi::c_void) -> i32 = std::mem::transmute(count_classes_ptr);
                            let get_class_info: GetClassInfoFn = std::mem::transmute(get_class_info_ptr);
                            
                            let count = count_classes(factory);
                            for i in 0..count {
                                let mut info = std::mem::zeroed();
                                if get_class_info(factory, i, &mut info) == 0 {
                                    let cat = std::ffi::CStr::from_ptr(info.category.as_ptr()).to_string_lossy();
                                    if cat == "Audio Effect" || cat == "Instrument" {
                                        plugin_name = std::ffi::CStr::from_ptr(info.name.as_ptr()).to_string_lossy().into_owned();
                                        // Vendor isn't directly in PClassInfo (it's in PFactoryInfo usually),
                                        // but for a quick scan this is 100x better than filename.
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // 3. Generate stable ID based on path (Namespace UUID)
        let namespace = Uuid::parse_str("6ba7b810-9dad-11d1-80b4-00c04fd430c8").unwrap(); // DNS namespace
        let stable_id = Uuid::new_v5(&namespace, path_str.as_bytes()).to_string();

        let plugin_info = PluginInfo {
            id: stable_id.clone(),
            name: plugin_name.clone(),
            vendor,
            path: path.to_path_buf(),
            plugin_type: plugin_type.clone(),
            is_blacklisted: false,
            blacklist_reason: None,
            thumbnail_path: None,
            last_scanned: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            category: Self::auto_categorize(&plugin_name, &plugin_type),
            is_favorite: false,
            tags: Vec::new(),
            last_used: None,
            custom_folder: None,
            hidden: false,
            deprecated: false,
            duplicate_of: None,
            cpu_usage_avg: None,
            latency_samples: None,
        };

        plugins.insert(plugin_info.id.clone(), plugin_info);

        self.log_scan(ScanLogEntry {
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            path: path_str,
            status: "Success".into(),
            error: None,
        });

        Ok(())
    }

    /// Detect plugin type from file extension
    fn detect_plugin_type(path: &Path) -> PluginType {
        if let Some(ext) = path.extension() {
            match ext.to_string_lossy().to_lowercase().as_str() {
                "vst3" => PluginType::VST3,
                "clap" => PluginType::CLAP,
                "wasm" => PluginType::WASM,
                "dll" => PluginType::VST2, // Assume VST2 for .dll
                _ => PluginType::VST2,
            }
        } else {
            PluginType::VST2
        }
    }

    /// Auto-categorize plugin based on name patterns
    fn auto_categorize(name: &str, plugin_type: &PluginType) -> PluginCategory {
        let name_lower = name.to_lowercase();

        // 1. Instrument detection (High priority)
        if *plugin_type == PluginType::Native
            || name_lower.contains("synth")
            || name_lower.contains("sampler")
            || name_lower.contains("drum")
            || name_lower.contains("piano")
            || name_lower.contains("organ")
            || name_lower.contains("bass")
            || name_lower.contains("vone")
            || name_lower.contains("kontakt")
            || name_lower.contains("omnisphere")
            || name_lower.contains("serum")
            || name_lower.contains("vital")
        {
            return PluginCategory::Instrument;
        }

        // 2. Dynamics
        if name_lower.contains("comp")
            || name_lower.contains("limit")
            || name_lower.contains("gate")
            || name_lower.contains("expander")
            || name_lower.contains("dynamics")
            || name_lower.contains("de-ess")
        {
            return PluginCategory::Dynamics;
        }

        // 3. EQ & Filters
        if name_lower.contains("eq")
            || name_lower.contains("equal")
            || name_lower.contains("filter")
            || name_lower.contains("prisma")
            || name_lower.contains("cutoff")
            || name_lower.contains("shelf")
        {
            return PluginCategory::EQ;
        }

        // 4. Reverb
        if name_lower.contains("reverb")
            || name_lower.contains("room")
            || name_lower.contains("hall")
            || name_lower.contains("plate")
            || name_lower.contains("ambience")
            || name_lower.contains("space")
        {
            return PluginCategory::Reverb;
        }

        // 5. Delay & Echo
        if name_lower.contains("delay") || name_lower.contains("echo") || name_lower.contains("tap") || name_lower.contains("repeat") {
            return PluginCategory::Delay;
        }

        // 6. Distortion & Saturation
        if name_lower.contains("dist")
            || name_lower.contains("sat")
            || name_lower.contains("drive")
            || name_lower.contains("amp")
            || name_lower.contains("fuzz")
            || name_lower.contains("tube")
            || name_lower.contains("crush")
            || name_lower.contains("overdrive")
        {
            return PluginCategory::Distortion;
        }

        // 7. Modulation
        if name_lower.contains("chorus")
            || name_lower.contains("flange")
            || name_lower.contains("phaser")
            || name_lower.contains("tremolo")
            || name_lower.contains("vibrato")
            || name_lower.contains("ensemble")
            || name_lower.contains("rotary")
        {
            return PluginCategory::Modulation;
        }

        // 8. Utility & Metering
        if name_lower.contains("util")
            || name_lower.contains("meter")
            || name_lower.contains("gain")
            || name_lower.contains("anal")
            || name_lower.contains("scope")
            || name_lower.contains("tuner")
            || name_lower.contains("mono")
            || name_lower.contains("stereo")
        {
            return PluginCategory::Utility;
        }

        // 9. MIDI FX
        if name_lower.contains("midi") || name_lower.contains("arp") || name_lower.contains("chord") || name_lower.contains("seq") {
            return PluginCategory::MidiFX;
        }

        PluginCategory::Other
    }

    /// Blacklist a plugin by its ID (Phase 2)
    pub fn blacklist_plugin(&self, plugin_id: &str, reason: &str) -> Result<(), String> {
        let mut plugins = self.plugins.lock().unwrap();
        if let Some(plugin) = plugins.get_mut(plugin_id) {
            plugin.is_blacklisted = true;
            plugin.blacklist_reason = Some(reason.to_string());

            let path_str = plugin.path.to_string_lossy().to_string();
            let mut blacklist = self.blacklist.lock().unwrap();
            if !blacklist.contains(&path_str) {
                blacklist.push(path_str);
            }
            Ok(())
        } else {
            Err(format!("Plugin not found: {}", plugin_id))
        }
    }

    /// Blacklist a plugin by path (used during scan crashes)
    #[allow(dead_code)]
    pub fn blacklist_by_path(&self, path: &str, reason: &str) -> Result<(), String> {
        let mut blacklist = self.blacklist.lock().unwrap();
        if !blacklist.contains(&path.to_string()) {
            blacklist.push(path.to_string());
        }

        let mut plugins = self.plugins.lock().unwrap();
        if let Some(plugin) = plugins
            .values_mut()
            .find(|p| p.path.to_string_lossy() == path)
        {
            plugin.is_blacklisted = true;
            plugin.blacklist_reason = Some(reason.to_string());
        }
        Ok(())
    }

    /// Get all plugins
    #[allow(dead_code)]
    pub fn get_all_plugins(&self) -> Vec<PluginInfo> {
        self.plugins.lock().unwrap().values().cloned().collect()
    }

    /// Get plugins by type
    #[allow(dead_code)]
    pub fn get_by_type(&self, plugin_type: PluginType) -> Vec<PluginInfo> {
        self.plugins
            .lock()
            .unwrap()
            .values()
            .filter(|p| p.plugin_type == plugin_type && !p.is_blacklisted)
            .cloned()
            .collect()
    }

    /// Search plugins by name
    #[allow(dead_code)]
    pub fn search(&self, query: &str) -> Vec<PluginInfo> {
        let query_lower = query.to_lowercase();
        self.plugins
            .lock()
            .unwrap()
            .values()
            .filter(|p| {
                p.name.to_lowercase().contains(&query_lower)
                    || p.vendor.to_lowercase().contains(&query_lower)
            })
            .cloned()
            .collect()
    }

    /// Setup file system watcher for auto-discovery
    fn setup_watcher(&mut self) -> Result<(), String> {
        let plugins = Arc::clone(&self.plugins);
        let blacklist = Arc::clone(&self.blacklist);
        let sender = self.event_sender.clone();

        let mut watcher = notify::recommended_watcher(move |res: Result<Event, notify::Error>| {
            if let Ok(event) = res {
                use uuid::Uuid;
                if let notify::EventKind::Create(_) = event.kind {
                    for path in event.paths {
                        if let Some(ext) = path.extension() {
                            let ext_str = ext.to_string_lossy().to_lowercase();
                            if matches!(ext_str.as_str(), "dll" | "vst3" | "clap") {
                                println!("🔌 New plugin detected: {:?}", path);

                                // Logic from scan_plugin_sandboxed, adapted for partial-thread context
                                let path_str = path.to_string_lossy().to_string();
                                let bl = blacklist.lock().unwrap();
                                if bl.contains(&path_str) {
                                    return;
                                }
                                drop(bl);

                                let plugin_type = Self::detect_plugin_type(&path);
                                let plugin_name = path
                                    .file_stem()
                                    .and_then(|s| s.to_str())
                                    .unwrap_or("Unknown")
                                    .to_string();

                                let plugin_info = PluginInfo {
                                    id: Uuid::new_v4().to_string(),
                                    name: plugin_name.clone(),
                                    vendor: "Unknown".to_string(),
                                    path: path.clone(),
                                    plugin_type: plugin_type.clone(),
                                    is_blacklisted: false,
                                    blacklist_reason: None,
                                    thumbnail_path: None,
                                    last_scanned: std::time::SystemTime::now()
                                        .duration_since(std::time::UNIX_EPOCH)
                                        .unwrap()
                                        .as_secs(),
                                    category: Self::auto_categorize(&plugin_name, &plugin_type),
                                    is_favorite: false,
                                    tags: Vec::new(),
                                    last_used: None,
                                    custom_folder: None,
                                    hidden: false,
                                    deprecated: false,
                                    duplicate_of: None,
                                    cpu_usage_avg: None,
                                    latency_samples: None,
                                };

                                let mut p = plugins.lock().unwrap();
                                p.insert(plugin_info.id.clone(), plugin_info);

                                // Notify Frontend
                                if let Some(tx) = &sender {
                                    let _ = tx.send(());
                                }
                            }
                        }
                    }
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

    /// Generate thumbnail for plugin (placeholder)
    #[allow(dead_code)]
    pub fn generate_thumbnail(&self, _plugin_id: &str) -> Result<Option<PathBuf>, String> {
        // TODO: Implement screenshot capture of plugin GUI
        // For now, return None to indicate no thumbnail available
        Ok(None)
    }

    /// Load a plugin instance from path
    pub fn load_plugin(
        &self,
        path_str: &str,
    ) -> Result<Box<dyn crate::engine::graph::AudioProcessor>, String> {
        let path = Path::new(path_str);
        if !path.exists() {
            return Err(format!("Plugin file not found: {}", path_str));
        }

        let ext = path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_lowercase();

        match ext.as_str() {
            "vst3" => {
                // Determine sample rate/block size (defaults for now)
                let sample_rate = 48000.0;
                let block_size = 512;

                let plugin =
                    crate::engine::vst3_bridge::Vst3Bridge::new(path_str, sample_rate, block_size)?;
                Ok(Box::new(plugin))
            }
            // Future: VST2, CLAP, WASM
            _ => Err(format!("Unsupported plugin type: .{}", ext)),
        }
    }

    // ========== PHASE 1: Plugin Browser Extensions ==========

    /// Get plugins by category
    #[allow(dead_code)]
    pub fn get_by_category(&self, category: PluginCategory) -> Vec<PluginInfo> {
        self.plugins
            .lock()
            .unwrap()
            .values()
            .filter(|p| p.category == category && !p.is_blacklisted)
            .cloned()
            .collect()
    }

    /// Get favorite plugins
    #[allow(dead_code)]
    pub fn get_favorites(&self) -> Vec<PluginInfo> {
        self.plugins
            .lock()
            .unwrap()
            .values()
            .filter(|p| p.is_favorite && !p.is_blacklisted)
            .cloned()
            .collect()
    }

    /// Toggle favorite status
    pub fn toggle_favorite(&self, plugin_id: &str) -> Result<(), String> {
        let mut plugins = self.plugins.lock().unwrap();
        if let Some(plugin) = plugins.get_mut(plugin_id) {
            plugin.is_favorite = !plugin.is_favorite;
            Ok(())
        } else {
            Err(format!("Plugin not found: {}", plugin_id))
        }
    }

    /// Add tag to plugin
    #[allow(dead_code)]
    pub fn add_tag(&self, plugin_id: &str, tag: String) -> Result<(), String> {
        let mut plugins = self.plugins.lock().unwrap();
        if let Some(plugin) = plugins.get_mut(plugin_id) {
            if !plugin.tags.contains(&tag) {
                plugin.tags.push(tag);
            }
            Ok(())
        } else {
            Err(format!("Plugin not found: {}", plugin_id))
        }
    }

    /// Remove tag from plugin
    #[allow(dead_code)]
    pub fn remove_tag(&self, plugin_id: &str, tag: &str) -> Result<(), String> {
        let mut plugins = self.plugins.lock().unwrap();
        if let Some(plugin) = plugins.get_mut(plugin_id) {
            plugin.tags.retain(|t| t != tag);
            Ok(())
        } else {
            Err(format!("Plugin not found: {}", plugin_id))
        }
    }

    /// Get plugins by tag
    #[allow(dead_code)]
    pub fn get_by_tag(&self, tag: &str) -> Vec<PluginInfo> {
        self.plugins
            .lock()
            .unwrap()
            .values()
            .filter(|p| p.tags.contains(&tag.to_string()) && !p.is_blacklisted)
            .cloned()
            .collect()
    }

    /// Update last used timestamp
    pub fn update_last_used(&self, plugin_id: &str) -> Result<(), String> {
        let mut plugins = self.plugins.lock().unwrap();
        if let Some(plugin) = plugins.get_mut(plugin_id) {
            plugin.last_used = Some(
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
            );
            Ok(())
        } else {
            Err(format!("Plugin not found: {}", plugin_id))
        }
    }

    /// Get recently used plugins
    #[allow(dead_code)]
    pub fn get_recent(&self, limit: usize) -> Vec<PluginInfo> {
        let mut plugins: Vec<PluginInfo> = self
            .plugins
            .lock()
            .unwrap()
            .values()
            .filter(|p| p.last_used.is_some() && !p.is_blacklisted)
            .cloned()
            .collect();

        plugins.sort_by(|a, b| b.last_used.cmp(&a.last_used));
        plugins.truncate(limit);
        plugins
    }

    /// Set custom folder/category for a plugin (Phase 2)
    pub fn set_custom_folder(&self, plugin_id: &str, folder: Option<String>) -> Result<(), String> {
        let mut plugins = self.plugins.lock().unwrap();
        if let Some(plugin) = plugins.get_mut(plugin_id) {
            plugin.custom_folder = folder;
            Ok(())
        } else {
            Err(format!("Plugin not found: {}", plugin_id))
        }
    }

    /// Get plugins by custom folder (Phase 2)
    #[allow(dead_code)]
    pub fn get_by_custom_folder(&self, folder: &str) -> Vec<PluginInfo> {
        self.plugins
            .lock()
            .unwrap()
            .values()
            .filter(|p| p.custom_folder.as_deref() == Some(folder) && !p.is_blacklisted)
            .cloned()
            .collect()
    }

    // ========== PHASE 3: Advanced Management & Performance ==========

    #[allow(dead_code)]
    pub fn set_hidden(&self, plugin_id: &str, hidden: bool) -> Result<(), String> {
        let mut plugins = self.plugins.lock().unwrap();
        if let Some(plugin) = plugins.get_mut(plugin_id) {
            plugin.hidden = hidden;
            Ok(())
        } else {
            Err(format!("Plugin not found: {}", plugin_id))
        }
    }

    #[allow(dead_code)]
    pub fn set_deprecated(&self, plugin_id: &str, deprecated: bool) -> Result<(), String> {
        let mut plugins = self.plugins.lock().unwrap();
        if let Some(plugin) = plugins.get_mut(plugin_id) {
            plugin.deprecated = deprecated;
            Ok(())
        } else {
            Err(format!("Plugin not found: {}", plugin_id))
        }
    }

    #[allow(dead_code)]
    pub fn merge_duplicates(&self, primary_id: &str, duplicate_id: &str) -> Result<(), String> {
        let mut plugins = self.plugins.lock().unwrap();
        if plugins.contains_key(primary_id) && plugins.contains_key(duplicate_id) {
            if let Some(dup) = plugins.get_mut(duplicate_id) {
                dup.hidden = true;
                dup.duplicate_of = Some(primary_id.to_string());
                Ok(())
            } else {
                Err("Duplicate not found".into())
            }
        } else {
            Err("One or both plugins not found".into())
        }
    }

    #[allow(dead_code)]
    pub fn update_performance(
        &self,
        plugin_id: &str,
        cpu: f32,
        latency: u32,
    ) -> Result<(), String> {
        let mut plugins = self.plugins.lock().unwrap();
        if let Some(plugin) = plugins.get_mut(plugin_id) {
            plugin.cpu_usage_avg = Some(cpu);
            plugin.latency_samples = Some(latency);
            Ok(())
        } else {
            Err(format!("Plugin not found: {}", plugin_id))
        }
    }

    // Plugin Chains

    #[allow(dead_code)]
    pub fn save_chain(&self, chain: PluginChain) -> Result<(), String> {
        let mut chains = self.chains.lock().unwrap();
        chains.insert(chain.id.clone(), chain);
        Ok(())
    }

    #[allow(dead_code)]
    pub fn get_all_chains(&self) -> Vec<PluginChain> {
        self.chains.lock().unwrap().values().cloned().collect()
    }

    #[allow(dead_code)]
    pub fn delete_chain(&self, chain_id: &str) -> Result<(), String> {
        let mut chains = self.chains.lock().unwrap();
        if chains.remove(chain_id).is_some() {
            Ok(())
        } else {
            Err("Chain not found".into())
        }
    }

    #[allow(dead_code)]
    pub fn rescan_all(&self) -> Result<(), String> {
        let paths = self.get_search_paths();
        for path in paths {
            let _ = self.scan_directory(&path);
        }
        Ok(())
    }

    // ========== PHASE 4: EXPERT - Database & Diagnostics ==========

    #[allow(dead_code)]
    pub fn add_search_path(&self, path: PathBuf) {
        let mut paths = self.search_paths.lock().unwrap();
        if !paths.contains(&path) {
            paths.push(path);
        }
    }

    #[allow(dead_code)]
    pub fn remove_search_path(&self, path: &Path) {
        let mut paths = self.search_paths.lock().unwrap();
        paths.retain(|p| p != path);
    }

    #[allow(dead_code)]
    pub fn get_search_paths(&self) -> Vec<PathBuf> {
        self.search_paths.lock().unwrap().clone()
    }

    #[allow(dead_code)]
    pub fn add_exclusion(&self, path: PathBuf) {
        let mut paths = self.excluded_paths.lock().unwrap();
        if !paths.contains(&path) {
            paths.push(path);
        }
    }

    #[allow(dead_code)]
    pub fn get_scan_log(&self) -> Vec<ScanLogEntry> {
        self.scan_log.lock().unwrap().clone()
    }

    pub fn log_scan(&self, entry: ScanLogEntry) {
        let mut log = self.scan_log.lock().unwrap();
        log.push(entry);
        if log.len() > 1000 {
            log.remove(0);
        }
    }

    #[allow(dead_code)]
    pub fn get_diagnostics(&self, _plugin_id: &str) -> Result<PluginDiagnostics, String> {
        // Mocking diagnostics for now, will be connected to audio engine stats later
        Ok(PluginDiagnostics {
            load_time_ms: 42,
            memory_usage_bytes: 1024 * 1024 * 15,
            error_count: 0,
            last_crash: None,
        })
    }

    /// Migrate VST2 references to VST3 if available
    #[allow(dead_code)]
    pub fn migrate_vst2_to_vst3(&self, plugin_id: &str) -> Result<String, String> {
        let plugins = self.plugins.lock().unwrap();
        if let Some(plugin) = plugins.get(plugin_id) {
            if plugin.plugin_type == PluginType::VST2 {
                // Search for VST3 version with same name/vendor
                if let Some(vst3) = plugins.values().find(|p| {
                    p.plugin_type == PluginType::VST3
                        && p.name == plugin.name
                        && p.vendor == plugin.vendor
                }) {
                    return Ok(vst3.id.clone());
                }
            }
        }
        Err("No suitable VST3 migration found".into())
    }

    #[allow(dead_code)]
    pub fn get_presets(&self, _plugin_id: &str) -> Result<Vec<PluginPreset>, String> {
        // Phase 4: Placeholder for actual preset scanning (.vstpreset, .fxp)
        Ok(vec![
            PluginPreset {
                name: "Init Patch".into(),
                category: "Basic".into(),
                author: Some("VIBE".into()),
                path: "".into(),
            },
            PluginPreset {
                name: "Deep Space Bass".into(),
                category: "Bass".into(),
                author: Some("VIBE".into()),
                path: "".into(),
            },
            PluginPreset {
                name: "Spectral Lead".into(),
                category: "Lead".into(),
                author: Some("VIBE".into()),
                path: "".into(),
            },
        ])
    }

    #[allow(dead_code)]
    pub fn set_thumbnail(&self, plugin_id: &str, path: String) -> Result<(), String> {
        let mut plugins = self.plugins.lock().unwrap();
        if let Some(plugin) = plugins.get_mut(plugin_id) {
            plugin.thumbnail_path = Some(std::path::PathBuf::from(path));
            Ok(())
        } else {
            Err("Plugin not found".into())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_manager_creation() {
        let manager = PluginManager::new();
        assert_eq!(manager.get_all_plugins().len(), 4);
    }

    #[test]
    fn test_plugin_type_detection() {
        let vst3_path = PathBuf::from("test.vst3");
        assert_eq!(
            PluginManager::detect_plugin_type(&vst3_path),
            PluginType::VST3
        );

        let clap_path = PathBuf::from("test.clap");
        assert_eq!(
            PluginManager::detect_plugin_type(&clap_path),
            PluginType::CLAP
        );
    }

    #[test]
    fn test_blacklist_functionality() {
        let manager = PluginManager::new();
        let path = "bad_plugin.dll";
        manager.blacklist_by_path(path, "Crashed").unwrap();

        let blacklist = manager.blacklist.lock().unwrap();
        assert!(blacklist.contains(&path.to_string()));
    }

    #[test]
    fn test_auto_categorization() {
        assert_eq!(
            PluginManager::auto_categorize("Super Synth", &PluginType::VST3),
            PluginCategory::Instrument
        );
        assert_eq!(
            PluginManager::auto_categorize("Pro Compressor", &PluginType::VST3),
            PluginCategory::Dynamics
        );
        assert_eq!(
            PluginManager::auto_categorize("FabFilter Pro-Q 3", &PluginType::VST3),
            PluginCategory::EQ
        );
        assert_eq!(
            PluginManager::auto_categorize("ValhallaRoom", &PluginType::VST3),
            PluginCategory::Reverb
        );
        assert_eq!(
            PluginManager::auto_categorize("Unknown Plugin", &PluginType::VST3),
            PluginCategory::Other
        );
    }

    #[test]
    fn test_favorites_management() {
        let manager = PluginManager::new();
        // Manually insert a plugin (since scan is hard to test without files)
        let id = "test_id";
        let info = PluginInfo {
            id: id.to_string(),
            name: "Test Plugin".into(),
            vendor: "Vendor".into(),
            path: PathBuf::from("test.vst3"),
            plugin_type: PluginType::VST3,
            is_blacklisted: false,
            blacklist_reason: None,
            thumbnail_path: None,
            last_scanned: 0,
            category: PluginCategory::Other,
            is_favorite: false,
            tags: vec![],
            last_used: None,
            custom_folder: None,
            hidden: false,
            deprecated: false,
            duplicate_of: None,
            cpu_usage_avg: None,
            latency_samples: None,
        };

        manager.plugins.lock().unwrap().insert(id.to_string(), info);

        assert_eq!(manager.get_favorites().len(), 0);
        manager.toggle_favorite(id).unwrap();
        assert_eq!(manager.get_favorites().len(), 1);
        assert_eq!(manager.get_favorites()[0].id, id);
        manager.toggle_favorite(id).unwrap();
        assert_eq!(manager.get_favorites().len(), 0);
    }

    #[test]
    fn test_tag_system() {
        let manager = PluginManager::new();
        let id = "tag_test_id";
        let info = PluginInfo {
            id: id.to_string(),
            name: "Tagged Plugin".into(),
            vendor: "Vendor".into(),
            path: PathBuf::from("test.vst3"),
            plugin_type: PluginType::VST3,
            is_blacklisted: false,
            blacklist_reason: None,
            thumbnail_path: None,
            last_scanned: 0,
            category: PluginCategory::Other,
            is_favorite: false,
            tags: vec![],
            last_used: None,
            custom_folder: None,
            hidden: false,
            deprecated: false,
            duplicate_of: None,
            cpu_usage_avg: None,
            latency_samples: None,
        };
        manager.plugins.lock().unwrap().insert(id.to_string(), info);

        manager.add_tag(id, "Analog".into()).unwrap();
        manager.add_tag(id, "Vintage".into()).unwrap();

        let tagged = manager.get_by_tag("Analog");
        assert_eq!(tagged.len(), 1);
        assert!(tagged[0].tags.contains(&"Analog".to_string()));
        assert!(tagged[0].tags.contains(&"Vintage".to_string()));

        manager.remove_tag(id, "Analog").unwrap();
        assert_eq!(manager.get_by_tag("Analog").len(), 0);
    }

    #[test]
    fn test_custom_folders() {
        let manager = PluginManager::new();
        let id = "folder_test";
        let info = PluginInfo {
            id: id.to_string(),
            name: "Folder Plugin".into(),
            vendor: "Vendor".into(),
            path: PathBuf::from("test.vst3"),
            plugin_type: PluginType::VST3,
            is_blacklisted: false,
            blacklist_reason: None,
            thumbnail_path: None,
            last_scanned: 0,
            category: PluginCategory::Other,
            is_favorite: false,
            tags: vec![],
            last_used: None,
            custom_folder: None,
            hidden: false,
            deprecated: false,
            duplicate_of: None,
            cpu_usage_avg: None,
            latency_samples: None,
        };
        manager.plugins.lock().unwrap().insert(id.to_string(), info);

        manager
            .set_custom_folder(id, Some("My Favorites".into()))
            .unwrap();
        let in_folder = manager.get_by_custom_folder("My Favorites");
        assert_eq!(in_folder.len(), 1);
        assert_eq!(in_folder[0].custom_folder.as_deref(), Some("My Favorites"));
    }

    #[test]
    fn test_search_paths() {
        let manager = PluginManager::new();
        let p = PathBuf::from("C:\\VSTPlugins");

        manager.add_search_path(p.clone());
        assert!(manager.get_search_paths().contains(&p));

        manager.remove_search_path(&p);
        assert!(!manager.get_search_paths().contains(&p));
    }
}
