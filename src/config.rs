use anyhow::{Context, Result};
use glob::glob;
use std::path::{Path, PathBuf};

const SUPPORTED_EXTENSIONS: &[&str] = &["ts", "tsx", "js", "jsx"];

#[derive(Debug)]
pub struct BiomeFiles {
    pub includes: Vec<String>,
    pub excludes: Vec<String>,
}

/// Parse biome.json content and extract file patterns.
/// Pure function — takes JSON string, returns config struct.
pub fn parse_biome_config(content: &str) -> Result<BiomeFiles> {
    let json: serde_json::Value =
        serde_json::from_str(content).context("Failed to parse biome.json")?;

    let mut includes = Vec::new();
    let mut excludes = Vec::new();

    if let Some(files) = json.get("files") {
        if let Some(include_arr) = files.get("include").and_then(|v| v.as_array()) {
            for item in include_arr {
                if let Some(s) = item.as_str() {
                    if let Some(stripped) = s.strip_prefix('!') {
                        excludes.push(stripped.to_string());
                    } else {
                        includes.push(s.to_string());
                    }
                }
            }
        }
    }

    if includes.is_empty() {
        includes.push("**".to_string());
    }

    Ok(BiomeFiles { includes, excludes })
}

/// Find biome.json by searching current dir then parent dirs.
pub fn find_biome_config(start_dir: &Path) -> Option<PathBuf> {
    let mut dir = start_dir.to_path_buf();
    loop {
        let config_path = dir.join("biome.json");
        if config_path.exists() {
            return Some(config_path);
        }
        let config_path = dir.join("biome.jsonc");
        if config_path.exists() {
            return Some(config_path);
        }
        if !dir.pop() {
            return None;
        }
    }
}

/// Resolve glob patterns from biome config into actual file paths.
/// Only includes files with supported extensions.
pub fn resolve_file_paths(config: &BiomeFiles, base_dir: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();

    for pattern in &config.includes {
        // For each supported extension, expand the glob pattern
        for ext in SUPPORTED_EXTENSIONS {
            let ext_pattern = if pattern.ends_with("**") {
                format!("{}/*.{}", pattern, ext)
            } else if pattern.ends_with('/') || pattern.ends_with("**") {
                format!("{}*.{}", pattern, ext)
            } else {
                // If pattern already has an extension or is specific, use as-is
                pattern.clone()
            };
            let full_pattern = base_dir.join(&ext_pattern).to_string_lossy().to_string();
            for entry in glob(&full_pattern).context("Invalid glob pattern")? {
                if let Ok(path) = entry {
                    if is_supported_file(&path) && !is_excluded(&path, &config.excludes, base_dir)
                    {
                        files.push(path);
                    }
                }
            }
        }
    }

    files.sort();
    files.dedup();
    Ok(files)
}

fn is_supported_file(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| SUPPORTED_EXTENSIONS.contains(&ext))
        .unwrap_or(false)
}

fn is_excluded(path: &Path, excludes: &[String], base_dir: &Path) -> bool {
    for pattern in excludes {
        let full_pattern = base_dir.join(pattern).to_string_lossy().to_string();
        if let Ok(matches) = glob::Pattern::new(&full_pattern) {
            if matches.matches_path(path) {
                return true;
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_basic_config() {
        let json = r#"{
            "files": {
                "include": ["src/**", "lib/**"]
            }
        }"#;
        let config = parse_biome_config(json).unwrap();
        assert_eq!(config.includes, vec!["src/**", "lib/**"]);
        assert!(config.excludes.is_empty());
    }

    #[test]
    fn test_parse_config_with_excludes() {
        let json = r#"{
            "files": {
                "include": ["**", "!dist", "!node_modules"]
            }
        }"#;
        let config = parse_biome_config(json).unwrap();
        assert_eq!(config.includes, vec!["**"]);
        assert_eq!(config.excludes, vec!["dist", "node_modules"]);
    }

    #[test]
    fn test_parse_config_no_files_field() {
        let json = r#"{
            "linter": {}
        }"#;
        let config = parse_biome_config(json).unwrap();
        assert_eq!(config.includes, vec!["**"]);
        assert!(config.excludes.is_empty());
    }

    #[test]
    fn test_parse_empty_config() {
        let json = "{}";
        let config = parse_biome_config(json).unwrap();
        assert_eq!(config.includes, vec!["**"]);
    }
}
