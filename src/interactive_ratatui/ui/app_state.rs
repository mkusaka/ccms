use crate::interactive_ratatui::constants::*;
use crate::interactive_ratatui::domain::models::{SearchOrder, SearchTab, SessionOrder};
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
    pub session_list: SessionListState,
    pub ui: UiState,
}

pub struct SessionListState {
    pub sessions: Vec<SessionInfo>,
    pub filtered_sessions: Vec<SessionInfo>,
    pub query: String,
    pub selected_index: usize,
    pub scroll_offset: usize,
    pub is_loading: bool,
    pub is_searching: bool,
    pub current_search_id: u64,
    pub preview_enabled: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SessionInfo {
    pub file_path: String,
    pub session_id: String,
    pub timestamp: String,
    pub message_count: usize,
    pub first_message: String,
    pub preview_messages: Vec<(String, String)>, // (role, content) pairs
    pub summary: Option<String>,
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
    pub preview_enabled: bool,
    pub current_tab: SearchTab,
}

pub struct SessionState {
    pub messages: Vec<String>, // Will be removed after full migration
    pub search_results: Vec<SearchResult>, // New: unified with search results
    pub query: String,
    pub filtered_indices: Vec<usize>, // Will be removed (handled by ResultList)
    pub selected_index: usize,
    pub scroll_offset: usize,
    pub order: SessionOrder,
    pub file_path: Option<String>,
    pub session_id: Option<String>,
    pub role_filter: Option<String>,
    pub preview_enabled: bool,
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
                preview_enabled: false,
                current_tab: SearchTab::Search,
            },
            session: SessionState {
                messages: Vec::new(),
                search_results: Vec::new(),
                query: String::new(),
                filtered_indices: Vec::new(),
                selected_index: 0,
                scroll_offset: 0,
                order: SessionOrder::Ascending,
                file_path: None,
                session_id: None,
                role_filter: None,
                preview_enabled: false,
            },
            session_list: SessionListState {
                sessions: Vec::new(),
                filtered_sessions: Vec::new(),
                query: String::new(),
                selected_index: 0,
                scroll_offset: 0,
                is_loading: false,
                is_searching: false,
                current_search_id: 0,
                preview_enabled: true, // Default to true for better UX
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
                self.ui.message = Some("[typing...]".to_string());
                Command::ScheduleSearch(300) // 300ms debounce
            }
            Message::SearchRequested => {
                self.search.is_searching = true;
                self.ui.message = Some("[searching...]".to_string());
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
                    Some(r) if r == "system" => Some("summary".to_string()),
                    _ => None,
                };
                // Update navigation history to preserve filter state
                if self.navigation_history.current_position().is_some() {
                    self.navigation_history
                        .update_current(self.create_navigation_state());
                }
                // Set searching state and message
                self.search.is_searching = true;
                self.ui.message = Some("[searching...]".to_string());
                self.search.current_search_id += 1;
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
                // Set searching state and message
                self.search.is_searching = true;
                self.ui.message = Some("[searching...]".to_string());
                self.search.current_search_id += 1;
                // Re-execute the search with the new order to get different results
                Command::ExecuteSearch
            }
            Message::TogglePreview => {
                self.search.preview_enabled = !self.search.preview_enabled;
                Command::None
            }
            Message::SwitchToSearchTab => {
                if self.mode == Mode::Search {
                    self.search.current_tab = SearchTab::Search;
                }
                Command::None
            }
            Message::SwitchToSessionListTab => {
                if self.mode == Mode::Search {
                    self.search.current_tab = SearchTab::SessionList;
                    // Load session list if not loaded yet
                    if self.session_list.sessions.is_empty() && !self.session_list.is_loading {
                        self.session_list.is_loading = true;
                        Command::LoadSessionList
                    } else {
                        Command::None
                    }
                } else {
                    Command::None
                }
            }
            Message::LoadSessionList => {
                self.session_list.is_loading = true;
                Command::LoadSessionList
            }
            Message::SessionListLoaded(sessions) => {
                self.session_list.sessions = sessions
                    .into_iter()
                    .map(
                        |(
                            file_path,
                            session_id,
                            timestamp,
                            message_count,
                            first_message,
                            preview_messages,
                            summary,
                        )| {
                            SessionInfo {
                                file_path,
                                session_id,
                                timestamp,
                                message_count,
                                first_message,
                                preview_messages,
                                summary,
                            }
                        },
                    )
                    .collect();
                self.session_list.is_loading = false;
                self.session_list.selected_index = 0;
                self.session_list.scroll_offset = 0;
                // Copy all sessions to filtered_sessions initially
                self.session_list.filtered_sessions = self.session_list.sessions.clone();
                Command::None
            }
            Message::SessionListQueryChanged(q) => {
                self.session_list.query = q;
                self.ui.message = Some("[typing...]".to_string());
                Command::ScheduleSessionListSearch(300) // 300ms debounce
            }
            Message::SessionListSearchRequested => {
                self.session_list.is_searching = true;
                self.ui.message = Some("[searching...]".to_string());
                self.session_list.current_search_id += 1;
                Command::ExecuteSessionListSearch
            }
            Message::SessionListSearchCompleted(filtered_sessions) => {
                self.session_list.filtered_sessions = filtered_sessions;
                self.session_list.is_searching = false;
                self.session_list.selected_index = 0;
                self.session_list.scroll_offset = 0;
                self.ui.message = None;
                Command::None
            }
            Message::SelectSessionFromList(index) => {
                if index < self.session_list.filtered_sessions.len() {
                    self.session_list.selected_index = index;
                }
                Command::None
            }
            Message::SessionListScrollUp => {
                if self.session_list.selected_index > 0 {
                    self.session_list.selected_index -= 1;
                }
                Command::None
            }
            Message::SessionListScrollDown => {
                if self.session_list.selected_index + 1 < self.session_list.filtered_sessions.len() {
                    self.session_list.selected_index += 1;
                }
                Command::None
            }
            Message::SessionListPageUp => {
                // Move up by full page (default 30 lines)
                let page_size = 30;
                self.session_list.selected_index =
                    self.session_list.selected_index.saturating_sub(page_size);
                Command::None
            }
            Message::SessionListPageDown => {
                // Move down by full page (default 30 lines)
                let page_size = 30;
                let max_index = self.session_list.filtered_sessions.len().saturating_sub(1);
                self.session_list.selected_index =
                    (self.session_list.selected_index + page_size).min(max_index);
                Command::None
            }
            Message::SessionListHalfPageUp => {
                // Move up by half page (default 15 lines)
                let half_page = 15;
                self.session_list.selected_index =
                    self.session_list.selected_index.saturating_sub(half_page);
                Command::None
            }
            Message::SessionListHalfPageDown => {
                // Move down by half page (default 15 lines)
                let half_page = 15;
                let max_index = self.session_list.filtered_sessions.len().saturating_sub(1);
                self.session_list.selected_index =
                    (self.session_list.selected_index + half_page).min(max_index);
                Command::None
            }
            Message::ToggleSessionListPreview => {
                self.session_list.preview_enabled = !self.session_list.preview_enabled;
                Command::None
            }
            Message::EnterSessionViewerFromList(file_path) => {
                // Find the session info to get the session_id
                if let Some(session_info) = self
                    .session_list
                    .filtered_sessions
                    .iter()
                    .find(|s| s.file_path == file_path)
                {
                    // If this is our first navigation, save the initial state
                    if self.navigation_history.is_empty() {
                        let initial_state = self.create_navigation_state();
                        self.navigation_history.push(initial_state);
                    } else {
                        // Update the current state before transitioning
                        if let Some(_current_pos) = self.navigation_history.current_position() {
                            self.navigation_history
                                .update_current(self.create_navigation_state());
                        }
                    }

                    self.mode = Mode::SessionViewer;
                    self.session.file_path = Some(file_path.clone());
                    self.session.session_id = Some(session_info.session_id.clone());
                    self.session.query.clear();
                    self.session.selected_index = 0;
                    self.session.scroll_offset = 0;

                    // Save the new state after transitioning
                    let new_state = self.create_navigation_state();
                    self.navigation_history.push(new_state);

                    Command::LoadSession(file_path)
                } else {
                    Command::None
                }
            }
            Message::SessionQueryChanged(q) => {
                self.session.query = q;
                // Trigger a new search with session_id filter
                if self.session.session_id.is_some() {
                    Command::ExecuteSessionSearch
                } else {
                    Command::None
                }
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
                // Update navigation history to preserve sort order
                if self.navigation_history.current_position().is_some() {
                    self.navigation_history
                        .update_current(self.create_navigation_state());
                }
                // Trigger a new search with updated order
                if self.session.session_id.is_some() {
                    Command::ExecuteSessionSearch
                } else {
                    Command::None
                }
            }
            Message::ToggleSessionRoleFilter => {
                self.session.role_filter = match &self.session.role_filter {
                    None => Some("user".to_string()),
                    Some(r) if r == "user" => Some("assistant".to_string()),
                    Some(r) if r == "assistant" => Some("system".to_string()),
                    Some(r) if r == "system" => Some("summary".to_string()),
                    _ => None,
                };
                // Update navigation history to preserve filter state
                if self.navigation_history.current_position().is_some() {
                    self.navigation_history
                        .update_current(self.create_navigation_state());
                }
                // Trigger a new search with updated filter
                if self.session.session_id.is_some() {
                    Command::ExecuteSessionSearch
                } else {
                    Command::None
                }
            }
            Message::ToggleSessionPreview => {
                self.session.preview_enabled = !self.session.preview_enabled;
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
                                        if let Some(item_type) =
                                            item.get("type").and_then(|t| t.as_str())
                                        {
                                            match item_type {
                                                "text" => item
                                                    .get("text")
                                                    .and_then(|t| t.as_str())
                                                    .map(|s| s.to_string()),
                                                "thinking" => item
                                                    .get("thinking")
                                                    .and_then(|t| t.as_str())
                                                    .map(|s| s.to_string()),
                                                "tool_use" => {
                                                    let name = item
                                                        .get("name")
                                                        .and_then(|n| n.as_str())
                                                        .unwrap_or("Tool");
                                                    Some(format!("[Tool: {name}]"))
                                                }
                                                _ => None,
                                            }
                                        } else {
                                            // Fallback for simple text
                                            item.get("text")
                                                .and_then(|t| t.as_str())
                                                .map(|s| s.to_string())
                                        }
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
                        message_type: "message".to_string(),
                        query: QueryCondition::Literal {
                            pattern: String::new(),
                            case_sensitive: false,
                        },
                        cwd: String::new(), // Not available from session viewer
                        raw_json: Some(raw_json), // Store full JSON
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
                preview_enabled: self.search.preview_enabled,
                current_tab: self.search.current_tab,
            },
            session_state: SessionStateSnapshot {
                messages: self.session.messages.clone(),
                search_results: self.session.search_results.clone(),
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
        self.search.preview_enabled = state.search_state.preview_enabled;
        self.search.current_tab = state.search_state.current_tab;

        // Restore session state
        self.session.messages = state.session_state.messages.clone();
        self.session.search_results = state.session_state.search_results.clone();
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
