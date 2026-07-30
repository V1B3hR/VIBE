use log::info;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForestResponse {
    pub text: Option<String>,
    pub action_type: Option<String>,
    pub data: Option<Value>,
    pub mood: Option<String>,
    pub animation: Option<String>,
    pub emotion: Option<String>, // NEW: Emotional sensing result
    pub error: Option<String>,
}

pub struct NeuralForestBridge {
    process: Option<Child>,
    python_path: String,
    script_path: String,
}

impl NeuralForestBridge {
    pub fn new(python_path: String, script_path: String) -> Self {
        Self {
            process: None,
            python_path,
            script_path,
        }
    }

    pub async fn start(&mut self) -> Result<(), String> {
        info!("Starting NeuralForest Sidecar...");

        let child = Command::new(&self.python_path)
            .arg(&self.script_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped()) // Capture stderr for logging
            .spawn()
            .map_err(|e| format!("Failed to spawn python process: {}", e))?;

        // Verify it's alive by reading the initial "ready" message
        // This is a bit tricky with ownership, effectively we trust it starts.
        // In a full implementation, we'd handshake here.

        self.process = Some(child);
        info!("NeuralForest Sidecar started successfully.");
        Ok(())
    }

    pub async fn send_command(
        &mut self,
        command: &str,
        data: Option<Value>,
    ) -> Result<ForestResponse, String> {
        if self.process.is_none() {
            // Try restart
            self.start().await?;
        }

        let child = self
            .process
            .as_mut()
            .ok_or("NeuralForest process is not running")?;

        let stdin = child.stdin.as_mut().ok_or("Failed to open stdin")?;
        let stdout = child.stdout.as_mut().ok_or("Failed to open stdout")?;

        let request = serde_json::json!({
            "command": command,
            "data": data.unwrap_or(serde_json::json!({}))
        });

        let mut json_str = request.to_string();
        json_str.push('\n');

        stdin
            .write_all(json_str.as_bytes())
            .await
            .map_err(|e| e.to_string())?;
        stdin.flush().await.map_err(|e| e.to_string())?;

        let mut reader = BufReader::new(stdout);
        let mut line = String::new();

        // Read response line
        match reader.read_line(&mut line).await {
            Ok(0) => return Err("NeuralForest closed the connection (EOF)".to_string()),
            Ok(_) => {}
            Err(e) => return Err(format!("Failed to read response: {}", e)),
        }

        let response: ForestResponse = serde_json::from_str(&line)
            .map_err(|e| format!("Invalid JSON from NeuralForest: {} (raw: {})", e, line))?;

        Ok(response)
    }

    #[allow(dead_code)]
    pub fn kill(&mut self) {
        if let Some(mut child) = self.process.take() {
            let _ = child.kill();
        }
    }

    pub fn is_alive(&self) -> bool {
        if let Some(child) = &self.process {
            // Check if process is still running without blocking
            child.id().is_some()
        } else {
            false
        }
    }
}

// Global thread-safe instance wrapper
// In VIBE architecture, this likely lives in AppState or AudioEngine
