use crate::interactive_ratatui::domain::models::{SearchRequest, SearchResponse};
use crate::query::condition::{QueryCondition, SearchResult};
use crate::search::SmolEngine;
use crate::search::engine::SearchEngineTrait;
use crate::search::file_discovery::discover_claude_files;
use crate::{SearchOptions, parse_query};
use anyhow::Result;
use std::sync::Arc;

// Type alias for session data: (file_path, session_id, timestamp, message_count, first_message)
type SessionData = (String, String, String, usize, String);

pub struct SearchService {
    engine: Arc<SmolEngine>,
}

impl SearchService {
    pub fn new(options: SearchOptions) -> Self {
        let engine = Arc::new(SmolEngine::new(options));
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

    pub fn get_all_sessions(&self) -> Result<Vec<SessionData>> {
        // Return format: (file_path, session_id, timestamp, message_count, first_message)
        let mut sessions: Vec<SessionData> = Vec::new();

        // Use discover_claude_files to find all session files
        let files = discover_claude_files(None)?;

        // Find all session files
        for path in files {
            // Read first line to get session info
            if let Ok(content) = std::fs::read_to_string(&path) {
                let mut session_id = String::new();
                let mut timestamp = String::new();
                let mut message_count = 0;
                let mut first_message = String::new();

                for line in content.lines() {
                    message_count += 1;

                    // Parse first message to get session info
                    if message_count == 1 {
                        if let Ok(json) = serde_json::from_str::<serde_json::Value>(line) {
                            if let Some(id) = json.get("sessionId").and_then(|v| v.as_str()) {
                                session_id = id.to_string();
                            }
                            if let Some(ts) = json.get("timestamp").and_then(|v| v.as_str()) {
                                timestamp = ts.to_string();
                            }

                            // Get first message content
                            if let Some(msg_type) = json.get("type").and_then(|v| v.as_str()) {
                                match msg_type {
                                    "user" | "assistant" => {
                                        if let Some(content) = json
                                            .get("message")
                                            .and_then(|m| m.get("content"))
                                            .and_then(|c| c.as_str())
                                        {
                                            first_message = content
                                                .chars()
                                                .take(100)
                                                .collect::<String>()
                                                .replace('\n', " ");
                                        } else if let Some(content_array) = json
                                            .get("message")
                                            .and_then(|m| m.get("content"))
                                            .and_then(|c| c.as_array())
                                        {
                                            if let Some(first_item) = content_array.first() {
                                                if let Some(text) =
                                                    first_item.get("text").and_then(|t| t.as_str())
                                                {
                                                    first_message = text
                                                        .chars()
                                                        .take(100)
                                                        .collect::<String>()
                                                        .replace('\n', " ");
                                                }
                                            }
                                        }
                                    }
                                    "summary" => {
                                        if let Some(summary) =
                                            json.get("summary").and_then(|s| s.as_str())
                                        {
                                            first_message = summary
                                                .chars()
                                                .take(100)
                                                .collect::<String>()
                                                .replace('\n', " ");
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                }

                if !session_id.is_empty() {
                    sessions.push((
                        path.to_string_lossy().to_string(),
                        session_id,
                        timestamp,
                        message_count,
                        first_message,
                    ));
                }
            }
        }

        // Sort by timestamp (descending)
        sessions.sort_by(|a, b| b.2.cmp(&a.2)); // Sort by timestamp descending

        Ok(sessions)
    }
}
