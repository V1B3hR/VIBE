use serde::Serialize;
use std::fs;
use std::path::PathBuf;

#[derive(Serialize, Clone, Debug)]
pub struct PluginMetadata {
    pub name: String,
    pub path: String,
    pub size: u64,
}

pub struct PluginScanner {
    pub plugin_dir: PathBuf,
}

impl PluginScanner {
    pub fn new(plugin_dir: PathBuf) -> Self {
        if !plugin_dir.exists() {
            let _ = fs::create_dir_all(&plugin_dir);
        }
        Self { plugin_dir }
    }

    pub fn scan(&self) -> Vec<PluginMetadata> {
        let mut plugins = Vec::new();

        if let Ok(entries) = fs::read_dir(&self.plugin_dir) {
            for entry in entries.filter_map(|e| e.ok()) {
                let path = entry.path();
                let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");

                if ext == "wasm" || ext == "dll" || ext == "vst3" {
                    if let Ok(metadata) = entry.metadata() {
                        let name = path
                            .file_stem()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .into_owned();

                        plugins.push(PluginMetadata {
                            name: if ext == "dll" {
                                format!("{} (VST2)", name)
                            } else if ext == "vst3" {
                                format!("{} (VST3)", name)
                            } else {
                                name
                            },
                            path: path.to_string_lossy().into_owned(),
                            size: metadata.len(),
                        });
                    }
                }
            }
        }

        plugins
    }
}
