use std::sync::mpsc;
use std::thread;
use std::path::PathBuf;
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};

use crate::query::condition::{SearchResult, SearchOptions};
use crate::search::engine::SearchEngine;
use crate::query::parser::parse_query;
use crate::interactive_ratatui::tuirealm_v3::error::{AppError, AppResult};

/// Service for handling search operations
pub struct SearchService {
    pub pattern: Option<String>,
    pub timestamp_gte: Option<String>,
    pub timestamp_lt: Option<String>,
    pub session_id: Option<String>,
    cancel_token: Arc<AtomicBool>,
}

impl SearchService {
    pub fn new() -> Self {
        Self {
            pattern: None,
            timestamp_gte: None,
            timestamp_lt: None,
            session_id: None,
            cancel_token: Arc::new(AtomicBool::new(false)),
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
        &mut self,
        query: String,
        role_filter: Option<String>,
        tx: mpsc::Sender<Vec<SearchResult>>,
    ) {
        // Cancel any previous search
        self.cancel_token.store(true, Ordering::Relaxed);
        
        // Create a new cancel token for this search
        let cancel_token = Arc::new(AtomicBool::new(false));
        self.cancel_token.clone_from(&cancel_token);
        
        let pattern = self.pattern.clone();
        let timestamp_gte = self.timestamp_gte.clone();
        let timestamp_lt = self.timestamp_lt.clone();
        let session_id = self.session_id.clone();
        
        thread::spawn(move || {
            // Check if cancelled before starting
            if cancel_token.load(Ordering::Relaxed) {
                return;
            }
            
            let results = Self::execute_search(
                query,
                role_filter,
                pattern,
                timestamp_gte,
                timestamp_lt,
                session_id,
            ).unwrap_or_else(|e| {
                eprintln!("Search error: {e}");
                vec![]
            });
            
            // Only send results if not cancelled
            if !cancel_token.load(Ordering::Relaxed) {
                let _ = tx.send(results);
            }
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
    ) -> AppResult<Vec<SearchResult>> {
        // Parse the query
        let condition = parse_query(&query).map_err(|e| AppError::InvalidQueryError {
            query: query.clone(),
            details: e.to_string(),
        })?;
        
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
            max_results: Some(1000), // Limit results to prevent memory issues
            role: role_filter,
            session_id: session_id.clone(),
            before: timestamp_lt,
            after: timestamp_gte,
            verbose: false,
            project_path: None,
        };
        let engine = SearchEngine::new(search_options);
        
        // Execute search
        engine.search(&glob_pattern, condition)
            .map(|(results, _, _)| results)
            .map_err(|e| AppError::SearchServiceError {
                details: e.to_string(),
            })
    }
    
    /// Execute search synchronously (for testing)
    #[cfg(test)]
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
        ).unwrap_or_else(|e| {
            eprintln!("Search sync error: {e}");
            vec![]
        })
    }
}
#[cfg(test)]
#[path = "search_service_test.rs"]
mod tests;
