use std::sync::mpsc;
use tuirealm::props::{AttrValue, Attribute};
use tuirealm::{Application, NoUserEvent, Update, StateValue};

use crate::query::condition::SearchResult;
use crate::interactive_ratatui::domain::models::SessionOrder;
use super::messages::{AppMessage, AppMode, ComponentId};
use super::state::AppState;
use super::components::{SearchInput, ResultList, ResultDetail, SessionViewer, HelpDialog};
use super::services::{SearchService, SessionService, ClipboardService};

/// Main application structure
pub struct App {
    /// Application state
    pub state: AppState,
    
    /// Services
    pub search_service: SearchService,
    pub session_service: SessionService,
    pub clipboard_service: ClipboardService,
    
    /// Search result channel
    search_rx: Option<mpsc::Receiver<Vec<SearchResult>>>,
}

impl App {
    pub fn new(
        pattern: Option<String>,
        timestamp_gte: Option<String>,
        timestamp_lt: Option<String>,
        session_id: Option<String>,
    ) -> Self {
        let mut search_service = SearchService::new();
        search_service.configure(pattern, timestamp_gte, timestamp_lt, session_id);
        
        Self {
            state: AppState::new(),
            search_service,
            session_service: SessionService::new(),
            clipboard_service: ClipboardService::new(),
            search_rx: None,
        }
    }
    
    /// Initialize the application
    pub fn init(&mut self, app: &mut Application<ComponentId, AppMessage, NoUserEvent>) -> anyhow::Result<()> {
        // Mount components
        app.mount(
            ComponentId::SearchInput,
            Box::new(SearchInput::new()),
            vec![],
        )?;
        
        app.mount(
            ComponentId::ResultList,
            Box::new(ResultList::new()),
            vec![],
        )?;
        
        app.mount(
            ComponentId::ResultDetail,
            Box::new(ResultDetail::new()),
            vec![],
        )?;
        
        app.mount(
            ComponentId::SessionViewer,
            Box::new(SessionViewer::new()),
            vec![],
        )?;
        
        app.mount(
            ComponentId::HelpDialog,
            Box::new(HelpDialog::new()),
            vec![],
        )?;
        
        // Set initial active component
        app.active(&ComponentId::SearchInput)?;
        
        // Update all components with initial state
        self.update_components(app)?;
        
        Ok(())
    }
    
    /// Update all components with current state
    pub fn update_components(&mut self, app: &mut Application<ComponentId, AppMessage, NoUserEvent>) -> anyhow::Result<()> {
        // Update SearchInput
        app.attr(
            &ComponentId::SearchInput,
            Attribute::Text,
            AttrValue::String(self.state.search_query.clone()),
        )?;
        
        app.attr(
            &ComponentId::SearchInput,
            Attribute::Custom("is_searching"),
            AttrValue::Flag(self.state.is_searching),
        )?;
        
        if let Some(filter) = &self.state.role_filter {
            app.attr(
                &ComponentId::SearchInput,
                Attribute::Custom("role_filter"),
                AttrValue::String(filter.clone()),
            )?;
        }
        
        if let Some(msg) = &self.state.status_message {
            app.attr(
                &ComponentId::SearchInput,
                Attribute::Custom("message"),
                AttrValue::String(msg.clone()),
            )?;
        }
        
        // Update ResultList
        // TODO: Handle complex types properly in tuirealm v3
        // app.attr(
        //     &ComponentId::ResultList,
        //     Attribute::Custom("results"),
        //     AttrValue::Payload(Box::new(self.state.search_results.clone())),
        // )?;
        
        app.attr(
            &ComponentId::ResultList,
            Attribute::Value,
            AttrValue::String(String::new()),
        )?;
        
        app.attr(
            &ComponentId::ResultList,
            Attribute::Custom("truncate"),
            AttrValue::Flag(self.state.truncation_enabled),
        )?;
        
        // Update ResultDetail
        if let Some(result) = &self.state.current_result {
            // TODO: Handle complex types properly in tuirealm v3
            // app.attr(
            //     &ComponentId::ResultDetail,
            //     Attribute::Custom("result"),
            //     AttrValue::Payload(Box::new(result.clone())),
            // )?;
        }
        
        app.attr(
            &ComponentId::ResultDetail,
            Attribute::Custom("scroll_offset"),
            AttrValue::String(String::new()),
        )?;
        
        if let Some(msg) = &self.state.status_message {
            app.attr(
                &ComponentId::ResultDetail,
                Attribute::Custom("message"),
                AttrValue::String(msg.clone()),
            )?;
        }
        
        // Update SessionViewer
        // TODO: Handle complex types properly in tuirealm v3
        // app.attr(
        //     &ComponentId::SessionViewer,
        //     Attribute::Custom("messages"),
        //     AttrValue::Payload(Box::new(self.state.session_messages.clone())),
        // )?;
        // 
        // app.attr(
        //     &ComponentId::SessionViewer,
        //     Attribute::Custom("filtered_indices"),
        //     AttrValue::Payload(Box::new(self.state.session_filtered_indices.clone())),
        // )?;
        
        app.attr(
            &ComponentId::SessionViewer,
            Attribute::Value,
            AttrValue::String(String::new()),
        )?;
        
        app.attr(
            &ComponentId::SessionViewer,
            Attribute::Custom("scroll_offset"),
            AttrValue::String(String::new()),
        )?;
        
        if let Some(order) = &self.state.session_order {
            // TODO: Handle complex types properly in tuirealm v3
            // app.attr(
            //     &ComponentId::SessionViewer,
            //     Attribute::Custom("order"),
            //     AttrValue::Payload(Box::new(order.clone())),
            // )?;
        }
        
        app.attr(
            &ComponentId::SessionViewer,
            Attribute::Custom("truncate"),
            AttrValue::Flag(self.state.truncation_enabled),
        )?;
        
        app.attr(
            &ComponentId::SessionViewer,
            Attribute::Custom("is_searching"),
            AttrValue::Flag(self.state.is_session_searching),
        )?;
        
        app.attr(
            &ComponentId::SessionViewer,
            Attribute::Custom("search_query"),
            AttrValue::String(self.state.session_query.clone()),
        )?;
        
        if let Some(id) = &self.state.session_id {
            app.attr(
                &ComponentId::SessionViewer,
                Attribute::Custom("session_id"),
                AttrValue::String(id.clone()),
            )?;
        }
        
        Ok(())
    }
    
    /// Execute search
    fn execute_search(&mut self) {
        if self.state.search_query.is_empty() {
            return;
        }
        
        self.state.is_searching = true;
        self.state.clear_message();
        
        let (tx, rx) = mpsc::channel();
        self.search_rx = Some(rx);
        
        // Execute search in background
        self.search_service.search_async(
            self.state.search_query.clone(),
            self.state.role_filter.clone(),
            tx,
        );
    }
    
    /// Load session messages
    fn load_session(&mut self, session_id: String) {
        match self.session_service.load_session(&session_id) {
            Ok(messages) => {
                self.state.session_id = Some(session_id);
                self.state.session_messages = messages;
                self.state.session_filtered_indices = (0..self.state.session_messages.len()).collect();
                self.state.selected_index = 0;
                self.state.session_scroll_offset = 0;
                self.state.session_query.clear();
                self.state.is_session_searching = false;
                self.state.change_mode(AppMode::SessionViewer);
            }
            Err(e) => {
                self.state.set_message(format!("Failed to load session: {}", e));
            }
        }
    }
    
    /// Filter session messages
    fn filter_session_messages(&mut self) {
        if self.state.session_query.is_empty() {
            self.state.session_filtered_indices = (0..self.state.session_messages.len()).collect();
        } else {
            let query_lower = self.state.session_query.to_lowercase();
            self.state.session_filtered_indices = self.state.session_messages
                .iter()
                .enumerate()
                .filter(|(_, msg)| msg.to_lowercase().contains(&query_lower))
                .map(|(i, _)| i)
                .collect();
        }
        
        // Reset selection
        if !self.state.session_filtered_indices.is_empty() {
            self.state.selected_index = 0;
            self.state.session_scroll_offset = 0;
        }
    }
    
    /// Handle copy operations
    fn handle_copy(&mut self, copy_type: &str) {
        let content = match (copy_type, &self.state.mode) {
            ("message", AppMode::ResultDetail) => {
                self.state.current_result.as_ref().map(|r| r.text.clone())
            }
            ("session", AppMode::ResultDetail) => {
                self.state.current_result.as_ref().map(|r| r.session_id.clone())
            }
            ("timestamp", AppMode::ResultDetail) => {
                self.state.current_result.as_ref().map(|r| r.timestamp.clone())
            }
            ("json", AppMode::ResultDetail) => {
                // TODO: Implement raw JSON extraction
                None
            }
            ("message", AppMode::SessionViewer) => {
                if let Some(idx) = self.state.session_filtered_indices.get(self.state.selected_index) {
                    self.state.session_messages.get(*idx).cloned()
                } else {
                    None
                }
            }
            ("json", AppMode::SessionViewer) => {
                if let Some(idx) = self.state.session_filtered_indices.get(self.state.selected_index) {
                    self.state.session_messages.get(*idx).cloned()
                } else {
                    None
                }
            }
            ("session_id", AppMode::SessionViewer) => {
                self.state.session_id.clone()
            }
            _ => None,
        };
        
        if let Some(text) = content {
            match self.clipboard_service.copy(&text) {
                Ok(_) => {
                    self.state.set_message(format!("Copied to clipboard"));
                }
                Err(e) => {
                    self.state.set_message(format!("Copy failed: {}", e));
                }
            }
        }
    }
}

impl Update<AppMessage> for App {
    fn update(&mut self, msg: Option<AppMessage>) -> Option<AppMessage> {
        // Check for search results
        if let Some(rx) = &self.search_rx {
            if let Ok(results) = rx.try_recv() {
                self.state.search_results = results;
                self.state.is_searching = false;
                self.state.selected_index = 0;
                self.search_rx = None;
                
                if self.state.search_results.is_empty() {
                    self.state.set_message("No results found".to_string());
                } else {
                    self.state.clear_message();
                }
            }
        }
        
        // Handle messages
        if let Some(msg) = msg {
            match msg {
                AppMessage::Quit => {
                    self.state.should_quit = true;
                }
                
                AppMessage::ChangeMode(mode) => {
                    self.state.change_mode(mode);
                }
                
                AppMessage::SearchQueryChanged(query) => {
                    self.state.search_query = query;
                }
                
                AppMessage::SearchRequested => {
                    self.execute_search();
                }
                
                AppMessage::SearchCompleted => {
                    // Results are already set via the channel check above
                    if self.state.search_results.is_empty() {
                        self.state.set_message("No results found".to_string());
                    }
                }
                
                AppMessage::ToggleRoleFilter => {
                    self.state.cycle_role_filter();
                }
                
                AppMessage::ResultUp => {
                    if self.state.selected_index > 0 {
                        self.state.selected_index -= 1;
                    }
                }
                
                AppMessage::ResultDown => {
                    if self.state.selected_index + 1 < self.state.search_results.len() {
                        self.state.selected_index += 1;
                    }
                }
                
                AppMessage::ResultPageUp => {
                    self.state.selected_index = self.state.selected_index.saturating_sub(10);
                }
                
                AppMessage::ResultPageDown => {
                    let max_index = self.state.search_results.len().saturating_sub(1);
                    self.state.selected_index = (self.state.selected_index + 10).min(max_index);
                }
                
                AppMessage::ResultHome => {
                    self.state.selected_index = 0;
                }
                
                AppMessage::ResultEnd => {
                    self.state.selected_index = self.state.search_results.len().saturating_sub(1);
                }
                
                AppMessage::EnterResultDetail(index) => {
                    if let Some(result) = self.state.search_results.get(index) {
                        self.state.current_result = Some(result.clone());
                        self.state.detail_scroll_offset = 0;
                        self.state.clear_message();
                        self.state.change_mode(AppMode::ResultDetail);
                    }
                }
                
                AppMessage::ExitResultDetail => {
                    self.state.return_to_previous_mode();
                }
                
                AppMessage::DetailScrollUp => {
                    self.state.detail_scroll_offset = self.state.detail_scroll_offset.saturating_sub(1);
                }
                
                AppMessage::DetailScrollDown => {
                    self.state.detail_scroll_offset += 1;
                }
                
                AppMessage::DetailPageUp => {
                    self.state.detail_scroll_offset = self.state.detail_scroll_offset.saturating_sub(10);
                }
                
                AppMessage::DetailPageDown => {
                    self.state.detail_scroll_offset += 10;
                }
                
                AppMessage::EnterSessionViewer(session_id) => {
                    self.load_session(session_id);
                }
                
                AppMessage::ExitSessionViewer => {
                    self.state.return_to_previous_mode();
                }
                
                AppMessage::SessionScrollUp => {
                    if self.state.selected_index > 0 {
                        self.state.selected_index -= 1;
                    }
                }
                
                AppMessage::SessionScrollDown => {
                    if self.state.selected_index + 1 < self.state.session_filtered_indices.len() {
                        self.state.selected_index += 1;
                    }
                }
                
                AppMessage::SessionPageUp => {
                    self.state.selected_index = self.state.selected_index.saturating_sub(10);
                }
                
                AppMessage::SessionPageDown => {
                    let max_index = self.state.session_filtered_indices.len().saturating_sub(1);
                    self.state.selected_index = (self.state.selected_index + 10).min(max_index);
                }
                
                AppMessage::SessionSearchStart => {
                    self.state.is_session_searching = true;
                    self.state.session_query.clear();
                }
                
                AppMessage::SessionSearchEnd => {
                    self.state.is_session_searching = false;
                }
                
                AppMessage::SessionQueryChanged(query) => {
                    self.state.session_query = query;
                    self.filter_session_messages();
                }
                
                AppMessage::SessionToggleOrder => {
                    self.state.session_order = match self.state.session_order {
                        None => Some(SessionOrder::Descending),
                        Some(SessionOrder::Descending) => Some(SessionOrder::Ascending),
                        Some(SessionOrder::Ascending) => Some(SessionOrder::Original),
                        Some(SessionOrder::Original) => None,
                    };
                    
                    // Reverse messages if needed
                    if matches!(self.state.session_order, Some(SessionOrder::Descending) | Some(SessionOrder::Ascending)) {
                        self.state.session_messages.reverse();
                        self.filter_session_messages();
                    }
                }
                
                AppMessage::ToggleTruncation => {
                    self.state.truncation_enabled = !self.state.truncation_enabled;
                }
                
                AppMessage::ShowHelp => {
                    self.state.change_mode(AppMode::Help);
                }
                
                AppMessage::ExitHelp => {
                    self.state.return_to_previous_mode();
                }
                
                AppMessage::CopyMessage => {
                    self.handle_copy("message");
                }
                
                AppMessage::CopySession => {
                    self.handle_copy("session");
                }
                
                AppMessage::CopyTimestamp => {
                    self.handle_copy("timestamp");
                }
                
                AppMessage::CopyRawJson => {
                    self.handle_copy("json");
                }
                
                AppMessage::CopySessionId => {
                    self.handle_copy("session_id");
                }
                
                // Missing arms
                AppMessage::SearchFailed(err) => {
                    self.state.set_message(format!("Search failed: {}", err));
                }
                
                AppMessage::ResultSelect(_) => {
                    // Already handled elsewhere
                }
                
                AppMessage::ClipboardSuccess(msg) => {
                    self.state.set_message(msg);
                }
                
                AppMessage::ClipboardFailed(err) => {
                    self.state.set_message(err);
                }
                
                AppMessage::ShowMessage(msg) => {
                    self.state.set_message(msg);
                }
                
                AppMessage::ClearMessage => {
                    self.state.clear_message();
                }
                
                AppMessage::DebouncedSearchReady(_) => {
                    // Already handled elsewhere
                }
                
                AppMessage::SessionLoaded(_, _) => {
                    // Already handled elsewhere
                }
                
                AppMessage::SessionLoadFailed(err) => {
                    self.state.set_message(format!("Session load failed: {}", err));
                }
            }
        }
        
        // Check if we should quit
        if self.state.should_quit {
            return Some(AppMessage::Quit);
        }
        
        None
    }
}