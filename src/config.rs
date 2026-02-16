use anyhow::{Context, Result};
use globset::{Glob, GlobSetBuilder};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

const SUPPORTED_EXTENSIONS: &[&str] = &["ts", "tsx", "js", "jsx"];

/// Directories that Biome ignores by default.
const DEFAULT_IGNORE: &[&str] = &["node_modules", ".git"];

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
    let mut excludes: Vec<String> = DEFAULT_IGNORE.iter().map(|s| s.to_string()).collect();

    if let Some(files) = json.get("files") {
        // Biome supports both "include" and "includes"
        let include_arr = files
            .get("includes")
            .or_else(|| files.get("include"))
            .and_then(|v| v.as_array());
        if let Some(include_arr) = include_arr {
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

        // Biome uses "ignore" for exclude patterns
        if let Some(ignore_arr) = files.get("ignore").and_then(|v| v.as_array()) {
            for item in ignore_arr {
                if let Some(s) = item.as_str() {
                    excludes.push(s.to_string());
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

/// Resolve file paths by walking the directory tree.
/// Skips excluded directories entirely (never enters node_modules, .git, etc).
/// Only returns files with supported extensions that match include patterns.
pub fn resolve_file_paths(config: &BiomeFiles, base_dir: &Path) -> Result<Vec<PathBuf>> {
    // Build include glob set
    let mut include_builder = GlobSetBuilder::new();
    for pattern in &config.includes {
        for ext in SUPPORTED_EXTENSIONS {
            let glob_pattern = if pattern.ends_with("**") {
                format!("{}/*.{}", pattern, ext)
            } else if pattern.ends_with('/') {
                format!("{}**/*.{}", pattern, ext)
            } else {
                // Pattern already has an extension or is specific — use as-is
                pattern.clone()
            };
            include_builder.add(
                Glob::new(&glob_pattern)
                    .with_context(|| format!("Invalid include pattern: {}", glob_pattern))?,
            );
        }
    }
    let include_set = include_builder
        .build()
        .context("Failed to build include glob set")?;

    let mut files = Vec::new();
    let excludes = &config.excludes;

    let walker = WalkDir::new(base_dir)
        .follow_links(false)
        .into_iter()
        .filter_entry(|entry| {
            // Skip excluded directories entirely (don't descend into them)
            if entry.file_type().is_dir() {
                let dir_name = entry.file_name().to_string_lossy();
                return !excludes.iter().any(|ex| dir_name == *ex);
            }
            true
        });

    for entry in walker {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };

        // Only process regular files
        if !entry.file_type().is_file() {
            continue;
        }

        let path = entry.path();

        // Check supported extension
        if !is_supported_file(path) {
            continue;
        }

        // Check matches include pattern (relative to base_dir)
        let rel_path = path.strip_prefix(base_dir).unwrap_or(path);
        if include_set.is_match(rel_path) {
            files.push(path.to_path_buf());
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
        // Default ignores are always present
        assert!(config.excludes.contains(&"node_modules".to_string()));
        assert!(config.excludes.contains(&".git".to_string()));
    }

    #[test]
    fn test_parse_config_with_excludes() {
        let json = r#"{
            "files": {
                "include": ["**", "!dist"],
                "ignore": ["build"]
            }
        }"#;
        let config = parse_biome_config(json).unwrap();
        assert_eq!(config.includes, vec!["**"]);
        assert!(config.excludes.contains(&"node_modules".to_string()));
        assert!(config.excludes.contains(&"dist".to_string()));
        assert!(config.excludes.contains(&"build".to_string()));
    }

    #[test]
    fn test_parse_config_no_files_field() {
        let json = r#"{
            "linter": {}
        }"#;
        let config = parse_biome_config(json).unwrap();
        assert_eq!(config.includes, vec!["**"]);
        assert!(config.excludes.contains(&"node_modules".to_string()));
    }

    #[test]
    fn test_parse_empty_config() {
        let json = "{}";
        let config = parse_biome_config(json).unwrap();
        assert_eq!(config.includes, vec!["**"]);
    }

    #[test]
    fn test_parse_config_with_includes_plural() {
        let json = r#"{
            "files": {
                "includes": ["**", "!dist"]
            }
        }"#;
        let config = parse_biome_config(json).unwrap();
        assert_eq!(config.includes, vec!["**"]);
        assert!(config.excludes.contains(&"dist".to_string()));
    }

}
