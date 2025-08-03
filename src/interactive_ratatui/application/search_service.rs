use crate::interactive_ratatui::domain::models::{SearchRequest, SearchResponse};
use crate::query::condition::{QueryCondition, SearchResult};
use crate::search::SmolEngine;
use crate::search::engine::SearchEngineTrait;
use crate::{SearchOptions, parse_query};
use anyhow::Result;

pub struct SearchService {
    base_options: SearchOptions,
}

impl SearchService {
    pub fn new(options: SearchOptions) -> Self {
        Self { base_options: options }
    }

    pub fn search(&self, request: SearchRequest) -> Result<SearchResponse> {
        let results = self.execute_search(
            &request.query,
            &request.pattern,
            request.role_filter,
            request.order,
            None, // No session_id filter for general search
        )?;

        Ok(SearchResponse {
            id: request.id,
            results,
        })
    }

    // New method for session-specific search
    pub fn search_session(&self, request: SearchRequest, session_id: String) -> Result<SearchResponse> {
        let results = self.execute_search(
            &request.query,
            &request.pattern,
            request.role_filter,
            request.order,
            Some(session_id),
        )?;

        Ok(SearchResponse {
            id: request.id,
            results,
        })
    }

    fn execute_search(
        &self,
        query: &str,
        pattern: &str,
        role_filter: Option<String>,
        order: crate::interactive_ratatui::domain::models::SearchOrder,
        session_id: Option<String>,
    ) -> Result<Vec<SearchResult>> {
        let query_condition = if query.trim().is_empty() {
            // Empty query means "match all" - use empty AND condition
            QueryCondition::And { conditions: vec![] }
        } else {
            parse_query(query)?
        };

        // Create a new options with session_id if provided
        let mut options = self.base_options.clone();
        if let Some(sid) = session_id {
            options.session_id = Some(sid);
            // For session viewer, show all messages without limit
            options.max_results = None;
        }

        // Create a new engine with the updated options
        let engine = SmolEngine::new(options);

        let (results, _, _) = engine.search_with_role_filter_and_order(
            pattern,
            query_condition,
            role_filter,
            order,
        )?;

        // Results are already sorted by the engine based on the order
        Ok(results)
    }
}
