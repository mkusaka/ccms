use crate::query::condition::SearchResult;
use super::models::SessionOrder;
use super::messages::AppMode;
use std::time::Instant;

#[cfg(test)]
#[path = "state_test.rs"]
mod tests;

/// Central application state - single source of truth
#[derive(Debug, Clone)]
pub struct AppState {
    // Application mode
    pub mode: AppMode,
    pub previous_mode: Option<AppMode>,
    
    // Search state
    pub search_query: String,
    pub search_results: Vec<SearchResult>,
    pub is_searching: bool,
    pub role_filter: Option<String>,
    
    // Search debouncing
    pub last_search_update: Option<Instant>,
    pub pending_search_query: Option<String>,
    
    // UI state
    pub selected_index: usize,
    pub status_message: Option<String>,
    pub truncation_enabled: bool,
    
    // Result detail state
    pub current_result: Option<SearchResult>,
    pub detail_scroll_offset: usize,
    
    // Session viewer state
    pub session_id: Option<String>,
    pub session_messages: Vec<String>,
    pub session_query: String,
    pub session_filtered_indices: Vec<usize>,
    pub session_order: Option<SessionOrder>,
    pub session_scroll_offset: usize,
    pub is_session_searching: bool,
    
    // Application control
    pub should_quit: bool,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            mode: AppMode::Search,
            previous_mode: None,
            
            search_query: String::new(),
            search_results: Vec::new(),
            is_searching: false,
            role_filter: None,
            
            last_search_update: None,
            pending_search_query: None,
            
            selected_index: 0,
            status_message: None,
            truncation_enabled: true,
            
            current_result: None,
            detail_scroll_offset: 0,
            
            session_id: None,
            session_messages: Vec::new(),
            session_query: String::new(),
            session_filtered_indices: Vec::new(),
            session_order: None,
            session_scroll_offset: 0,
            is_session_searching: false,
            
            should_quit: false,
        }
    }
}

impl AppState {
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Clear status message
    pub fn clear_message(&mut self) {
        self.status_message = None;
    }
    
    /// Set status message
    pub fn set_message(&mut self, message: String) {
        self.status_message = Some(message);
    }
    
    /// Change application mode
    pub fn change_mode(&mut self, new_mode: AppMode) {
        if self.mode != new_mode {
            self.previous_mode = Some(self.mode);
            self.mode = new_mode;
        }
    }
    
    /// Return to previous mode
    pub fn return_to_previous_mode(&mut self) {
        if let Some(prev) = self.previous_mode {
            self.mode = prev;
            self.previous_mode = None;
        }
    }
    
    /// Cycle through role filters
    pub fn cycle_role_filter(&mut self) {
        self.role_filter = match self.role_filter.as_deref() {
            None => Some("User".to_string()),
            Some("User") => Some("Assistant".to_string()),
            Some("Assistant") => Some("System".to_string()),
            Some("System") => None,
            _ => None,
        };
    }
}