use std::sync::mpsc;
use std::thread;
use std::path::PathBuf;

use crate::query::condition::{SearchResult, SearchOptions};
use crate::search::engine::SearchEngine;
use crate::query::parser::parse_query;

/// Service for handling search operations
pub struct SearchService {
    pattern: Option<String>,
    timestamp_gte: Option<String>,
    timestamp_lt: Option<String>,
    session_id: Option<String>,
}

impl SearchService {
    pub fn new() -> Self {
        Self {
            pattern: None,
            timestamp_gte: None,
            timestamp_lt: None,
            session_id: None,
        }
    }
    
    /// Configure search service
    pub fn configure(
        &mut self,
        pattern: Option<String>,
        timestamp_gte: Option<String>,
        timestamp_lt: Option<String>,
        session_id: Option<String>,
    ) {
        self.pattern = pattern;
        self.timestamp_gte = timestamp_gte;
        self.timestamp_lt = timestamp_lt;
        self.session_id = session_id;
    }
    
    /// Execute search asynchronously
    pub fn search_async(
        &self,
        query: String,
        role_filter: Option<String>,
        tx: mpsc::Sender<Vec<SearchResult>>,
    ) {
        let pattern = self.pattern.clone();
        let timestamp_gte = self.timestamp_gte.clone();
        let timestamp_lt = self.timestamp_lt.clone();
        let session_id = self.session_id.clone();
        
        thread::spawn(move || {
            let results = Self::execute_search(
                query,
                role_filter,
                pattern,
                timestamp_gte,
                timestamp_lt,
                session_id,
            );
            
            // Send results back
            let _ = tx.send(results);
        });
    }
    
    /// Execute the actual search
    fn execute_search(
        query: String,
        role_filter: Option<String>,
        pattern: Option<String>,
        timestamp_gte: Option<String>,
        timestamp_lt: Option<String>,
        session_id: Option<String>,
    ) -> Vec<SearchResult> {
        // Parse the query
        let condition = match parse_query(&query) {
            Ok(condition) => condition,
            Err(_) => return vec![],
        };
        
        // Get home directory
        let home_dir = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        let claude_path = home_dir.join(".claude").join("chats");
        
        // Determine glob pattern
        let glob_pattern = pattern
            .clone()
            .or_else(|| {
                // Try to get pattern from environment or use default
                std::env::var("CLAUDE_CHAT_PATTERN").ok()
            })
            .unwrap_or_else(|| {
                // Default pattern
                format!("{}/*.json", claude_path.display())
            });
        
        // Create search engine with options
        let search_options = SearchOptions {
            max_results: None, // No limit
            role: role_filter,
            session_id: session_id.clone(),
            before: timestamp_lt,
            after: timestamp_gte,
            verbose: false,
            project_path: None,
        };
        let engine = SearchEngine::new(search_options);
        
        // Execute search
        let (results, _, _) = engine
            .search(&glob_pattern, condition)
            .unwrap_or_default();
        
        results
    }
    
    /// Execute search synchronously (for testing)
    pub fn search_sync(
        &self,
        query: String,
        role_filter: Option<String>,
    ) -> Vec<SearchResult> {
        Self::execute_search(
            query,
            role_filter,
            self.pattern.clone(),
            self.timestamp_gte.clone(),
            self.timestamp_lt.clone(),
            self.session_id.clone(),
        )
    }
}