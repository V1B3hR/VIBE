use std::path::PathBuf;

use std::thread;
use std::time::Duration;

/// Auto-save manager for crash recovery
/// Periodically saves project state to a binary .vibe-autosave file
pub struct AutoSaveManager {
    interval: Duration,
    #[allow(dead_code)]
    save_path: PathBuf,
    stop_signal: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

impl AutoSaveManager {
    pub fn new(path: PathBuf) -> Self {
        Self {
            interval: Duration::from_secs(30),
            save_path: path,
            stop_signal: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    /// Start auto-save background thread
    /// This will periodically call the save callback
    pub fn start<F>(self, save_callback: F)
    where
        F: Fn() -> Result<(), String> + Send + 'static,
    {
        let interval = self.interval;
        let stop_signal = self.stop_signal.clone();

        thread::spawn(move || {
            println!("VIBE: AutoSave System Started (Interval: {:?})", interval);
            while !stop_signal.load(std::sync::atomic::Ordering::Relaxed) {
                // Sleep inside loop to check more frequently?
                // For now, simple sleep is fine
                thread::sleep(interval);

                if stop_signal.load(std::sync::atomic::Ordering::Relaxed) {
                    break;
                }

                // Perform auto-save
                match save_callback() {
                    Ok(_) => {
                        // Silent success to reduce log noise
                    }
                    Err(e) => {
                        eprintln!("VIBE AutoSave Error: {}", e);
                    }
                }
            }
            println!("VIBE: AutoSave System Stopped gracefully.");
        });
    }

}

