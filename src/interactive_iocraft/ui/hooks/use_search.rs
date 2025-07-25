//! Search functionality hook

use crate::interactive_iocraft::application::SearchService;
use crate::interactive_iocraft::domain::models::SearchRequest;
use crate::interactive_iocraft::SearchResult;
use iocraft::prelude::*;
use std::sync::Arc;

pub struct UseSearchResult {
    pub results: Vec<SearchResult>,
    pub loading: bool,
    pub error: Option<String>,
}

/// Hook that manages search functionality with debouncing
pub fn use_search(
    hooks: &mut Hooks,
    query: &str,
    pattern: &str,
    role_filter: Option<String>,
) -> UseSearchResult {
    let search_service = hooks.use_context::<Arc<SearchService>>().clone();
    
    let results = hooks.use_state(Vec::<SearchResult>::new);
    let loading = hooks.use_state(|| false);
    let error = hooks.use_state(|| None::<String>);
    
    // Debounce the search query
    // TODO: Fix debouncing to work correctly with initial values
    let debounced_query = query.to_string();
    
    // Track if this is the first run
    let is_first_run = hooks.use_state(|| true);
    
    // Track previous query to detect changes
    let previous_query = hooks.use_state(|| String::new());
    let previous_pattern = hooks.use_state(|| String::new());
    let previous_role_filter = hooks.use_state(|| None::<String>);
    
    // Check if we need to run a search
    let should_search = *is_first_run.read() || 
        *previous_query.read() != debounced_query ||
        *previous_pattern.read() != pattern ||
        *previous_role_filter.read() != role_filter;
    
    // Run search when parameters change
    hooks.use_future({
        let search_service = search_service.clone();
        let mut results = results.clone();
        let mut loading = loading.clone();
        let mut error = error.clone();
        let query = debounced_query.clone();
        let pattern = pattern.to_string();
        let role_filter = role_filter.clone();
        let mut is_first_run = is_first_run.clone();
        let mut previous_query = previous_query.clone();
        let mut previous_pattern = previous_pattern.clone();
        let mut previous_role_filter = previous_role_filter.clone();
        
        async move {
            if !should_search {
                return;
            }
            
            // Update tracking state
            is_first_run.set(false);
            previous_query.set(query.clone());
            previous_pattern.set(pattern.clone());
            previous_role_filter.set(role_filter.clone());
            
            // Clear previous error
            error.set(None);
            
            // Set loading state
            loading.set(true);
            
            let request = SearchRequest {
                id: 0, // Simple incrementing ID for now
                query: query.clone(),
                pattern,
                role_filter,
            };
            
            match search_service.search(request) {
                Ok(response) => {
                    results.set(response.results);
                }
                Err(e) => {
                    error.set(Some(format!("Search error: {}", e)));
                    results.set(Vec::new());
                }
            }
            
            loading.set(false);
        }
    });
    
    UseSearchResult {
        results: results.read().clone(),
        loading: *loading.read(),
        error: error.read().clone(),
    }
}