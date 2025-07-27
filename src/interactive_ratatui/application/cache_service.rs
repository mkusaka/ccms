use crate::SessionMessage;
use crate::interactive_ratatui::constants::*;
use crate::interactive_ratatui::domain::models::CachedFile;
use anyhow::Result;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub struct CacheService {
    files: HashMap<PathBuf, CachedFile>,
}

impl CacheService {
    pub fn new() -> Self {
        Self {
            files: HashMap::new(),
        }
    }

    pub fn get_messages(&mut self, path: &Path) -> Result<&CachedFile> {
        let metadata = std::fs::metadata(path)?;
        let modified = metadata.modified()?;

        let needs_reload = match self.files.get(path) {
            Some(cached) => cached.last_modified != modified,
            None => true,
        };

        if needs_reload {
            let file = std::fs::File::open(path)?;
            let reader = std::io::BufReader::with_capacity(FILE_READ_BUFFER_SIZE, file);
            use std::io::BufRead;

            let mut messages = Vec::new();
            let mut raw_lines = Vec::new();

            for line in reader.lines() {
                let line = line?;
                if line.trim().is_empty() {
                    continue;
                }

                raw_lines.push(line.clone());

                let mut json_bytes = line.as_bytes().to_vec();
                if let Ok(message) = simd_json::serde::from_slice::<SessionMessage>(&mut json_bytes)
                {
                    messages.push(message);
                }
            }

            self.files.insert(
                path.to_path_buf(),
                CachedFile {
                    messages,
                    raw_lines,
                    last_modified: modified,
                },
            );
        }

        Ok(self.files.get(path).unwrap())
    }

}
