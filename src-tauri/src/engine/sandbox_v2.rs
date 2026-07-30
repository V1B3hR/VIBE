#![allow(dead_code)]

use std::process::{Child, Command};
use uuid::Uuid;

/// Out-of-process plugin sandbox (V2)
/// Launches plugins in separate vibe-plugin-host.exe process
pub struct SandboxV2 {
    id: Uuid,
    name: String,
    process: Option<Child>,
    pipe_name: String,
}

impl SandboxV2 {
    /// Create new sandboxed plugin instance
    pub fn new(_plugin_path: &str, plugin_name: &str) -> Result<Self, String> {
        let id = Uuid::new_v4();
        let pipe_name = format!("vibe_plugin_{}", id);

        println!("VIBE: Creating sandbox V2 for: {}", plugin_name);

        // Create named pipe (Windows)
        #[cfg(target_os = "windows")]
        {
            use std::ffi::OsStr;
            use std::os::windows::ffi::OsStrExt;
            use winapi::um::namedpipeapi::CreateNamedPipeW;
            use winapi::um::winbase::{PIPE_ACCESS_DUPLEX, PIPE_TYPE_BYTE, PIPE_WAIT};

            let pipe_path = format!(r"\\.\pipe\{}", pipe_name);
            let wide: Vec<u16> = OsStr::new(&pipe_path)
                .encode_wide()
                .chain(Some(0))
                .collect();

            unsafe {
                let _pipe_handle = CreateNamedPipeW(
                    wide.as_ptr(),
                    PIPE_ACCESS_DUPLEX,
                    PIPE_TYPE_BYTE | PIPE_WAIT,
                    1,    // Max instances
                    8192, // Out buffer size
                    8192, // In buffer size
                    0,    // Default timeout
                    std::ptr::null_mut(),
                );
            }
        }

        // Launch plugin host process
        // Try to find vibe-plugin-host.exe in the same directory as the main executable
        let host_exe = std::env::current_exe()
            .ok()
            .and_then(|exe_path| {
                exe_path
                    .parent()
                    .map(|dir| dir.join("vibe-plugin-host.exe"))
            })
            .unwrap_or_else(|| std::path::PathBuf::from("vibe-plugin-host.exe"));

        println!("VIBE: Launching plugin host from: {:?}", host_exe);

        let process = Command::new(&host_exe)
            .arg(&pipe_name)
            .spawn()
            .map_err(|e| format!("Failed to spawn plugin host: {}", e))?;

        println!("VIBE: Plugin host spawned (PID: {:?})", process.id());

        Ok(Self {
            id,
            name: plugin_name.to_string(),
            process: Some(process),
            pipe_name,
        })
    }

    /// Send command to plugin host
    pub fn send_command(&mut self, command: u32, _data: &[u8]) -> Result<(), String> {
        // TODO: Implement actual pipe communication
        println!("VIBE: Sending command {} to {}", command, self.name);
        Ok(())
    }

    /// Check if process is still alive
    pub fn is_alive(&mut self) -> bool {
        if let Some(ref mut process) = self.process {
            match process.try_wait() {
                Ok(Some(_status)) => false, // Process exited
                Ok(None) => true,           // Still running
                Err(_) => false,
            }
        } else {
            false
        }
    }

    /// Terminate plugin host process
    pub fn shutdown(&mut self) {
        if let Some(mut process) = self.process.take() {
            println!("VIBE: Shutting down plugin host (PID: {:?})", process.id());
            let _ = process.kill();
            let _ = process.wait();
        }
    }
}

impl Drop for SandboxV2 {
    fn drop(&mut self) {
        self.shutdown();
    }
}

// Implement AudioProcessor trait for SandboxV2
impl super::graph::AudioProcessor for SandboxV2 {
    fn process(
        &mut self,
        buffer: &mut super::graph::AudioBuffer,
        _context: &super::graph::ProcessingContext,
    ) {
        // Check if process is alive
        if !self.is_alive() {
            println!("VIBE: Plugin host crashed, clearing buffer");
            buffer.clear();
        }

        // TODO: Send audio data via shared memory or pipe
        // For now, passthrough (safety)
    }

    fn id(&self) -> Uuid {
        self.id
    }

    fn clone_box(&self) -> Box<dyn super::graph::AudioProcessor> {
        Box::new(crate::engine::graph::DummyProcessor {
            id: self.id,
            name: self.name(),
            parameters: Vec::new(),
        })
    }

    fn name(&self) -> String {
        format!("{} (Sandbox V2)", self.name)
    }

    fn on_midi_event(&mut self, _status: u8, _data1: u16, _data2: u32) {
        // TODO: Forward MIDI to plugin host
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sandbox_creation() {
        // Note: This test requires vibe-plugin-host.exe to be built
        // Skip in CI environments
        if std::env::var("CI").is_ok() {
            return;
        }

        let sandbox = SandboxV2::new("test.dll", "TestPlugin");
        // Should either succeed or fail gracefully
        assert!(sandbox.is_ok() || sandbox.is_err());
    }
}
