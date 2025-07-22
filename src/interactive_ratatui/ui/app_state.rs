use crate::SearchOptions;
use crate::interactive_ratatui::domain::models::SessionOrder;
use crate::interactive_ratatui::ui::commands::Command;
use crate::interactive_ratatui::ui::events::Message;
use crate::query::condition::SearchResult;

// Re-export Mode
pub use crate::interactive_ratatui::domain::models::Mode;

pub struct AppState {
    pub mode: Mode,
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
                if self.search.selected_index > 0 {
                    self.search.selected_index -= 1;
                    self.adjust_scroll_offset();
                }
                Command::None
            }
            Message::ScrollDown => {
                if self.search.selected_index + 1 < self.search.results.len() {
                    self.search.selected_index += 1;
                    self.adjust_scroll_offset();
                }
                Command::None
            }
            Message::EnterResultDetail => {
                if let Some(result) = self.get_selected_result() {
                    self.ui.selected_result = Some(result.clone());
                    self.ui.detail_scroll_offset = 0;
                    self.mode = Mode::ResultDetail;
                }
                Command::None
            }
            Message::EnterSessionViewer => {
                if let Some(result) = self.search.results.get(self.search.selected_index) {
                    let file = result.file.clone();
                    self.mode = Mode::SessionViewer;
                    self.session.file_path = Some(file.clone());
                    self.session.query.clear();
                    self.session.selected_index = 0;
                    self.session.scroll_offset = 0;
                    Command::LoadSession(file)
                } else {
                    Command::None
                }
            }
            Message::ExitToSearch => {
                self.mode = Mode::Search;
                self.ui.detail_scroll_offset = 0;
                self.session.messages.clear();
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
                if self.session.selected_index > 0 {
                    self.session.selected_index -= 1;
                    self.adjust_session_scroll_offset();
                }
                Command::None
            }
            Message::SessionScrollDown => {
                if self.session.selected_index + 1 < self.session.filtered_indices.len() {
                    self.session.selected_index += 1;
                    self.adjust_session_scroll_offset();
                }
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

    fn adjust_scroll_offset(&mut self) {
        // This would be calculated based on terminal height
        // For now, keep it simple
        let visible_height = 20; // This should come from terminal size

        if self.search.selected_index < self.search.scroll_offset {
            self.search.scroll_offset = self.search.selected_index;
        } else if self.search.selected_index >= self.search.scroll_offset + visible_height {
            self.search.scroll_offset = self.search.selected_index - visible_height + 1;
        }
    }

    fn adjust_session_scroll_offset(&mut self) {
        let visible_height = 20; // This should come from terminal size

        if self.session.selected_index < self.session.scroll_offset {
            self.session.scroll_offset = self.session.selected_index;
        } else if self.session.selected_index >= self.session.scroll_offset + visible_height {
            self.session.scroll_offset = self.session.selected_index - visible_height + 1;
        }
    }

    fn update_session_filter(&mut self) {
        use crate::interactive_ratatui::domain::filter::SessionFilter;

        self.session.filtered_indices =
            SessionFilter::filter_messages(&self.session.messages, &self.session.query);

        // Reset selection if current selection is out of bounds
        if self.session.selected_index >= self.session.filtered_indices.len() {
            self.session.selected_index = 0;
            self.session.scroll_offset = 0;
        }
    }
}
