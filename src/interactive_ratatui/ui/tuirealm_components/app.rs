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
use crate::query::condition::SearchOptions;

/// The main application model for tui-realm
pub struct Model {
    /// The tui-realm application instance
    pub app: Application<ComponentId, AppMessage, NoUserEvent>,
    /// Terminal bridge for rendering
    pub terminal: TerminalBridge<CrosstermTerminalAdapter>,
    /// Current application mode
    pub mode: Mode,
    /// Search service
    pub search_service: SearchService,
    /// Session service
    pub session_service: SessionService,
    /// Cache service
    pub cache_service: Arc<Mutex<CacheService>>,
    /// Current search results
    pub results: Vec<crate::query::condition::SearchResult>,
    /// Selected result index
    pub selected_index: usize,
    /// Current session order
    pub session_order: Option<SessionOrder>,
    /// Session messages
    pub session_messages: Vec<String>,
    /// Session filtered indices
    pub session_filtered_indices: Vec<usize>,
    /// Whether the application should quit
    pub quit: bool,
}

impl Model {
    pub fn new(
        search_options: SearchOptions,
        _session_filter: Option<String>,
        _role_filter: Option<String>,
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

        let terminal = TerminalBridge::init_crossterm()?;

        let cache_service = Arc::new(Mutex::new(CacheService::new()));
        let session_service = SessionService::new(cache_service.clone());

        Ok(Self {
            app,
            terminal,
            mode: Mode::Search,
            search_service: SearchService::new(search_options),
            session_service,
            cache_service: cache_service.clone(),
            results: Vec::new(),
            selected_index: 0,
            session_order: None,
            session_messages: Vec::new(),
            session_filtered_indices: Vec::new(),
            quit: false,
        })
    }

    pub fn tick(&mut self, poll_strategy: PollStrategy) -> anyhow::Result<Vec<AppMessage>> {
        Ok(self.app.tick(poll_strategy)?)
    }

    pub fn view(&mut self) -> anyhow::Result<()> {
        self.terminal.raw_mut().draw(|f| {
            let _ = self.app.view(&ComponentId::SearchBar, f, f.area());
        })?;
        Ok(())
    }
}

impl Update<AppMessage> for Model {
    fn update(&mut self, msg: Option<AppMessage>) -> Option<AppMessage> {
        match msg {
            Some(AppMessage::QueryChanged(query)) => {
                // Start search with the new query
                let request = SearchRequest {
                    id: 1, // Simple ID for now
                    query: query.clone(),
                    role_filter: None,
                    pattern: "**/*.jsonl".to_string(), // Default pattern for now
                };
                
                // Execute search (simplified for now)
                match self.search_service.search(request) {
                    Ok(response) => {
                        self.results = response.results;
                        self.selected_index = 0;
                    }
                    Err(e) => {
                        // Handle error
                        eprintln!("Search error: {}", e);
                    }
                }
                
                None
            }
            Some(AppMessage::NavigateUp) => {
                if self.selected_index > 0 {
                    self.selected_index -= 1;
                }
                None
            }
            Some(AppMessage::NavigateDown) => {
                if self.selected_index < self.results.len().saturating_sub(1) {
                    self.selected_index += 1;
                }
                None
            }
            Some(AppMessage::EnterResultDetail) => {
                self.mode = Mode::ResultDetail;
                None
            }
            Some(AppMessage::ExitResultDetail) => {
                self.mode = Mode::Search;
                None
            }
            Some(AppMessage::EnterSessionViewer(_session_id)) => {
                // Load session
                if let Some(result) = self.results.get(self.selected_index) {
                    match self.session_service.load_session(&result.file) {
                        Ok(messages) => {
                            // Convert SessionMessage to raw JSON strings
                            self.session_messages = messages.into_iter()
                                .map(|msg| serde_json::to_string(&msg).unwrap_or_default())
                                .collect();
                            self.mode = Mode::SessionViewer;
                        }
                        Err(e) => {
                            eprintln!("Failed to load session: {}", e);
                        }
                    }
                }
                None
            }
            Some(AppMessage::ExitSessionViewer) => {
                self.mode = Mode::Search;
                self.session_messages.clear();
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