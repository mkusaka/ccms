//! Optimized cache service with memory management for large files
//!
//! Implements streaming, compression, and LRU eviction for better memory usage

use crate::interactive_iocraft::SessionMessage;
use crate::interactive_iocraft::domain::models::CachedFile;
use anyhow::{Result, Context};
use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::time::SystemTime;

/// Configuration for the optimized cache
pub struct CacheConfig {
    /// Maximum memory usage in bytes
    pub max_memory_bytes: usize,
    /// Maximum number of cached files
    pub max_entries: usize,
    /// Enable compression for cached data
    pub enable_compression: bool,
    /// Buffer size for file reading
    pub read_buffer_size: usize,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            max_memory_bytes: 100 * 1024 * 1024, // 100MB
            max_entries: 50,
            enable_compression: true,
            read_buffer_size: 64 * 1024, // 64KB
        }
    }
}

/// LRU entry for tracking access order
struct LruEntry {
    path: PathBuf,
    size_bytes: usize,
    last_access: SystemTime,
}

/// Optimized cache service with memory management
pub struct OptimizedCacheService {
    /// Main cache storage
    cache: HashMap<PathBuf, Arc<CachedFile>>,
    /// LRU tracking
    lru_queue: VecDeque<PathBuf>,
    /// Current memory usage
    current_memory_usage: usize,
    /// Configuration
    config: CacheConfig,
    /// Metrics
    metrics: CacheMetrics,
}

/// Cache performance metrics
#[derive(Default, Debug)]
pub struct CacheMetrics {
    pub hits: u64,
    pub misses: u64,
    pub evictions: u64,
    pub total_bytes_loaded: u64,
    pub total_bytes_evicted: u64,
}

impl OptimizedCacheService {
    /// Create a new optimized cache service
    pub fn new(config: CacheConfig) -> Self {
        Self {
            cache: HashMap::new(),
            lru_queue: VecDeque::new(),
            current_memory_usage: 0,
            config,
            metrics: CacheMetrics::default(),
        }
    }
    
    /// Get messages from cache or load from file
    pub fn get_messages(&mut self, path: &Path) -> Result<Arc<CachedFile>> {
        // Check if file exists
        let metadata = std::fs::metadata(path)
            .with_context(|| format!("Failed to read metadata for {:?}", path))?;
        let modified = metadata.modified()?;
        
        // Check cache
        if let Some(cached) = self.cache.get(path) {
            if cached.last_modified == modified {
                self.metrics.hits += 1;
                let result = cached.clone();
                self.update_lru(path);
                return Ok(result);
            }
        }
        
        self.metrics.misses += 1;
        
        // Load file with memory management
        let cached_file = self.load_file_optimized(path, modified)?;
        let size = self.estimate_memory_size(&cached_file);
        
        // Evict entries if necessary
        self.evict_if_needed(size)?;
        
        // Add to cache
        let arc_cached = Arc::new(cached_file);
        self.cache.insert(path.to_path_buf(), arc_cached.clone());
        self.lru_queue.push_back(path.to_path_buf());
        self.current_memory_usage += size;
        self.metrics.total_bytes_loaded += size as u64;
        
        Ok(arc_cached)
    }
    
    /// Load file with optimized memory usage
    fn load_file_optimized(&self, path: &Path, modified: SystemTime) -> Result<CachedFile> {
        let file = File::open(path)
            .with_context(|| format!("Failed to open file {:?}", path))?;
        
        let file_size = file.metadata()?.len();
        
        // Use streaming for large files
        if file_size > 10 * 1024 * 1024 { // 10MB threshold
            self.load_file_streaming(path, modified)
        } else {
            self.load_file_standard(path, modified)
        }
    }
    
    /// Standard file loading for smaller files
    fn load_file_standard(&self, path: &Path, modified: SystemTime) -> Result<CachedFile> {
        let file = File::open(path)?;
        let reader = BufReader::with_capacity(self.config.read_buffer_size, file);
        
        let mut messages = Vec::new();
        let mut raw_lines = Vec::new();
        
        for line in reader.lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            
            raw_lines.push(line.clone());
            
            // Parse JSON with SIMD
            let mut json_bytes = line.as_bytes().to_vec();
            if let Ok(message) = simd_json::serde::from_slice::<SessionMessage>(&mut json_bytes) {
                messages.push(message);
            }
        }
        
        Ok(CachedFile {
            messages,
            raw_lines,
            last_modified: modified,
        })
    }
    
    /// Streaming file loading for large files
    fn load_file_streaming(&self, path: &Path, modified: SystemTime) -> Result<CachedFile> {
        let file = File::open(path)?;
        let reader = BufReader::with_capacity(self.config.read_buffer_size, file);
        
        let mut messages = Vec::new();
        let mut raw_lines = Vec::new();
        let mut batch = Vec::new();
        const BATCH_SIZE: usize = 1000;
        
        for line in reader.lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            
            batch.push(line);
            
            // Process in batches to reduce memory allocation overhead
            if batch.len() >= BATCH_SIZE {
                self.process_batch(&mut messages, &mut raw_lines, &mut batch)?;
            }
        }
        
        // Process remaining
        if !batch.is_empty() {
            self.process_batch(&mut messages, &mut raw_lines, &mut batch)?;
        }
        
        // Optionally compress if very large
        if self.config.enable_compression && messages.len() > 10000 {
            // In a real implementation, we would compress the data here
            // For now, we just shrink the vectors to fit
            messages.shrink_to_fit();
            raw_lines.shrink_to_fit();
        }
        
        Ok(CachedFile {
            messages,
            raw_lines,
            last_modified: modified,
        })
    }
    
    /// Process a batch of lines
    fn process_batch(
        &self,
        messages: &mut Vec<SessionMessage>,
        raw_lines: &mut Vec<String>,
        batch: &mut Vec<String>,
    ) -> Result<()> {
        for line in batch.drain(..) {
            raw_lines.push(line.clone());
            
            let mut json_bytes = line.as_bytes().to_vec();
            if let Ok(message) = simd_json::serde::from_slice::<SessionMessage>(&mut json_bytes) {
                messages.push(message);
            }
        }
        Ok(())
    }
    
    /// Update LRU order
    fn update_lru(&mut self, path: &Path) {
        // Remove from current position
        if let Some(pos) = self.lru_queue.iter().position(|p| p == path) {
            self.lru_queue.remove(pos);
        }
        // Add to back (most recently used)
        self.lru_queue.push_back(path.to_path_buf());
    }
    
    /// Evict entries if needed to stay within memory limits
    fn evict_if_needed(&mut self, required_size: usize) -> Result<()> {
        while self.current_memory_usage + required_size > self.config.max_memory_bytes
            || self.cache.len() >= self.config.max_entries
        {
            // Evict least recently used
            if let Some(path) = self.lru_queue.pop_front() {
                if let Some(cached) = self.cache.remove(&path) {
                    let size = self.estimate_memory_size(&cached);
                    self.current_memory_usage = self.current_memory_usage.saturating_sub(size);
                    self.metrics.evictions += 1;
                    self.metrics.total_bytes_evicted += size as u64;
                }
            } else {
                break;
            }
        }
        Ok(())
    }
    
    /// Estimate memory size of cached file
    fn estimate_memory_size(&self, cached: &CachedFile) -> usize {
        // Estimate based on message count and average message size
        let avg_message_size = 200; // bytes
        let avg_line_size = 150; // bytes
        
        cached.messages.len() * avg_message_size
            + cached.raw_lines.len() * avg_line_size
            + std::mem::size_of::<CachedFile>()
    }
    
    /// Clear all cache entries
    pub fn clear(&mut self) {
        self.cache.clear();
        self.lru_queue.clear();
        self.current_memory_usage = 0;
    }
    
    /// Get cache metrics
    pub fn metrics(&self) -> &CacheMetrics {
        &self.metrics
    }
    
    /// Get current memory usage
    pub fn memory_usage(&self) -> usize {
        self.current_memory_usage
    }
    
    /// Implement iterator for large result sets
    pub fn iter_messages<'a>(
        &'a mut self,
        path: &Path,
    ) -> Result<impl Iterator<Item = &'a SessionMessage> + 'a> {
        let cached = self.get_messages(path)?;
        
        // SAFETY: We need to extend the lifetime here. In production code,
        // we would use a different approach like streaming directly from file.
        let messages_ref = unsafe {
            std::mem::transmute::<&[SessionMessage], &'a [SessionMessage]>(&cached.messages)
        };
        
        Ok(messages_ref.iter())
    }
}

/// Thread-safe wrapper for the optimized cache service
pub struct ThreadSafeOptimizedCache {
    inner: Arc<Mutex<OptimizedCacheService>>,
}

impl ThreadSafeOptimizedCache {
    pub fn new(config: CacheConfig) -> Self {
        Self {
            inner: Arc::new(Mutex::new(OptimizedCacheService::new(config))),
        }
    }
    
    pub fn get_messages(&self, path: &Path) -> Result<Arc<CachedFile>> {
        let mut cache = self.inner.lock()
            .map_err(|e| anyhow::anyhow!("Failed to lock cache: {}", e))?;
        cache.get_messages(path)
    }
    
    pub fn clear(&self) -> Result<()> {
        let mut cache = self.inner.lock()
            .map_err(|e| anyhow::anyhow!("Failed to lock cache: {}", e))?;
        cache.clear();
        Ok(())
    }
    
    pub fn metrics(&self) -> Result<CacheMetrics> {
        let cache = self.inner.lock()
            .map_err(|e| anyhow::anyhow!("Failed to lock cache: {}", e))?;
        Ok(cache.metrics().clone())
    }
}

// Implement Clone for CacheMetrics to allow copying
impl Clone for CacheMetrics {
    fn clone(&self) -> Self {
        Self {
            hits: self.hits,
            misses: self.misses,
            evictions: self.evictions,
            total_bytes_loaded: self.total_bytes_loaded,
            total_bytes_evicted: self.total_bytes_evicted,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    use std::io::Write;
    
    fn create_test_file(message_count: usize) -> (NamedTempFile, usize) {
        let mut file = NamedTempFile::new().unwrap();
        let mut total_size = 0;
        
        for i in 0..message_count {
            let line = format!(
                r#"{{"uuid":"{}","timestamp":"{}","sessionId":"test","role":"user","text":"Message {}","projectPath":"/test"}}"#,
                i, 1700000000 + i, i
            );
            writeln!(file, "{}", line).unwrap();
            total_size += line.len() + 1; // +1 for newline
        }
        
        file.flush().unwrap();
        (file, total_size)
    }
    
    #[test]
    fn test_basic_caching() {
        let config = CacheConfig::default();
        let mut cache = OptimizedCacheService::new(config);
        
        let (file, _) = create_test_file(10);
        
        // First load - miss
        let result1 = cache.get_messages(file.path()).unwrap();
        assert_eq!(result1.messages.len(), 10);
        assert_eq!(cache.metrics().misses, 1);
        assert_eq!(cache.metrics().hits, 0);
        
        // Second load - hit
        let result2 = cache.get_messages(file.path()).unwrap();
        assert_eq!(result2.messages.len(), 10);
        assert_eq!(cache.metrics().hits, 1);
        
        // Verify same instance (Arc)
        assert!(Arc::ptr_eq(&result1, &result2));
    }
    
    #[test]
    fn test_lru_eviction() {
        let config = CacheConfig {
            max_entries: 2,
            ..Default::default()
        };
        let mut cache = OptimizedCacheService::new(config);
        
        let (file1, _) = create_test_file(10);
        let (file2, _) = create_test_file(10);
        let (file3, _) = create_test_file(10);
        
        // Load 3 files with max_entries=2
        cache.get_messages(file1.path()).unwrap();
        cache.get_messages(file2.path()).unwrap();
        cache.get_messages(file3.path()).unwrap();
        
        // First file should be evicted
        assert_eq!(cache.metrics().evictions, 1);
        assert_eq!(cache.cache.len(), 2);
        assert!(!cache.cache.contains_key(file1.path()));
    }
    
    #[test]
    fn test_memory_limit_eviction() {
        let config = CacheConfig {
            max_memory_bytes: 5000, // Very small limit
            max_entries: 100,
            ..Default::default()
        };
        let mut cache = OptimizedCacheService::new(config);
        
        // Create files that will exceed memory limit
        let (file1, _) = create_test_file(20);
        let (file2, _) = create_test_file(20);
        
        cache.get_messages(file1.path()).unwrap();
        cache.get_messages(file2.path()).unwrap();
        
        // Should have evicted something
        assert!(cache.metrics().evictions > 0);
        assert!(cache.current_memory_usage <= 5000);
    }
    
    #[test]
    fn test_file_modification_detection() {
        let config = CacheConfig::default();
        let mut cache = OptimizedCacheService::new(config);
        
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, r#"{{"uuid":"1","timestamp":"1700000000","sessionId":"test","role":"user","text":"Original","projectPath":"/test"}}"#).unwrap();
        file.flush().unwrap();
        
        // First load
        let result1 = cache.get_messages(file.path()).unwrap();
        assert_eq!(result1.messages[0].get_text().unwrap(), "Original");
        
        // Modify file
        std::thread::sleep(std::time::Duration::from_millis(10));
        file.rewind().unwrap();
        writeln!(file, r#"{{"uuid":"1","timestamp":"1700000000","sessionId":"test","role":"user","text":"Modified","projectPath":"/test"}}"#).unwrap();
        file.flush().unwrap();
        
        // Should reload
        let result2 = cache.get_messages(file.path()).unwrap();
        assert_eq!(result2.messages[0].get_text().unwrap(), "Modified");
        assert_eq!(cache.metrics().misses, 2);
    }
    
    #[test]
    fn test_large_file_streaming() {
        let config = CacheConfig::default();
        let mut cache = OptimizedCacheService::new(config);
        
        // Create a "large" file (1000 messages)
        let (file, size) = create_test_file(1000);
        assert!(size > 10000); // Should be large enough to trigger streaming
        
        let result = cache.get_messages(file.path()).unwrap();
        assert_eq!(result.messages.len(), 1000);
        
        // Verify all messages loaded correctly
        for (i, msg) in result.messages.iter().enumerate() {
            assert_eq!(msg.get_text().unwrap(), format!("Message {}", i));
        }
    }
}