#[tauri::command]
pub fn log_frontend_action(action: String, details: String) {
    log::info!("UI_ACTION: [{}] {}", action, details);
}

#[tauri::command]
pub fn log_frontend_error(message: String, stack: String) {
    log::error!("UI_ERROR: {} \nStack: {}", message, stack);
}

#[tauri::command]
pub fn show_in_explorer(path: String) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .args(["/select,", &path]) // Comma is important
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .args(["-R", &path])
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(target_os = "linux")]
    {
        let parent = std::path::Path::new(&path)
            .parent()
            .unwrap_or(std::path::Path::new(&path));
        std::process::Command::new("xdg-open")
            .arg(parent)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub async fn save_file_dialog(
    app: tauri::AppHandle,
    filters: String,
    default_name: String,
) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::DialogExt;
    let mut dialog = app.dialog().file();

    if !default_name.is_empty() {
        dialog = dialog.set_file_name(default_name);
    }

    if filters == "audio" {
        dialog = dialog.add_filter("Audio", &["wav", "mp3", "flac"]);
    } else if filters == "midi" {
        dialog = dialog.add_filter("MIDI", &["mid", "midi"]);
    }

    let result = dialog.blocking_save_file();
    Ok(result.map(|p| p.to_string()))
}

#[tauri::command]
pub async fn open_file_dialog(
    app: tauri::AppHandle,
    filters: String,
) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::DialogExt;
    let mut dialog = app.dialog().file();

    if filters == "audio" {
        dialog = dialog.add_filter("Audio", &["wav", "mp3", "flac"]);
    } else if filters == "plugin" {
        dialog = dialog.add_filter("Plugin", &["dll", "vst3", "component", "vst"]);
    }

    let result = dialog.blocking_pick_file();
    Ok(result.map(|p| p.to_string()))
}
