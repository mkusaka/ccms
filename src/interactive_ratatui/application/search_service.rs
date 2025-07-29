use crate::interactive_ratatui::domain::models::{SearchRequest, SearchResponse};
use crate::query::condition::{QueryCondition, SearchResult};
use crate::search::engine::SearchEngine;
use crate::{SearchOptions, parse_query};
use anyhow::Result;
use std::sync::Arc;

pub struct SearchService {
    engine: Arc<SearchEngine>,
}

impl SearchService {
    pub fn new(options: SearchOptions) -> Self {
        let engine = Arc::new(SearchEngine::new(options));
        Self { engine }
    }

    pub fn search(&self, request: SearchRequest) -> Result<SearchResponse> {
        let results = self.execute_search(
            &request.query,
            &request.pattern,
            request.role_filter,
            request.order,
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
    ) -> Result<Vec<SearchResult>> {
        let query_condition = if query.trim().is_empty() {
            // Empty query means "match all" - use empty AND condition
            QueryCondition::And { conditions: vec![] }
        } else {
            parse_query(query)?
        };

        let (results, _, _) = self.engine.search_with_role_filter_and_order(
            pattern,
            query_condition,
            role_filter,
            order,
        )?;

        // Results are already sorted by the engine based on the order
        Ok(results)
    }
}
