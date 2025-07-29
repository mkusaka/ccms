use crate::interactive_ratatui::constants::*;
use crate::interactive_ratatui::domain::models::{SearchOrder, SessionOrder};
use crate::interactive_ratatui::ui::commands::Command;
use crate::interactive_ratatui::ui::events::Message;
use crate::interactive_ratatui::ui::navigation::{
    NavigationHistory, NavigationState, SearchStateSnapshot, SessionStateSnapshot, UiStateSnapshot,
};
use crate::query::condition::{QueryCondition, SearchResult};

// Re-export Mode
pub use crate::interactive_ratatui::domain::models::Mode;

pub struct AppState {
    pub mode: Mode,
    pub navigation_history: NavigationHistory,
    pub search: SearchState,
    pub session: SessionState,
    pub ui: UiState,
}

pub struct SearchState {
    pub query: String,
    pub results: Vec<SearchResult>,
    pub selected_index: usize,
    pub scroll_offset: usize,
    pub role_filter: Option<String>,
    pub is_searching: bool,
    pub current_search_id: u64,
    pub order: SearchOrder,
}

pub struct SessionState {
    pub messages: Vec<String>,
    pub query: String,
    pub filtered_indices: Vec<usize>,
    pub selected_index: usize,
    pub scroll_offset: usize,
    pub order: SessionOrder,
    pub file_path: Option<String>,
    pub session_id: Option<String>,
    pub role_filter: Option<String>,
}

pub struct UiState {
    pub message: Option<String>,
    pub detail_scroll_offset: usize,
    pub selected_result: Option<SearchResult>,
    pub truncation_enabled: bool,
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

impl AppState {
    pub fn new() -> Self {
        Self {
            mode: Mode::Search,
            navigation_history: NavigationHistory::new(MAX_NAVIGATION_HISTORY),
            search: SearchState {
                query: String::new(),
                results: Vec::new(),
                selected_index: 0,
                scroll_offset: 0,
                role_filter: None,
                is_searching: false,
                current_search_id: 0,
                order: SearchOrder::Descending,
            },
            session: SessionState {
                messages: Vec::new(),
                query: String::new(),
                filtered_indices: Vec::new(),
                selected_index: 0,
                scroll_offset: 0,
                order: SessionOrder::Ascending,
                file_path: None,
                session_id: None,
                role_filter: None,
            },
            ui: UiState {
                message: None,
                detail_scroll_offset: 0,
                selected_result: None,
                truncation_enabled: true,
            },
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
                // Results are already sorted by the search engine based on current order
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
            Message::EnterMessageDetail => {
                if let Some(result) = self.search.results.get(self.search.selected_index).cloned() {
                    // Only save state if we're actually changing modes
                    if self.mode != Mode::MessageDetail {
                        // If this is our first navigation, save the initial state
                        if self.navigation_history.is_empty() {
                            let initial_state = self.create_navigation_state();
                            self.navigation_history.push(initial_state);
                        } else if self.mode == Mode::Search {
                            // Update the current search state before transitioning
                            // This ensures the current selection is saved
                            if let Some(_current_pos) = self.navigation_history.current_position() {
                                self.navigation_history
                                    .update_current(self.create_navigation_state());
                            }
                        }

                        self.ui.selected_result = Some(result);
                        self.ui.detail_scroll_offset = 0;
                        self.mode = Mode::MessageDetail;

                        // Save the new state after transitioning
                        let new_state = self.create_navigation_state();
                        self.navigation_history.push(new_state);
                    } else {
                        self.ui.selected_result = Some(result);
                        self.ui.detail_scroll_offset = 0;
                    }
                }
                Command::None
            }
            Message::EnterSessionViewer => {
                // Try to get result from selected result (when in detail view) or search results
                let result = if self.mode == Mode::MessageDetail {
                    self.ui.selected_result.as_ref()
                } else {
                    self.search.results.get(self.search.selected_index)
                };

                if let Some(result) = result {
                    // If this is our first navigation, save the initial state
                    if self.navigation_history.is_empty() {
                        let initial_state = self.create_navigation_state();
                        self.navigation_history.push(initial_state);
                    } else if self.mode == Mode::Search {
                        // Update the current search state before transitioning
                        // This ensures the current selection is saved
                        if let Some(_current_pos) = self.navigation_history.current_position() {
                            self.navigation_history
                                .update_current(self.create_navigation_state());
                        }
                    }

                    let file = result.file.clone();
                    self.mode = Mode::SessionViewer;
                    self.session.file_path = Some(file.clone());
                    self.session.session_id = Some(result.session_id.clone());
                    self.session.query.clear();
                    self.session.selected_index = 0;
                    self.session.scroll_offset = 0;

                    // Save the new state after transitioning
                    let new_state = self.create_navigation_state();
                    self.navigation_history.push(new_state);

                    Command::LoadSession(file)
                } else {
                    Command::None
                }
            }
            Message::ExitToSearch => {
                // Go back in navigation history
                if let Some(previous_state) = self.navigation_history.go_back() {
                    let command = self.restore_navigation_state(&previous_state);
                    self.ui.detail_scroll_offset = 0;
                    return command;
                } else {
                    // No history, go to Search
                    self.mode = Mode::Search;
                    self.session.messages.clear();
                }
                self.ui.detail_scroll_offset = 0;
                Command::None
            }
            Message::ShowHelp => {
                // If this is our first navigation, save the initial state
                if self.navigation_history.is_empty() {
                    let initial_state = self.create_navigation_state();
                    self.navigation_history.push(initial_state);
                }

                let command = self.set_mode(Mode::Help);

                // Save the new state after transitioning
                let new_state = self.create_navigation_state();
                self.navigation_history.push(new_state);

                command
            }
            Message::CloseHelp => {
                // Go back in navigation history
                if let Some(previous_state) = self.navigation_history.go_back() {
                    return self.restore_navigation_state(&previous_state);
                } else {
                    // No history, go to Search
                    self.mode = Mode::Search;
                }
                Command::None
            }
            Message::ToggleRoleFilter => {
                self.search.role_filter = match &self.search.role_filter {
                    None => Some("user".to_string()),
                    Some(r) if r == "user" => Some("assistant".to_string()),
                    Some(r) if r == "assistant" => Some("system".to_string()),
                    _ => None,
                };
                // Update navigation history to preserve filter state
                if self.navigation_history.current_position().is_some() {
                    self.navigation_history
                        .update_current(self.create_navigation_state());
                }
                Command::ExecuteSearch
            }
            Message::ToggleSearchOrder => {
                self.search.order = match self.search.order {
                    SearchOrder::Descending => SearchOrder::Ascending,
                    SearchOrder::Ascending => SearchOrder::Descending,
                };
                // Update navigation history to preserve sort order
                if self.navigation_history.current_position().is_some() {
                    self.navigation_history
                        .update_current(self.create_navigation_state());
                }
                // Re-execute the search with the new order to get different results
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
            Message::SessionNavigated(selected_index, scroll_offset) => {
                // Update session state with the current navigation position
                self.session.selected_index = selected_index;
                self.session.scroll_offset = scroll_offset;
                Command::None
            }
            Message::ToggleSessionOrder => {
                self.session.order = match self.session.order {
                    SessionOrder::Ascending => SessionOrder::Descending,
                    SessionOrder::Descending => SessionOrder::Ascending,
                };
                // Re-apply filter with new order
                self.update_session_filter();
                // Update navigation history to preserve sort order
                if self.navigation_history.current_position().is_some() {
                    self.navigation_history
                        .update_current(self.create_navigation_state());
                }
                Command::None
            }
            Message::ToggleSessionRoleFilter => {
                self.session.role_filter = match &self.session.role_filter {
                    None => Some("user".to_string()),
                    Some(r) if r == "user" => Some("assistant".to_string()),
                    Some(r) if r == "assistant" => Some("system".to_string()),
                    _ => None,
                };
                // Re-apply filter with new role
                self.update_session_filter();
                // Update navigation history to preserve filter state
                if self.navigation_history.current_position().is_some() {
                    self.navigation_history
                        .update_current(self.create_navigation_state());
                }
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
            Message::EnterMessageDetailFromSession(raw_json, file_path, session_id) => {
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

                    // If this is our first navigation, save the initial state
                    if self.navigation_history.is_empty() {
                        let initial_state = self.create_navigation_state();
                        self.navigation_history.push(initial_state);
                    } else if self.mode == Mode::SessionViewer {
                        // Update the current session state before transitioning
                        // This ensures the current selection is saved
                        if let Some(_current_pos) = self.navigation_history.current_position() {
                            self.navigation_history
                                .update_current(self.create_navigation_state());
                        }
                    }

                    self.ui.selected_result = Some(result);
                    self.ui.detail_scroll_offset = 0;
                    self.mode = Mode::MessageDetail;

                    // Save the new state after transitioning
                    let new_state = self.create_navigation_state();
                    self.navigation_history.push(new_state);
                }
                Command::None
            }
            Message::NavigateBack => {
                // Update the current state in history before going back
                if let Some(_current_pos) = self.navigation_history.current_position() {
                    self.navigation_history
                        .update_current(self.create_navigation_state());
                }

                #[cfg(test)]
                println!(
                    "NavigateBack: can_go_back = {}",
                    self.navigation_history.can_go_back()
                );

                if let Some(previous_state) = self.navigation_history.go_back() {
                    // When we're at position 0 and go back, we go to the initial state
                    // In that case, go_back() returns the state at position 0 (what we saved)
                    // and we should restore that state
                    #[cfg(test)]
                    println!(
                        "NavigateBack: restoring state with mode = {:?}",
                        previous_state.mode
                    );

                    return self.restore_navigation_state(&previous_state);
                } else {
                    #[cfg(test)]
                    println!("NavigateBack: go_back() returned None");
                }
                Command::None
            }
            Message::NavigateForward => {
                // Update the current state in history before going forward
                if let Some(_current_pos) = self.navigation_history.current_position() {
                    self.navigation_history
                        .update_current(self.create_navigation_state());
                }

                if self.navigation_history.can_go_forward() {
                    if let Some(next_state) = self.navigation_history.go_forward() {
                        return self.restore_navigation_state(&next_state);
                    }
                }
                Command::None
            }
            Message::CopyToClipboard(content) => Command::CopyToClipboard(content),
            Message::Quit => {
                Command::None // Handle in main loop
            }
            _ => Command::None,
        }
    }

    pub fn update_session_filter(&mut self) {
        use crate::interactive_ratatui::domain::filter::SessionFilter;
        use crate::interactive_ratatui::domain::session_list_item::SessionListItem;

        // Convert raw JSON strings to SessionListItems for search
        let items: Vec<SessionListItem> = self
            .session
            .messages
            .iter()
            .filter_map(|line| SessionListItem::from_json_line(line))
            .collect();

        self.session.filtered_indices =
            SessionFilter::filter_messages(&items, &self.session.query, &self.session.role_filter);

        // Apply ordering
        match self.session.order {
            SessionOrder::Ascending => {
                // Sort indices by timestamp (ascending)
                // Empty timestamps come first
                self.session.filtered_indices.sort_by(|&a, &b| {
                    let a_time = &items[a].timestamp;
                    let b_time = &items[b].timestamp;
                    match (a_time.is_empty(), b_time.is_empty()) {
                        (true, false) => std::cmp::Ordering::Less,
                        (false, true) => std::cmp::Ordering::Greater,
                        _ => a_time.cmp(b_time),
                    }
                });
            }
            SessionOrder::Descending => {
                // Sort indices by timestamp (descending)
                // Empty timestamps come first (at the top)
                self.session.filtered_indices.sort_by(|&a, &b| {
                    let a_time = &items[a].timestamp;
                    let b_time = &items[b].timestamp;
                    match (a_time.is_empty(), b_time.is_empty()) {
                        (true, false) => std::cmp::Ordering::Less,
                        (false, true) => std::cmp::Ordering::Greater,
                        _ => b_time.cmp(a_time),
                    }
                });
            }
        }

        // Reset selection if current selection is out of bounds
        if self.session.selected_index >= self.session.filtered_indices.len() {
            self.session.selected_index = 0;
            self.session.scroll_offset = 0;
        }
    }

    // Create a snapshot of current state
    pub fn create_navigation_state(&self) -> NavigationState {
        NavigationState {
            mode: self.mode,
            search_state: SearchStateSnapshot {
                query: self.search.query.clone(),
                results: self.search.results.clone(),
                selected_index: self.search.selected_index,
                scroll_offset: self.search.scroll_offset,
                role_filter: self.search.role_filter.clone(),
                order: self.search.order,
            },
            session_state: SessionStateSnapshot {
                messages: self.session.messages.clone(),
                query: self.session.query.clone(),
                filtered_indices: self.session.filtered_indices.clone(),
                selected_index: self.session.selected_index,
                scroll_offset: self.session.scroll_offset,
                order: self.session.order,
                file_path: self.session.file_path.clone(),
                session_id: self.session.session_id.clone(),
                role_filter: self.session.role_filter.clone(),
            },
            ui_state: UiStateSnapshot {
                message: self.ui.message.clone(),
                detail_scroll_offset: self.ui.detail_scroll_offset,
                selected_result: self.ui.selected_result.clone(),
                truncation_enabled: self.ui.truncation_enabled,
            },
        }
    }

    // Restore state from a snapshot
    pub fn restore_navigation_state(&mut self, state: &NavigationState) -> Command {
        self.mode = state.mode;

        // Restore search state
        self.search.query = state.search_state.query.clone();
        self.search.results = state.search_state.results.clone();
        self.search.selected_index = state.search_state.selected_index;
        self.search.scroll_offset = state.search_state.scroll_offset;
        self.search.role_filter = state.search_state.role_filter.clone();
        self.search.order = state.search_state.order;

        // Restore session state
        self.session.messages = state.session_state.messages.clone();
        self.session.query = state.session_state.query.clone();
        self.session.filtered_indices = state.session_state.filtered_indices.clone();
        self.session.selected_index = state.session_state.selected_index;
        self.session.scroll_offset = state.session_state.scroll_offset;
        self.session.order = state.session_state.order;
        self.session.file_path = state.session_state.file_path.clone();
        self.session.session_id = state.session_state.session_id.clone();
        self.session.role_filter = state.session_state.role_filter.clone();

        // Restore UI state
        self.ui.message = state.ui_state.message.clone();
        self.ui.detail_scroll_offset = state.ui_state.detail_scroll_offset;
        self.ui.selected_result = state.ui_state.selected_result.clone();
        self.ui.truncation_enabled = state.ui_state.truncation_enabled;

        // Execute mode-specific initialization
        self.initialize_mode()
    }

    // Initialize mode-specific data (like React's componentDidMount)
    // This is called when restoring from navigation history.
    // For direct transitions, initialization is handled in the message handlers
    // because they often need transition-specific context (e.g., selected result).
    fn initialize_mode(&mut self) -> Command {
        match self.mode {
            Mode::SessionViewer => {
                // Reload session data when returning to SessionViewer
                // This ensures the session messages are always fresh
                if let Some(file_path) = &self.session.file_path {
                    Command::LoadSession(file_path.clone())
                } else {
                    Command::None
                }
            }
            Mode::MessageDetail => {
                // ResultDetail initialization during direct transition:
                // - Selected result is set in EnterResultDetail handler
                // - Scroll position is reset here for consistency
                self.ui.detail_scroll_offset = 0;
                Command::None
            }
            Mode::Search => {
                // Search mode maintains its state across transitions
                // No special initialization needed
                Command::None
            }
            Mode::Help => {
                // Help is a stateless dialog
                // No initialization needed
                Command::None
            }
        }
    }

    // Set mode with initialization (for direct transitions)
    fn set_mode(&mut self, mode: Mode) -> Command {
        self.mode = mode;
        self.initialize_mode()
    }
}
