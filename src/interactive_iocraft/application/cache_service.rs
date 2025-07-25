use crate::interactive_iocraft::SessionMessage;
use crate::interactive_iocraft::domain::models::CachedFile;
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
            let reader = std::io::BufReader::with_capacity(32 * 1024, file);
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

        self.files.get(path)
            .ok_or_else(|| anyhow::anyhow!("Cache entry not found for path: {:?}", path))
    }

    #[allow(dead_code)]
    pub fn clear(&mut self) {
        self.files.clear();
    }
    
    // Session cache methods
    pub fn get(&self, path: &str) -> Option<&Vec<String>> {
        self.files.get(Path::new(path)).map(|cached| &cached.raw_lines)
    }
    
    pub fn put(&mut self, path: String, messages: Vec<String>) {
        let cached_file = CachedFile {
            messages: vec![], // We don't parse messages here
            raw_lines: messages,
            last_modified: std::time::SystemTime::now(),
        };
        self.files.insert(PathBuf::from(path), cached_file);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;
    use tempfile::NamedTempFile;
    use std::thread;
    use std::time::Duration;
    
    fn create_test_jsonl() -> (NamedTempFile, Vec<String>) {
        let mut file = NamedTempFile::new().unwrap();
        let lines = vec![
            r#"{"uuid":"123","timestamp":"2023-11-20T10:00:00Z","sessionId":"abc","role":"user","text":"First message","projectPath":"/test"}"#.to_string(),
            r#"{"uuid":"124","timestamp":"2023-11-20T10:01:00Z","sessionId":"abc","role":"assistant","text":"Second message","projectPath":"/test"}"#.to_string(),
            r#"{"uuid":"125","timestamp":"2023-11-20T10:02:00Z","sessionId":"abc","role":"system","text":"Third message","projectPath":"/test"}"#.to_string(),
        ];
        
        for line in &lines {
            writeln!(file, "{}", line).unwrap();
        }
        file.flush().unwrap();
        (file, lines)
    }
    
    #[test]
    fn test_cache_service_new() {
        let cache = CacheService::new();
        assert_eq!(cache.files.len(), 0);
    }
    
    #[test]
    fn test_get_messages_first_load() {
        let (file, lines) = create_test_jsonl();
        let mut cache = CacheService::new();
        
        let result = cache.get_messages(file.path());
        assert!(result.is_ok());
        
        let cached_file = result.unwrap();
        assert_eq!(cached_file.messages.len(), 3);
        assert_eq!(cached_file.raw_lines.len(), 3);
        
        // Verify raw lines match
        for (i, line) in cached_file.raw_lines.iter().enumerate() {
            assert_eq!(line, &lines[i]);
        }
    }
    
    #[test]
    fn test_get_messages_cached() {
        let (file, _) = create_test_jsonl();
        let mut cache = CacheService::new();
        
        // First load
        let result1 = cache.get_messages(file.path());
        assert!(result1.is_ok());
        
        // Second load - should use cache
        let result2 = cache.get_messages(file.path());
        assert!(result2.is_ok());
        
        // Both should return the same data
        assert_eq!(result1.unwrap().messages.len(), result2.unwrap().messages.len());
    }
    
    #[test]
    fn test_get_messages_file_modified() {
        let (mut file, _) = create_test_jsonl();
        let mut cache = CacheService::new();
        
        // First load
        let result1 = cache.get_messages(file.path());
        assert!(result1.is_ok());
        let first_count = result1.unwrap().messages.len();
        
        // Wait a moment to ensure file modification time changes
        thread::sleep(Duration::from_millis(10));
        
        // Modify file
        writeln!(file, r#"{{"uuid":"126","timestamp":"2023-11-20T10:03:00Z","sessionId":"abc","role":"user","text":"Fourth message","projectPath":"/test"}}"#).unwrap();
        file.flush().unwrap();
        
        // Second load - should reload due to modification
        let result2 = cache.get_messages(file.path());
        assert!(result2.is_ok());
        let second_count = result2.unwrap().messages.len();
        
        assert_eq!(second_count, first_count + 1);
    }
    
    #[test]
    fn test_clear() {
        let (file, _) = create_test_jsonl();
        let mut cache = CacheService::new();
        
        // Load file
        let _ = cache.get_messages(file.path());
        assert_eq!(cache.files.len(), 1);
        
        // Clear cache
        cache.clear();
        assert_eq!(cache.files.len(), 0);
    }
    
    #[test]
    fn test_get_put() {
        let mut cache = CacheService::new();
        let path = "/test/path.jsonl";
        let messages = vec!["line1".to_string(), "line2".to_string()];
        
        // Put messages
        cache.put(path.to_string(), messages.clone());
        
        // Get messages
        let result = cache.get(path);
        assert!(result.is_some());
        assert_eq!(result.unwrap(), &messages);
    }
    
    #[test]
    fn test_get_nonexistent() {
        let cache = CacheService::new();
        let result = cache.get("/nonexistent/path.jsonl");
        assert!(result.is_none());
    }
    
    #[test]
    fn test_get_messages_nonexistent_file() {
        let mut cache = CacheService::new();
        let result = cache.get_messages(Path::new("/nonexistent/file.jsonl"));
        assert!(result.is_err());
    }
    
    #[test]
    fn test_empty_lines_skipped() {
        let mut file = NamedTempFile::new().unwrap();
        // Write lines with empty lines in between
        writeln!(file, r#"{{"uuid":"1","timestamp":"2023-11-20T10:00:00Z","sessionId":"abc","role":"user","text":"First","projectPath":"/test"}}"#).unwrap();
        writeln!(file, "").unwrap(); // Empty line
        writeln!(file, "   ").unwrap(); // Whitespace-only line
        writeln!(file, r#"{{"uuid":"2","timestamp":"2023-11-20T10:01:00Z","sessionId":"abc","role":"assistant","text":"Second","projectPath":"/test"}}"#).unwrap();
        file.flush().unwrap();
        
        let mut cache = CacheService::new();
        let result = cache.get_messages(file.path());
        assert!(result.is_ok());
        
        let cached_file = result.unwrap();
        assert_eq!(cached_file.messages.len(), 2); // Only 2 valid messages
        assert_eq!(cached_file.raw_lines.len(), 2); // Only 2 non-empty lines
    }
    
    #[test]
    fn test_invalid_json_skipped() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, r#"{{"uuid":"1","timestamp":"2023-11-20T10:00:00Z","sessionId":"abc","role":"user","text":"Valid","projectPath":"/test"}}"#).unwrap();
        writeln!(file, "invalid json").unwrap();
        writeln!(file, r#"{{"uuid":"2","timestamp":"2023-11-20T10:01:00Z","sessionId":"abc","role":"assistant","text":"Also valid","projectPath":"/test"}}"#).unwrap();
        file.flush().unwrap();
        
        let mut cache = CacheService::new();
        let result = cache.get_messages(file.path());
        assert!(result.is_ok());
        
        let cached_file = result.unwrap();
        assert_eq!(cached_file.messages.len(), 2); // Only valid messages parsed
        assert_eq!(cached_file.raw_lines.len(), 3); // All non-empty lines kept
    }
}
