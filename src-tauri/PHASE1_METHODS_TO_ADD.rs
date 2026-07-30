// ========== PHASE 1: Plugin Browser Extensions ==========
// Add these methods to PluginManager impl block (before the closing brace at line 453)

/// Get plugins by category
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
