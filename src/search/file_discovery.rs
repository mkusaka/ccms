use anyhow::{Context, Result};
use dirs::home_dir;
use globset::{Glob, GlobSet, GlobSetBuilder};
use jwalk::WalkDir;
use std::path::{Path, PathBuf};

pub struct FileDiscovery {
    glob_set: GlobSet,
}

impl FileDiscovery {
    pub fn new(patterns: Vec<String>) -> Result<Self> {
        let mut builder = GlobSetBuilder::new();

        for pattern in patterns {
            let glob =
                Glob::new(&pattern).with_context(|| format!("Invalid glob pattern: {pattern}"))?;
            builder.add(glob);
        }

        let glob_set = builder.build().context("Failed to build glob set")?;

        Ok(Self { glob_set })
    }

    pub fn from_pattern(pattern: &str) -> Result<Self> {
        Self::new(vec![pattern.to_string()])
    }

    pub fn discover_files(&self, base_path: &Path) -> Result<Vec<PathBuf>> {
        // Use jwalk for high-performance parallel file discovery
        let mut files: Vec<PathBuf> = WalkDir::new(base_path)
            .parallelism(jwalk::Parallelism::RayonNewPool(0)) // Use all CPUs
            .follow_links(true)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file() && self.glob_set.is_match(e.path()))
            .map(|e| e.path())
            .collect();

        // Sort by modification time (newest first)
        files.sort_by_cached_key(|path| {
            std::fs::metadata(path)
                .and_then(|m| m.modified())
                .map(std::cmp::Reverse)
                .ok()
        });

        Ok(files)
    }
}

pub fn expand_tilde(path: &str) -> PathBuf {
    if let Some(stripped) = path.strip_prefix("~/") {
        if let Some(home) = home_dir() {
            return home.join(stripped);
        }
    }
    PathBuf::from(path)
}

pub fn default_claude_pattern() -> String {
    "~/.claude/projects/**/*.jsonl".to_string()
}

pub fn discover_claude_files(pattern: Option<&str>) -> Result<Vec<PathBuf>> {
    let default_pattern = default_claude_pattern();
    let pattern = pattern.unwrap_or(&default_pattern);
    let expanded_path = expand_tilde(pattern);

    // Extract base path and glob pattern
    let path_str = expanded_path.to_string_lossy();
    let (base_path, glob_pattern) = if let Some(pos) = path_str.find("**") {
        let base = &path_str[..pos];
        (PathBuf::from(base), path_str.to_string())
    } else if let Some(pos) = path_str.find('*') {
        let base = &path_str[..pos];
        let parent = Path::new(base).parent().unwrap_or(Path::new("/"));
        (parent.to_path_buf(), path_str.to_string())
    } else {
        // No glob pattern, treat as single file
        return Ok(vec![expanded_path]);
    };

    let discovery = FileDiscovery::from_pattern(&glob_pattern)?;
    discovery.discover_files(&base_path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{File, create_dir_all};
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_expand_tilde() {
        let home = home_dir().unwrap();
        assert_eq!(expand_tilde("~/test"), home.join("test"));
        assert_eq!(
            expand_tilde("/absolute/path"),
            PathBuf::from("/absolute/path")
        );
    }

    #[test]
    fn test_file_discovery() -> Result<()> {
        let temp_dir = tempdir()?;
        let base_path = temp_dir.path();

        // Create test directory structure
        create_dir_all(base_path.join("project1"))?;
        create_dir_all(base_path.join("project2/subdir"))?;

        // Create test files
        File::create(base_path.join("project1/session1.jsonl"))?.write_all(b"test")?;
        File::create(base_path.join("project1/session2.jsonl"))?.write_all(b"test")?;
        File::create(base_path.join("project2/session3.jsonl"))?.write_all(b"test")?;
        File::create(base_path.join("project2/subdir/session4.jsonl"))?.write_all(b"test")?;
        File::create(base_path.join("project1/other.txt"))?.write_all(b"test")?;

        // Test discovery with glob pattern
        let pattern = format!("{}/**/*.jsonl", base_path.display());
        let discovery = FileDiscovery::from_pattern(&pattern)?;
        let files = discovery.discover_files(base_path)?;

        assert_eq!(files.len(), 4);

        // Verify all files are .jsonl
        for file in &files {
            assert!(file.to_string_lossy().ends_with(".jsonl"));
        }

        Ok(())
    }

    #[test]
    fn test_multiple_patterns() -> Result<()> {
        let temp_dir = tempdir()?;
        let base_path = temp_dir.path();

        // Create test files
        File::create(base_path.join("test.jsonl"))?;
        File::create(base_path.join("test.json"))?;
        File::create(base_path.join("test.txt"))?;

        // Test with multiple patterns
        let patterns = vec![
            format!("{}/*.jsonl", base_path.display()),
            format!("{}/*.json", base_path.display()),
        ];

        let discovery = FileDiscovery::new(patterns)?;
        let files = discovery.discover_files(base_path)?;

        assert_eq!(files.len(), 2);

        Ok(())
    }
}
