/// Performance optimizations for tui-realm v3 implementation
use std::collections::HashMap;
use std::sync::Arc;
use crate::query::condition::SearchResult;

/// Cached filter results to avoid repeated filtering
pub struct FilterCache {
    cache: HashMap<Option<String>, Vec<usize>>,
    results_hash: u64,
}

impl FilterCache {
    pub fn new() -> Self {
        Self {
            cache: HashMap::new(),
            results_hash: 0,
        }
    }
    
    /// Get filtered indices from cache or compute them
    pub fn get_filtered_indices(
        &mut self,
        results: &[SearchResult],
        role_filter: &Option<String>,
    ) -> Vec<usize> {
        // Simple hash to detect if results have changed
        let new_hash = results.len() as u64;
        if new_hash != self.results_hash {
            self.cache.clear();
            self.results_hash = new_hash;
        }
        
        // Check cache
        if let Some(indices) = self.cache.get(role_filter) {
            return indices.clone();
        }
        
        // Compute and cache
        let indices: Vec<usize> = match role_filter {
            None => (0..results.len()).collect(),
            Some(role) => results
                .iter()
                .enumerate()
                .filter(|(_, r)| r.role == *role)
                .map(|(i, _)| i)
                .collect(),
        };
        
        self.cache.insert(role_filter.clone(), indices.clone());
        indices
    }
}

/// Optimized session search using indexed search
pub struct SessionSearchIndex {
    messages: Vec<String>,
    word_index: HashMap<String, Vec<usize>>,
}

impl SessionSearchIndex {
    pub fn new(messages: Vec<String>) -> Self {
        let mut word_index = HashMap::new();
        
        // Build word index
        for (i, msg) in messages.iter().enumerate() {
            for word in msg.to_lowercase().split_whitespace() {
                word_index
                    .entry(word.to_string())
                    .or_insert_with(Vec::new)
                    .push(i);
            }
        }
        
        Self {
            messages,
            word_index,
        }
    }
    
    /// Search using the word index
    pub fn search(&self, query: &str) -> Vec<usize> {
        if query.is_empty() {
            return (0..self.messages.len()).collect();
        }
        
        let query_lower = query.to_lowercase();
        let words: Vec<&str> = query_lower.split_whitespace().collect();
        
        if words.is_empty() {
            return (0..self.messages.len()).collect();
        }
        
        // For single word queries, use index
        if words.len() == 1 {
            if let Some(indices) = self.word_index.get(words[0]) {
                return indices.clone();
            }
        }
        
        // For multi-word or partial matches, fall back to contains
        self.messages
            .iter()
            .enumerate()
            .filter(|(_, msg)| msg.to_lowercase().contains(&query_lower))
            .map(|(i, _)| i)
            .collect()
    }
}

/// Virtual scrolling window for large datasets
pub struct VirtualScrollWindow<T> {
    items: Arc<Vec<T>>,
    window_size: usize,
    window_start: usize,
}

impl<T: Clone> VirtualScrollWindow<T> {
    pub fn new(items: Vec<T>, window_size: usize) -> Self {
        Self {
            items: Arc::new(items),
            window_size,
            window_start: 0,
        }
    }
    
    /// Get the current window of items
    pub fn get_window(&self) -> Vec<T> {
        let end = (self.window_start + self.window_size).min(self.items.len());
        self.items[self.window_start..end].to_vec()
    }
    
    /// Update window position based on selected index
    pub fn update_window(&mut self, selected_index: usize) {
        // Keep selected item in middle of window when possible
        let half_window = self.window_size / 2;
        
        if selected_index < half_window {
            self.window_start = 0;
        } else if selected_index + half_window >= self.items.len() {
            self.window_start = self.items.len().saturating_sub(self.window_size);
        } else {
            self.window_start = selected_index - half_window;
        }
    }
    
    /// Get total number of items
    pub fn total_items(&self) -> usize {
        self.items.len()
    }
}

/// AttrValue cache to avoid repeated string parsing
pub struct AttrValueCache {
    cache: HashMap<String, serde_json::Value>,
}

impl AttrValueCache {
    pub fn new() -> Self {
        Self {
            cache: HashMap::new(),
        }
    }
    
    /// Get parsed value from cache or parse and cache it
    pub fn get_or_parse(&mut self, key: &str, value: &str) -> Result<serde_json::Value, serde_json::Error> {
        let cache_key = format!("{}-{}", key, value);
        
        if let Some(cached) = self.cache.get(&cache_key) {
            return Ok(cached.clone());
        }
        
        match serde_json::from_str(value) {
            Ok(parsed) => {
                self.cache.insert(cache_key, parsed.clone());
                Ok(parsed)
            }
            Err(e) => Err(e),
        }
    }
    
    /// Clear cache when data changes
    pub fn clear(&mut self) {
        self.cache.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    fn create_test_results() -> Vec<SearchResult> {
        vec![
            SearchResult {
                role: "User".to_string(),
                text: "Hello world".to_string(),
                ..Default::default()
            },
            SearchResult {
                role: "Assistant".to_string(),
                text: "Hi there".to_string(),
                ..Default::default()
            },
            SearchResult {
                role: "User".to_string(),
                text: "How are you?".to_string(),
                ..Default::default()
            },
        ]
    }
    
    #[test]
    fn test_filter_cache() {
        let mut cache = FilterCache::new();
        let results = create_test_results();
        
        // First call should compute
        let indices1 = cache.get_filtered_indices(&results, &Some("User".to_string()));
        assert_eq!(indices1, vec![0, 2]);
        
        // Second call should use cache
        let indices2 = cache.get_filtered_indices(&results, &Some("User".to_string()));
        assert_eq!(indices2, vec![0, 2]);
        
        // Different filter should compute new
        let indices3 = cache.get_filtered_indices(&results, &Some("Assistant".to_string()));
        assert_eq!(indices3, vec![1]);
    }
    
    #[test]
    fn test_session_search_index() {
        let messages = vec![
            "Hello world".to_string(),
            "Testing search functionality".to_string(),
            "Hello again".to_string(),
        ];
        
        let index = SessionSearchIndex::new(messages);
        
        // Single word search
        let results = index.search("hello");
        assert_eq!(results, vec![0, 2]);
        
        // Multi-word search
        let results = index.search("search functionality");
        assert_eq!(results, vec![1]);
        
        // Empty search
        let results = index.search("");
        assert_eq!(results, vec![0, 1, 2]);
    }
    
    #[test]
    fn test_virtual_scroll_window() {
        let items: Vec<i32> = (0..100).collect();
        let mut window = VirtualScrollWindow::new(items, 10);
        
        // Initial window
        let view = window.get_window();
        assert_eq!(view, vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);
        
        // Update to middle
        window.update_window(50);
        let view = window.get_window();
        assert_eq!(view[0], 45); // 50 - window_size/2
        assert_eq!(view.len(), 10);
        
        // Update to end
        window.update_window(95);
        let view = window.get_window();
        assert_eq!(view[0], 90); // 100 - window_size
        assert_eq!(view[9], 99);
    }
}