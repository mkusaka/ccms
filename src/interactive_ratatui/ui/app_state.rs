use crate::SearchOptions;
use crate::interactive_ratatui::domain::models::SessionOrder;
use crate::interactive_ratatui::ui::commands::Command;
use crate::interactive_ratatui::ui::events::Message;
use crate::query::condition::{QueryCondition, SearchResult};

// Re-export Mode
pub use crate::interactive_ratatui::domain::models::Mode;

pub struct AppState {
    pub mode: Mode,
    pub mode_stack: Vec<Mode>,
    pub search: SearchState,
    pub session: SessionState,
    pub ui: UiState,
    #[allow(dead_code)]
    pub base_options: SearchOptions,
    #[allow(dead_code)]
    pub max_results: usize,
}

pub struct SearchState {
    pub query: String,
    pub results: Vec<SearchResult>,
    pub selected_index: usize,
    pub scroll_offset: usize,
    pub role_filter: Option<String>,
    pub is_searching: bool,
    pub current_search_id: u64,
}

pub struct SessionState {
    pub messages: Vec<String>,
    pub query: String,
    pub filtered_indices: Vec<usize>,
    pub selected_index: usize,
    pub scroll_offset: usize,
    pub order: Option<SessionOrder>,
    pub file_path: Option<String>,
    pub session_id: Option<String>,
}

pub struct UiState {
    pub message: Option<String>,
    pub detail_scroll_offset: usize,
    pub selected_result: Option<SearchResult>,
    pub truncation_enabled: bool,
}

impl AppState {
    pub fn new(base_options: SearchOptions, max_results: usize) -> Self {
        Self {
            mode: Mode::Search,
            mode_stack: Vec::new(),
            search: SearchState {
                query: String::new(),
                results: Vec::new(),
                selected_index: 0,
                scroll_offset: 0,
                role_filter: None,
                is_searching: false,
                current_search_id: 0,
            },
            session: SessionState {
                messages: Vec::new(),
                query: String::new(),
                filtered_indices: Vec::new(),
                selected_index: 0,
                scroll_offset: 0,
                order: None,
                file_path: None,
                session_id: None,
            },
            ui: UiState {
                message: None,
                detail_scroll_offset: 0,
                selected_result: None,
                truncation_enabled: true,
            },
            base_options,
            max_results,
        }
    }

    pub fn update(&mut self, msg: Message) -> Command {
        match msg {
            Message::QueryChanged(q) => {
                self.search.query = q;
                self.ui.message = Some("typing...".to_string());
                Command::ScheduleSearch(300) // 300ms debounce
            }
            Message::SearchRequested => {
                self.search.is_searching = true;
                self.ui.message = Some("searching...".to_string());
                self.search.current_search_id += 1;
                Command::ExecuteSearch
            }
            Message::SearchCompleted(results) => {
                self.search.results = results;
                self.search.is_searching = false;
                self.search.selected_index = 0;
                self.search.scroll_offset = 0;
                self.ui.message = None;
                Command::None
            }
            Message::SelectResult(index) => {
                if index < self.search.results.len() {
                    self.search.selected_index = index;
                }
                Command::None
            }
            Message::ScrollUp => {
                // Scroll handling is now done within ResultList
                Command::None
            }
            Message::ScrollDown => {
                // Scroll handling is now done within ResultList
                Command::None
            }
            Message::EnterResultDetail => {
                if let Some(result) = self.get_selected_result() {
                    self.ui.selected_result = Some(result.clone());
                    self.ui.detail_scroll_offset = 0;
                    self.mode_stack.push(self.mode);
                    self.mode = Mode::ResultDetail;
                }
                Command::None
            }
            Message::EnterSessionViewer => {
                // Try to get result from selected result (when in detail view) or search results
                let result = if self.mode == Mode::ResultDetail {
                    self.ui.selected_result.as_ref()
                } else {
                    self.search.results.get(self.search.selected_index)
                };

                if let Some(result) = result {
                    let file = result.file.clone();
                    self.mode_stack.push(self.mode);
                    self.mode = Mode::SessionViewer;
                    self.session.file_path = Some(file.clone());
                    self.session.session_id = Some(result.session_id.clone());
                    self.session.query.clear();
                    self.session.selected_index = 0;
                    self.session.scroll_offset = 0;
                    Command::LoadSession(file)
                } else {
                    Command::None
                }
            }
            Message::ExitToSearch => {
                // Pop mode from stack if available, otherwise go to Search
                self.mode = self.mode_stack.pop().unwrap_or(Mode::Search);
                if self.mode == Mode::Search {
                    // Only clear session messages when returning to search
                    self.session.messages.clear();
                }
                self.ui.detail_scroll_offset = 0;
                Command::None
            }
            Message::ShowHelp => {
                self.mode = Mode::Help;
                Command::None
            }
            Message::CloseHelp => {
                self.mode = Mode::Search;
                Command::None
            }
            Message::ToggleRoleFilter => {
                self.search.role_filter = match &self.search.role_filter {
                    None => Some("user".to_string()),
                    Some(r) if r == "user" => Some("assistant".to_string()),
                    Some(r) if r == "assistant" => Some("system".to_string()),
                    _ => None,
                };
                Command::ExecuteSearch
            }
            Message::ToggleTruncation => {
                self.ui.truncation_enabled = !self.ui.truncation_enabled;
                let status = if self.ui.truncation_enabled {
                    "Truncated"
                } else {
                    "Full Text"
                };
                self.ui.message = Some(format!("Message display: {status}"));
                Command::None
            }
            Message::SessionQueryChanged(q) => {
                self.session.query = q;
                self.update_session_filter();
                Command::None
            }
            Message::SessionScrollUp => {
                // Deprecated: Navigation is now handled internally by SessionViewer
                Command::None
            }
            Message::SessionScrollDown => {
                // Deprecated: Navigation is now handled internally by SessionViewer
                Command::None
            }
            Message::SessionNavigated => {
                // Navigation is handled internally by SessionViewer's ListViewer
                Command::None
            }
            Message::ToggleSessionOrder => {
                self.session.order = match self.session.order {
                    None => Some(SessionOrder::Ascending),
                    Some(SessionOrder::Ascending) => Some(SessionOrder::Descending),
                    Some(SessionOrder::Descending) => Some(SessionOrder::Original),
                    Some(SessionOrder::Original) => None,
                };
                // Re-apply filter with new order
                self.update_session_filter();
                Command::None
            }
            Message::SetStatus(msg) => {
                self.ui.message = Some(msg);
                Command::None
            }
            Message::ClearStatus => {
                self.ui.message = None;
                Command::None
            }
            Message::EnterResultDetailFromSession(raw_json, file_path, session_id) => {
                // Parse the raw JSON to create a SearchResult
                if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(&raw_json) {
                    let role = json_value
                        .get("type")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown")
                        .to_string();

                    let timestamp = json_value
                        .get("timestamp")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();

                    let uuid = json_value
                        .get("uuid")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();

                    // Extract content based on message type
                    let content = match role.as_str() {
                        "summary" => json_value
                            .get("summary")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string(),
                        "system" => json_value
                            .get("content")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string(),
                        _ => {
                            // For user and assistant messages
                            if let Some(content) = json_value
                                .get("message")
                                .and_then(|m| m.get("content"))
                                .and_then(|c| c.as_str())
                            {
                                content.to_string()
                            } else if let Some(arr) = json_value
                                .get("message")
                                .and_then(|m| m.get("content"))
                                .and_then(|c| c.as_array())
                            {
                                let texts: Vec<String> = arr
                                    .iter()
                                    .filter_map(|item| {
                                        item.get("text")
                                            .and_then(|t| t.as_str())
                                            .map(|s| s.to_string())
                                    })
                                    .collect();
                                texts.join(" ")
                            } else {
                                String::new()
                            }
                        }
                    };

                    // Create a SearchResult
                    let result = SearchResult {
                        file: file_path,
                        uuid,
                        timestamp,
                        session_id: session_id.unwrap_or_default(),
                        role,
                        text: content, // Store extracted content
                        has_tools: json_value.get("toolResults").is_some(),
                        has_thinking: false, // Not available from session viewer
                        message_type: "message".to_string(),
                        query: QueryCondition::Literal {
                            pattern: String::new(),
                            case_sensitive: false,
                        },
                        project_path: String::new(), // Not available from session viewer
                        raw_json: Some(raw_json),    // Store full JSON
                    };

                    self.ui.selected_result = Some(result);
                    self.ui.detail_scroll_offset = 0;
                    self.mode_stack.push(self.mode);
                    self.mode = Mode::ResultDetail;
                }
                Command::None
            }
            Message::CopyToClipboard(text) => Command::CopyToClipboard(text),
            Message::Quit => {
                Command::None // Handle in main loop
            }
            _ => Command::None,
        }
    }

    fn get_selected_result(&self) -> Option<&SearchResult> {
        self.search.results.get(self.search.selected_index)
    }

    fn update_session_filter(&mut self) {
        use crate::interactive_ratatui::domain::filter::SessionFilter;
        use crate::interactive_ratatui::domain::session_list_item::SessionListItem;

        // Convert raw JSON strings to SessionListItems for search
        let items: Vec<SessionListItem> = self
            .session
            .messages
            .iter()
            .enumerate()
            .filter_map(|(idx, line)| SessionListItem::from_json_line(idx, line))
            .collect();

        self.session.filtered_indices = SessionFilter::filter_messages(&items, &self.session.query);

        // Reset selection if current selection is out of bounds
        if self.session.selected_index >= self.session.filtered_indices.len() {
            self.session.selected_index = 0;
            self.session.scroll_offset = 0;
        }
    }
}
