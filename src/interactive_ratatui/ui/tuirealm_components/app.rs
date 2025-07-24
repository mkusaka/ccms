use tuirealm::application::PollStrategy;
use tuirealm::terminal::{TerminalBridge, CrosstermTerminalAdapter};
use tuirealm::{Application, EventListenerCfg, Update, NoUserEvent, MockComponent};
use std::sync::{Arc, Mutex};

use crate::interactive_ratatui::application::search_service::SearchService;
use crate::interactive_ratatui::application::session_service::SessionService;
use crate::interactive_ratatui::application::cache_service::CacheService;
use crate::interactive_ratatui::domain::models::{Mode, SearchRequest, SessionOrder};
use crate::interactive_ratatui::ui::tuirealm_components::messages::{AppMessage, ComponentId};
use crate::interactive_ratatui::ui::tuirealm_components::text_input::TextInput;
use crate::interactive_ratatui::ui::tuirealm_components::async_search::{AsyncSearchHandler, SearchDebouncer};
use crate::query::condition::{SearchOptions, SearchResult};

/// Search-related state
pub struct SearchState {
    pub query: String,
    pub results: Vec<SearchResult>,
    pub selected_index: usize,
    pub scroll_offset: usize,
    pub role_filter: Option<String>,
    pub is_searching: bool,
    pub current_search_id: u64,
}

/// Session viewer state
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

/// UI-related state
pub struct UiState {
    pub message: Option<String>,
    pub detail_scroll_offset: usize,
    pub selected_result: Option<SearchResult>,
    pub truncation_enabled: bool,
}

/// The main application model for tui-realm
pub struct Model {
    /// The tui-realm application instance
    pub app: Application<ComponentId, AppMessage, NoUserEvent>,
    /// Terminal bridge for rendering (optional for testing)
    pub terminal: Option<TerminalBridge<CrosstermTerminalAdapter>>,
    /// Current application mode
    pub mode: Mode,
    /// Mode stack for navigation
    pub mode_stack: Vec<Mode>,
    /// Search-related state
    pub search_state: SearchState,
    /// Session viewer state
    pub session_state: SessionState,
    /// UI-related state
    pub ui_state: UiState,
    /// Search service
    pub search_service: Arc<SearchService>,
    /// Session service
    pub session_service: SessionService,
    /// Cache service
    pub cache_service: Arc<Mutex<CacheService>>,
    /// Base search options
    pub base_options: SearchOptions,
    /// Maximum results to display
    pub max_results: usize,
    /// Whether the application should quit
    pub quit: bool,
    /// Async search handler
    pub async_search: Option<AsyncSearchHandler>,
    /// Search debouncer
    pub search_debouncer: SearchDebouncer,
}

impl Model {
    pub fn new(
        search_options: SearchOptions,
        max_results: usize,
    ) -> anyhow::Result<Self> {
        Self::new_with_terminal(search_options, max_results, true)
    }

    /// Create a new model with optional terminal initialization (for testing)
    pub fn new_with_terminal(
        search_options: SearchOptions,
        max_results: usize,
        init_terminal: bool,
    ) -> anyhow::Result<Self> {
        let mut app = Application::init(
            EventListenerCfg::default()
                .crossterm_input_listener(std::time::Duration::from_millis(50), 10)
                .poll_timeout(std::time::Duration::from_millis(50))
                .tick_interval(std::time::Duration::from_secs(1)),
        );

        // Mount SearchBar component
        let mut search_bar = TextInput::default();
        search_bar.attr(
            tuirealm::Attribute::Custom("id"),
            tuirealm::AttrValue::String("search_bar".to_string()),
        );
        search_bar.attr(
            tuirealm::Attribute::Title,
            tuirealm::AttrValue::Title(("Search".to_string(), tuirealm::props::Alignment::Left)),
        );
        app.mount(ComponentId::SearchBar, Box::new(search_bar), vec![])?;

        // Focus on search bar initially
        app.active(&ComponentId::SearchBar)?;

        let terminal = if init_terminal {
            Some(TerminalBridge::init_crossterm()?)
        } else {
            None
        };

        let cache_service = Arc::new(Mutex::new(CacheService::new()));
        let session_service = SessionService::new(cache_service.clone());
        let search_service = Arc::new(SearchService::new(search_options.clone()));

        // Initialize async search handler
        let mut async_search = AsyncSearchHandler::new(search_service.clone());
        if init_terminal {
            async_search.start();
        }

        Ok(Self {
            app,
            terminal,
            mode: Mode::Search,
            mode_stack: Vec::new(),
            search_state: SearchState {
                query: String::new(),
                results: Vec::new(),
                selected_index: 0,
                scroll_offset: 0,
                role_filter: None,
                is_searching: false,
                current_search_id: 0,
            },
            session_state: SessionState {
                messages: Vec::new(),
                query: String::new(),
                filtered_indices: Vec::new(),
                selected_index: 0,
                scroll_offset: 0,
                order: None,
                file_path: None,
                session_id: None,
            },
            ui_state: UiState {
                message: None,
                detail_scroll_offset: 0,
                selected_result: None,
                truncation_enabled: true,
            },
            search_service,
            session_service,
            cache_service: cache_service.clone(),
            base_options: search_options,
            max_results,
            quit: false,
            async_search: Some(async_search),
            search_debouncer: SearchDebouncer::new(300), // 300ms debounce
        })
    }

    pub fn tick(&mut self, poll_strategy: PollStrategy) -> anyhow::Result<Vec<AppMessage>> {
        let mut messages = self.app.tick(poll_strategy)?;
        
        // Check for debounced search
        if let Some(query) = self.search_debouncer.should_search() {
            if query == self.search_state.query && !self.search_state.is_searching {
                messages.push(AppMessage::SearchRequested);
            }
        }
        
        // Check for async search results
        if let Some(async_search) = &self.async_search {
            if let Some(response) = async_search.poll_results() {
                // Only process if this is the latest search
                if response.id >= self.search_state.current_search_id {
                    messages.push(AppMessage::SearchCompleted(response.results));
                }
            }
        }
        
        Ok(messages)
    }

    pub fn view(&mut self) -> anyhow::Result<()> {
        if let Some(terminal) = &mut self.terminal {
            terminal.raw_mut().draw(|f| {
                let _ = self.app.view(&ComponentId::SearchBar, f, f.area());
            })?;
        }
        Ok(())
    }

    fn get_selected_result(&self) -> Option<&SearchResult> {
        self.search_state.results.get(self.search_state.selected_index)
    }

    fn adjust_session_scroll_offset(&mut self) {
        let visible_height = 20; // This should come from terminal size

        if self.session_state.selected_index < self.session_state.scroll_offset {
            self.session_state.scroll_offset = self.session_state.selected_index;
        } else if self.session_state.selected_index >= self.session_state.scroll_offset + visible_height {
            self.session_state.scroll_offset = self.session_state.selected_index - visible_height + 1;
        }
    }

    fn update_session_filter(&mut self) {
        use crate::interactive_ratatui::domain::filter::SessionFilter;
        use crate::interactive_ratatui::domain::session_list_item::SessionListItem;

        // Convert raw JSON strings to SessionListItems for search
        let items: Vec<SessionListItem> = self
            .session_state
            .messages
            .iter()
            .enumerate()
            .filter_map(|(idx, line)| SessionListItem::from_json_line(idx, line))
            .collect();

        self.session_state.filtered_indices = SessionFilter::filter_messages(&items, &self.session_state.query);

        // Reset selection if current selection is out of bounds
        if self.session_state.selected_index >= self.session_state.filtered_indices.len() {
            self.session_state.selected_index = 0;
            self.session_state.scroll_offset = 0;
        }
    }
}

impl Update<AppMessage> for Model {
    fn update(&mut self, msg: Option<AppMessage>) -> Option<AppMessage> {
        match msg {
            Some(AppMessage::QueryChanged(q)) => {
                self.search_state.query = q.clone();
                self.ui_state.message = Some("typing...".to_string());
                // Update debouncer with new query
                self.search_debouncer.update_query(q);
                None
            }
            Some(AppMessage::SearchRequested) => {
                self.search_state.is_searching = true;
                self.ui_state.message = Some("searching...".to_string());
                self.search_state.current_search_id += 1;
                
                // Create search request
                let request = SearchRequest {
                    id: self.search_state.current_search_id,
                    query: self.search_state.query.clone(),
                    role_filter: self.search_state.role_filter.clone(),
                    pattern: "**/*.jsonl".to_string(),  // Default pattern for JSONL files
                };
                
                // Send async search request
                if let Some(async_search) = &self.async_search {
                    if let Err(e) = async_search.search(request) {
                        self.search_state.is_searching = false;
                        self.ui_state.message = Some(format!("Search error: {}", e));
                    }
                } else {
                    // Fallback to sync search for testing
                    match self.search_service.search(request) {
                        Ok(response) => {
                            self.search_state.results = response.results;
                            self.search_state.is_searching = false;
                            self.search_state.selected_index = 0;
                            self.search_state.scroll_offset = 0;
                            self.ui_state.message = None;
                        }
                        Err(e) => {
                            self.search_state.is_searching = false;
                            self.ui_state.message = Some(format!("Search error: {}", e));
                        }
                    }
                }
                None
            }
            Some(AppMessage::SearchCompleted(results)) => {
                self.search_state.results = results;
                self.search_state.is_searching = false;
                self.search_state.selected_index = 0;
                self.search_state.scroll_offset = 0;
                self.ui_state.message = None;
                None
            }
            Some(AppMessage::SelectResult(index)) => {
                if index < self.search_state.results.len() {
                    self.search_state.selected_index = index;
                }
                None
            }
            Some(AppMessage::NavigateUp) => {
                if self.search_state.selected_index > 0 {
                    self.search_state.selected_index -= 1;
                }
                None
            }
            Some(AppMessage::NavigateDown) => {
                if self.search_state.selected_index < self.search_state.results.len().saturating_sub(1) {
                    self.search_state.selected_index += 1;
                }
                None
            }
            Some(AppMessage::EnterResultDetail) => {
                if let Some(result) = self.get_selected_result() {
                    self.ui_state.selected_result = Some(result.clone());
                    self.ui_state.detail_scroll_offset = 0;
                    self.mode_stack.push(self.mode);
                    self.mode = Mode::ResultDetail;
                }
                None
            }
            Some(AppMessage::ExitResultDetail) => {
                self.mode = self.mode_stack.pop().unwrap_or(Mode::Search);
                self.ui_state.detail_scroll_offset = 0;
                None
            }
            Some(AppMessage::EnterSessionViewer(session_id)) => {
                // Try to get result from selected result (when in detail view) or search results
                let result = if self.mode == Mode::ResultDetail {
                    self.ui_state.selected_result.as_ref()
                } else {
                    self.search_state.results.get(self.search_state.selected_index)
                };

                if let Some(result) = result {
                    let file = result.file.clone();
                    self.mode_stack.push(self.mode);
                    self.mode = Mode::SessionViewer;
                    self.session_state.file_path = Some(file.clone());
                    self.session_state.session_id = Some(session_id);
                    self.session_state.query.clear();
                    self.session_state.selected_index = 0;
                    self.session_state.scroll_offset = 0;
                    
                    // Load session
                    match self.session_service.load_session(&file) {
                        Ok(messages) => {
                            // Convert SessionMessage to raw JSON strings
                            self.session_state.messages = messages.into_iter()
                                .map(|msg| serde_json::to_string(&msg).unwrap_or_default())
                                .collect();
                        }
                        Err(e) => {
                            self.ui_state.message = Some(format!("Failed to load session: {}", e));
                        }
                    }
                }
                None
            }
            Some(AppMessage::ExitSessionViewer) => {
                // Pop mode from stack if available, otherwise go to Search
                self.mode = self.mode_stack.pop().unwrap_or(Mode::Search);
                if self.mode == Mode::Search {
                    // Only clear session messages when returning to search
                    self.session_state.messages.clear();
                }
                self.ui_state.detail_scroll_offset = 0;
                None
            }
            Some(AppMessage::EnterHelp) => {
                self.mode_stack.push(self.mode);
                self.mode = Mode::Help;
                None
            }
            Some(AppMessage::ExitHelp) => {
                self.mode = self.mode_stack.pop().unwrap_or(Mode::Search);
                None
            }
            Some(AppMessage::ToggleRoleFilter) => {
                self.search_state.role_filter = match &self.search_state.role_filter {
                    None => Some("user".to_string()),
                    Some(r) if r == "user" => Some("assistant".to_string()),
                    Some(r) if r == "assistant" => Some("system".to_string()),
                    _ => None,
                };
                // Trigger new search
                Some(AppMessage::SearchRequested)
            }
            Some(AppMessage::ToggleTruncation) => {
                self.ui_state.truncation_enabled = !self.ui_state.truncation_enabled;
                let status = if self.ui_state.truncation_enabled {
                    "Truncated"
                } else {
                    "Full Text"
                };
                self.ui_state.message = Some(format!("Message display: {status}"));
                None
            }
            Some(AppMessage::SessionQueryChanged(q)) => {
                self.session_state.query = q;
                self.update_session_filter();
                None
            }
            Some(AppMessage::SessionScrollUp) => {
                if self.session_state.selected_index > 0 {
                    self.session_state.selected_index -= 1;
                    self.adjust_session_scroll_offset();
                }
                None
            }
            Some(AppMessage::SessionScrollDown) => {
                if self.session_state.selected_index + 1 < self.session_state.filtered_indices.len() {
                    self.session_state.selected_index += 1;
                    self.adjust_session_scroll_offset();
                }
                None
            }
            Some(AppMessage::ToggleSessionOrder) => {
                self.session_state.order = match self.session_state.order {
                    None => Some(SessionOrder::Ascending),
                    Some(SessionOrder::Ascending) => Some(SessionOrder::Descending),
                    Some(SessionOrder::Descending) => Some(SessionOrder::Original),
                    Some(SessionOrder::Original) => None,
                };
                // Re-apply filter with new order
                self.update_session_filter();
                None
            }
            Some(AppMessage::SetStatus(msg)) => {
                self.ui_state.message = Some(msg);
                None
            }
            Some(AppMessage::ClearStatus) => {
                self.ui_state.message = None;
                None
            }
            Some(AppMessage::CopyToClipboard(text)) => {
                // Use arboard for clipboard operations
                match arboard::Clipboard::new() {
                    Ok(mut clipboard) => {
                        match clipboard.set_text(&text) {
                            Ok(_) => {
                                self.ui_state.message = Some("Copied!".to_string());
                            }
                            Err(e) => {
                                self.ui_state.message = Some(format!("Failed to copy: {}", e));
                            }
                        }
                    }
                    Err(e) => {
                        self.ui_state.message = Some(format!("Clipboard error: {}", e));
                    }
                }
                None
            }
            Some(AppMessage::Quit) => {
                self.quit = true;
                None
            }
            _ => None,
        }
    }
}