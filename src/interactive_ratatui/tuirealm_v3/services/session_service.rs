use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::path::PathBuf;

/// Service for handling session operations
pub struct SessionService {
    cache: std::collections::HashMap<String, Vec<String>>,
}

impl SessionService {
    pub fn new() -> Self {
        Self {
            cache: std::collections::HashMap::new(),
        }
    }
    
    /// Load a session by ID
    pub fn load_session(&mut self, session_id: &str) -> anyhow::Result<Vec<String>> {
        // Check cache first
        if let Some(messages) = self.cache.get(session_id) {
            return Ok(messages.clone());
        }
        
        // Find the file containing this session
        let file_path = self.find_session_file(session_id)?;
        
        // Load messages from file
        let messages = self.load_messages_from_file(&file_path, session_id)?;
        
        // Cache the results
        self.cache.insert(session_id.to_string(), messages.clone());
        
        Ok(messages)
    }
    
    /// Find the file containing a specific session
    fn find_session_file(&self, session_id: &str) -> anyhow::Result<PathBuf> {
        // Get home directory
        let home_dir = dirs::home_dir()
            .ok_or_else(|| anyhow::anyhow!("Could not find home directory"))?;
        
        let claude_path = home_dir.join(".claude").join("chats");
        
        // Check if directory exists
        if !claude_path.exists() {
            return Err(anyhow::anyhow!("Claude chats directory not found"));
        }
        
        // Search for files containing this session
        for entry in fs::read_dir(&claude_path)? {
            let entry = entry?;
            let path = entry.path();
            
            // Only check .json files
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                // Quick check if file might contain the session
                if self.file_contains_session(&path, session_id)? {
                    return Ok(path);
                }
            }
        }
        
        Err(anyhow::anyhow!("Session not found: {}", session_id))
    }
    
    /// Check if a file contains a specific session
    fn file_contains_session(&self, path: &PathBuf, session_id: &str) -> anyhow::Result<bool> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        
        for line in reader.lines() {
            let line = line?;
            if line.contains(session_id) {
                return Ok(true);
            }
        }
        
        Ok(false)
    }
    
    /// Load messages from a file for a specific session
    fn load_messages_from_file(&self, path: &PathBuf, session_id: &str) -> anyhow::Result<Vec<String>> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let mut messages = Vec::new();
        let mut in_session = false;
        
        for line in reader.lines() {
            let line = line?;
            
            // Parse the JSON line
            let mut line_copy = line.clone();
            if let Ok(json) = unsafe { simd_json::serde::from_str::<serde_json::Value>(&mut line_copy) } {
                if let Some(sid) = json.get("session_id").and_then(|v| v.as_str()) {
                    if sid == session_id {
                        in_session = true;
                        messages.push(line);
                    } else if in_session {
                        // We've moved to a different session, stop reading
                        break;
                    }
                }
            }
        }
        
        Ok(messages)
    }
    
    /// Clear the cache
    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }
    
    /// Get cache size
    pub fn cache_size(&self) -> usize {
        self.cache.len()
    }
}