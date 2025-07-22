use std::sync::Arc;
use anyhow::Result;
use crate::{SearchOptions, parse_query};
use crate::query::condition::SearchResult;
use crate::search::engine::SearchEngine;
use crate::interactive_ratatui::domain::filter::SearchFilter;
use crate::interactive_ratatui::domain::models::{SearchRequest, SearchResponse};

pub struct SearchService {
    engine: Arc<SearchEngine>,
    base_options: SearchOptions,
}

impl SearchService {
    pub fn new(options: SearchOptions) -> Self {
        let engine = Arc::new(SearchEngine::new(options.clone()));
        Self {
            engine,
            base_options: options,
        }
    }

    pub fn search(&self, request: SearchRequest) -> Result<SearchResponse> {
        let mut results = self.execute_search(&request.query, &request.pattern)?;
        
        // Apply filters
        let filter = SearchFilter::new(request.role_filter);
        filter.apply(&mut results)?;
        
        Ok(SearchResponse {
            id: request.id,
            results,
        })
    }

    fn execute_search(&self, query: &str, pattern: &str) -> Result<Vec<SearchResult>> {
        if query.trim().is_empty() {
            return Ok(Vec::new());
        }

        let query_condition = parse_query(query)?;
        let (mut results, _, _) = self.engine.search(pattern, query_condition)?;
        
        // Sort by timestamp descending
        results.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        
        Ok(results)
    }
}