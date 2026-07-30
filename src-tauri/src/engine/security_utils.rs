#![allow(dead_code)]

use std::path::{Component, Path, PathBuf};

/// Security Utility for Path Traversal Protection and Input Validation
pub struct PathSecurity;

impl PathSecurity {
    /// Validates and canonicalizes a path, ensuring it stays strictly inside allowed root directories
    pub fn validate_safe_path(path: &Path, allowed_roots: &[&Path]) -> Result<PathBuf, String> {
        let path_str = path.to_string_lossy();

        // 1. Check for null byte injection
        if path_str.contains('\0') {
            return Err("Invalid path: contains null byte injection".to_string());
        }

        // 2. Reject explicit relative traversal attempts
        if path.components().any(|c| c == Component::ParentDir) {
            return Err("Access denied: path traversal ('..') detected".to_string());
        }

        // 3. Normalize path components
        let mut normalized = PathBuf::new();
        for comp in path.components() {
            match comp {
                Component::CurDir => continue,
                Component::ParentDir => return Err("Access denied: path traversal detected".to_string()),
                _ => normalized.push(comp),
            }
        }

        // 4. Canonicalize if file exists
        let final_path = if normalized.exists() {
            normalized.canonicalize().map_err(|e| format!("Canonicalization failed: {}", e))?
        } else {
            normalized
        };

        // 5. Verify path starts within at least one allowed root directory (if allowed_roots specified)
        if !allowed_roots.is_empty() {
            let is_allowed = allowed_roots.iter().any(|root| {
                if let Ok(canonical_root) = root.canonicalize() {
                    final_path.starts_with(canonical_root) || final_path.starts_with(root)
                } else {
                    final_path.starts_with(root)
                }
            });

            if !is_allowed {
                return Err("Access denied: path is outside permitted workspace directory".to_string());
            }
        }

        Ok(final_path)
    }

    /// Sanitizes string inputs for safe filename usage (stripping invalid OS characters)
    pub fn sanitize_filename(name: &str) -> String {
        name.chars()
            .map(|c| match c {
                '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' | '\0' => '_',
                _ => c,
            })
            .collect::<String>()
            .trim()
            .to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_path_traversal_rejection() {
        let bad_path = Path::new("../../etc/passwd");
        let result = PathSecurity::validate_safe_path(bad_path, &[]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("path traversal"));
    }

    #[test]
    fn test_null_byte_rejection() {
        let bad_path = Path::new("samples/kick\0.wav");
        let result = PathSecurity::validate_safe_path(bad_path, &[]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("null byte"));
    }

    #[test]
    fn test_allowed_root_restriction() {
        let temp_dir = env::temp_dir();
        let allowed_root = temp_dir.join("vibe_allowed");
        let _ = std::fs::create_dir_all(&allowed_root);

        let valid_file = allowed_root.join("sample.wav");
        let _ = std::fs::write(&valid_file, b"test");

        let outside_file = temp_dir.join("outside.wav");
        let _ = std::fs::write(&outside_file, b"test");

        let ok_result = PathSecurity::validate_safe_path(&valid_file, &[&allowed_root]);
        assert!(ok_result.is_ok());

        let denied_result = PathSecurity::validate_safe_path(&outside_file, &[&allowed_root]);
        assert!(denied_result.is_err());

        let _ = std::fs::remove_file(valid_file);
        let _ = std::fs::remove_file(outside_file);
        let _ = std::fs::remove_dir(allowed_root);
    }

    #[test]
    fn test_sanitize_filename() {
        let unsafe_name = "Lead Synth: Vocal/Fx* <Demo>?";
        let safe_name = PathSecurity::sanitize_filename(unsafe_name);
        assert_eq!(safe_name, "Lead Synth_ Vocal_Fx_ _Demo__");
    }
}
