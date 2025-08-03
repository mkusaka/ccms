use crate::interactive_ratatui::domain::models::{SearchRequest, SearchResponse};
use crate::query::condition::{QueryCondition, SearchResult};
use crate::search::SmolEngine;
use crate::search::engine::SearchEngineTrait;
use crate::search::file_discovery::discover_claude_files;
use crate::utils::path_encoding::encode_project_path;
use crate::{SearchOptions, parse_query};
use anyhow::Result;

// Type alias for session data: (file_path, session_id, timestamp, message_count, first_message, preview_messages, summary)
pub type SessionData = (
    String,
    String,
    String,
    usize,
    String,
    Vec<(String, String)>,
    Option<String>,
);

pub struct SearchService {
    base_options: SearchOptions,
}

impl SearchService {
    pub fn new(options: SearchOptions) -> Self {
        Self {
            base_options: options,
        }
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
    pub fn search_session(
        &self,
        request: SearchRequest,
        session_id: String,
    ) -> Result<SearchResponse> {
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

        // Debug log
        let _ = crate::interactive_ratatui::debug::write_debug_log(&format!(
            "execute_search: base_options.max_results = {:?}, session_id = {:?}",
            self.base_options.max_results, session_id
        ));

        if let Some(sid) = session_id {
            options.session_id = Some(sid);
            // For session viewer, show all messages without limit
            options.max_results = None;
            let _ = crate::interactive_ratatui::debug::write_debug_log(
                "execute_search: Setting max_results to None for session viewer",
            );
        }

        // Create a new engine with the updated options
        let engine = SmolEngine::new(options);

        let (results, _, _) = engine.search_with_role_filter_and_order(
            pattern,
            query_condition,
            role_filter,
            order,
        )?;

        let _ = crate::interactive_ratatui::debug::write_debug_log(&format!(
            "execute_search: Returned {} results",
            results.len()
        ));

        // Results are already sorted by the engine based on the order
        Ok(results)
    }

    pub fn get_all_sessions(&self) -> Result<Vec<SessionData>> {
        // Return format: (file_path, session_id, timestamp, message_count, first_message)
        let mut sessions: Vec<SessionData> = Vec::new();

        // Use discover_claude_files to find all session files
        let files = if let Some(ref project_path) = self.base_options.project_path {
            // When project_path is specified, look for Claude sessions for that project
            // Convert the project path to Claude's project directory format
            let encoded_path = encode_project_path(project_path);

            let claude_project_dir = format!("~/.claude/projects/{encoded_path}/**/*.jsonl");

            discover_claude_files(Some(&claude_project_dir))?
        } else {
            // Use default pattern to find all sessions
            discover_claude_files(None)?
        };

        // Find all session files
        for path in files {
            // Read first line to get session info
            if let Ok(content) = std::fs::read_to_string(&path) {
                let mut session_id = String::new();
                let mut timestamp = String::new();
                let mut message_count = 0;
                let mut first_message = String::new();
                let mut preview_messages: Vec<(String, String)> = Vec::new();
                let mut summary_message: Option<String> = None;
                const MAX_PREVIEW_MESSAGES: usize = 5;

                for line in content.lines() {
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(line) {
                        message_count += 1;

                        // First message - get session info
                        if message_count == 1 {
                            if let Some(id) = json.get("sessionId").and_then(|v| v.as_str()) {
                                session_id = id.to_string();
                            }
                            if let Some(ts) = json.get("timestamp").and_then(|v| v.as_str()) {
                                timestamp = ts.to_string();
                            }
                        }

                        // Process all messages for preview
                        if let Some(msg_type) = json.get("type").and_then(|v| v.as_str()) {
                            match msg_type {
                                "user" | "assistant" => {
                                    let mut content = String::new();

                                    // Extract content
                                    if let Some(msg_content) = json
                                        .get("message")
                                        .and_then(|m| m.get("content"))
                                        .and_then(|c| c.as_str())
                                    {
                                        content = msg_content
                                            .chars()
                                            .take(200)
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
                                                content = text
                                                    .chars()
                                                    .take(200)
                                                    .collect::<String>()
                                                    .replace('\n', " ");
                                            }
                                        }
                                    }

                                    // Set first message if not already set
                                    if first_message.is_empty()
                                        && msg_type == "user"
                                        && !content.is_empty()
                                    {
                                        first_message = content.clone();
                                    }

                                    // Collect preview messages
                                    if preview_messages.len() < MAX_PREVIEW_MESSAGES
                                        && !content.is_empty()
                                    {
                                        preview_messages.push((msg_type.to_string(), content));
                                    }
                                }
                                "summary" => {
                                    if let Some(summary) =
                                        json.get("summary").and_then(|s| s.as_str())
                                    {
                                        summary_message = Some(
                                            summary
                                                .chars()
                                                .take(200)
                                                .collect::<String>()
                                                .replace('\n', " "),
                                        );

                                        if first_message.is_empty() {
                                            first_message =
                                                summary_message.clone().unwrap_or_default();
                                        }
                                    }
                                }
                                _ => {}
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
                        preview_messages,
                        summary_message,
                    ));
                }
            }
        }

        // Sort by timestamp (descending)
        sessions.sort_by(|a, b| b.2.cmp(&a.2)); // Sort by timestamp descending

        Ok(sessions)
    }
}
