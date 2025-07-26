use anyhow::Result;
use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::Mutex;

use crate::{SearchOptions, schemas::SessionMessage, search::SearchEngine};

pub struct SearchService {
    options: SearchOptions,
    file_pattern: String,
    cache: Arc<Mutex<HashMap<String, Vec<SessionMessage>>>>,
}

impl SearchService {
    pub fn new(options: SearchOptions, file_pattern: String) -> Self {
        Self {
            options,
            file_pattern,
            cache: Arc::new(Mutex::new(HashMap::new())),
        }
    }
    
    pub async fn search(&self, query: &str) -> Vec<SessionMessage> {
        // Check cache first
        let cache_key = format!("{}-{:?}", query, self.options);
        let cache = self.cache.lock().await;
        if let Some(cached_results) = cache.get(&cache_key) {
            return cached_results.clone();
        }
        drop(cache);
        
        // Perform search using the existing search engine
        let engine = SearchEngine::new(self.options.clone());
        
        // Parse the query
        let query_condition = match crate::parse_query(query) {
            Ok(q) => q,
            Err(e) => {
                eprintln!("Query parse error: {}", e);
                return vec![];
            }
        };
        
        let results = match engine.search(&self.file_pattern, query_condition) {
            Ok((search_results, _, _)) => {
                // Convert SearchResult to SessionMessage
                search_results.into_iter()
                    .filter_map(|sr| sr.message)
                    .collect()
            },
            Err(e) => {
                eprintln!("Search error: {}", e);
                vec![]
            }
        };
        
        // Cache results
        let mut cache = self.cache.lock().await;
        cache.insert(cache_key, results.clone());
        
        results
    }
    
    pub async fn load_session(&self, session_path: &str) -> Vec<SessionMessage> {
        // Check cache first
        let cache = self.cache.lock().await;
        if let Some(cached_messages) = cache.get(session_path) {
            return cached_messages.clone();
        }
        drop(cache);
        
        // Load session using a search with empty query and specific file pattern
        let engine = SearchEngine::new(SearchOptions {
            max_results: None,
            ..self.options.clone()
        });
        
        // Use empty query condition to get all messages
        let query_condition = crate::parse_query("").unwrap_or(crate::query::QueryCondition::Literal("".to_string()));
        
        let messages = match engine.search(session_path, query_condition) {
            Ok((search_results, _, _)) => {
                // Convert SearchResult to SessionMessage
                search_results.into_iter()
                    .filter_map(|sr| sr.message)
                    .collect()
            },
            Err(e) => {
                eprintln!("Failed to load session: {}", e);
                vec![]
            }
        };
        
        // Cache messages
        let mut cache = self.cache.lock().await;
        cache.insert(session_path.to_string(), messages.clone());
        
        messages
    }
}