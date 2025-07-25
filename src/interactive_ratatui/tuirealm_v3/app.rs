use std::sync::mpsc;
use tuirealm::props::{AttrValue, Attribute};
use tuirealm::{Application, NoUserEvent, Update};
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::Frame;

use crate::query::condition::SearchResult;
use super::models::SessionOrder;
use super::messages::{AppMessage, AppMode, ComponentId};
use super::state::AppState;
use super::components::{SearchInput, ResultList, ResultDetail, SessionViewer, HelpDialog, ErrorDialog};
use super::services::{SearchService, SessionService, ClipboardService};
use super::error::{AppError, AppResult};
use super::type_safe_wrapper::{SearchResults, SessionMessages, helpers};

#[cfg(test)]
#[path = "app_test.rs"]
mod tests;

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
    pub fn init(&mut self, app: &mut Application<ComponentId, AppMessage, NoUserEvent>) -> AppResult<()> {
        // Mount components
        app.mount(
            ComponentId::SearchInput,
            Box::new(SearchInput::new()),
            vec![],
        ).map_err(|e| AppError::ComponentInitError {
            component: "SearchInput".to_string(),
            details: e.to_string(),
        })?;
        
        app.mount(
            ComponentId::ResultList,
            Box::new(ResultList::new()),
            vec![],
        ).map_err(|e| AppError::ComponentInitError {
            component: "ResultList".to_string(),
            details: e.to_string(),
        })?;
        
        app.mount(
            ComponentId::ResultDetail,
            Box::new(ResultDetail::new()),
            vec![],
        ).map_err(|e| AppError::ComponentInitError {
            component: "ResultDetail".to_string(),
            details: e.to_string(),
        })?;
        
        app.mount(
            ComponentId::SessionViewer,
            Box::new(SessionViewer::new()),
            vec![],
        ).map_err(|e| AppError::ComponentInitError {
            component: "SessionViewer".to_string(),
            details: e.to_string(),
        })?;
        
        app.mount(
            ComponentId::HelpDialog,
            Box::new(HelpDialog::new()),
            vec![],
        ).map_err(|e| AppError::ComponentInitError {
            component: "HelpDialog".to_string(),
            details: e.to_string(),
        })?;
        
        app.mount(
            ComponentId::ErrorDialog,
            Box::new(ErrorDialog::new()),
            vec![],
        ).map_err(|e| AppError::ComponentInitError {
            component: "ErrorDialog".to_string(),
            details: e.to_string(),
        })?;
        
        // GlobalShortcuts is not mounted as a component
        // Instead, each component handles global shortcuts internally
        
        
        // Set initial active component
        app.active(&ComponentId::SearchInput).map_err(|e| AppError::ComponentInitError {
            component: "SearchInput".to_string(),
            details: format!("Failed to set active: {e}"),
        })?;
        
        // Update all components with initial state
        self.update_components(app)?;
        
        Ok(())
    }
    
    /// Helper to update component attribute with proper error handling
    fn update_attr(
        app: &mut Application<ComponentId, AppMessage, NoUserEvent>,
        component: &ComponentId,
        attr: Attribute,
        value: AttrValue,
    ) -> AppResult<()> {
        app.attr(component, attr, value).map_err(|e| AppError::ComponentUpdateError {
            component: format!("{component:?}"),
            details: e.to_string(),
        })
    }
    
    /// Update all components with current state
    pub fn update_components(&mut self, app: &mut Application<ComponentId, AppMessage, NoUserEvent>) -> AppResult<()> {
        // Update SearchInput
        Self::update_attr(
            app,
            &ComponentId::SearchInput,
            Attribute::Text,
            AttrValue::String(self.state.search_query.clone()),
        )?;
        
        Self::update_attr(
            app,
            &ComponentId::SearchInput,
            Attribute::Custom("is_searching"),
            AttrValue::Flag(self.state.is_searching),
        )?;
        
        // Show typing indicator when there's a pending search
        Self::update_attr(
            app,
            &ComponentId::SearchInput,
            Attribute::Custom("is_typing"),
            AttrValue::Flag(self.state.pending_search_query.is_some()),
        )?;
        
        if let Some(filter) = &self.state.role_filter {
            Self::update_attr(
                app,
                &ComponentId::SearchInput,
                Attribute::Custom("role_filter"),
                AttrValue::String(filter.clone()),
            )?;
        }
        
        if let Some(msg) = &self.state.status_message {
            Self::update_attr(
                app,
                &ComponentId::SearchInput,
                Attribute::Custom("message"),
                AttrValue::String(msg.clone()),
            )?;
        }
        
        // Update ResultList
        // Pass search results using type-safe wrapper
        helpers::set_type_safe_attr(
            app,
            &ComponentId::ResultList,
            Attribute::Custom("search_results"),
            SearchResults(self.state.search_results.clone()),
        ).map_err(|e| AppError::ComponentUpdateError {
            component: "ResultList".to_string(),
            details: e,
        })?;
        
        Self::update_attr(
            app,
            &ComponentId::ResultList,
            Attribute::Value,
            AttrValue::String(self.state.selected_index.to_string()),
        )?;
        
        Self::update_attr(
            app,
            &ComponentId::ResultList,
            Attribute::Custom("truncate"),
            AttrValue::Flag(self.state.truncation_enabled),
        )?;
        
        // Update ResultDetail
        if let Some(result) = &self.state.current_result {
            Self::update_attr(
                app,
                &ComponentId::ResultDetail,
                Attribute::Custom("session_id"),
                AttrValue::String(result.session_id.clone()),
            )?;
            
            Self::update_attr(
                app,
                &ComponentId::ResultDetail,
                Attribute::Custom("file"),
                AttrValue::String(result.file.clone()),
            )?;
            
            Self::update_attr(
                app,
                &ComponentId::ResultDetail,
                Attribute::Custom("timestamp"),
                AttrValue::String(result.timestamp.clone()),
            )?;
            
            Self::update_attr(
                app,
                &ComponentId::ResultDetail,
                Attribute::Custom("role"),
                AttrValue::String(result.role.clone()),
            )?;
            
            Self::update_attr(
                app,
                &ComponentId::ResultDetail,
                Attribute::Custom("text"),
                AttrValue::String(result.text.clone()),
            )?;
            
            if let Some(raw_json) = &result.raw_json {
                Self::update_attr(
                    app,
                    &ComponentId::ResultDetail,
                    Attribute::Custom("raw_json"),
                    AttrValue::String(raw_json.clone()),
                )?;
            }
        }
        
        Self::update_attr(
            app,
            &ComponentId::ResultDetail,
            Attribute::Custom("scroll_offset"),
            AttrValue::String(self.state.detail_scroll_offset.to_string()),
        )?;
        
        if let Some(msg) = &self.state.status_message {
            Self::update_attr(
                app,
                &ComponentId::ResultDetail,
                Attribute::Custom("message"),
                AttrValue::String(msg.clone()),
            )?;
        }
        
        // Update SessionViewer
        Self::update_attr(
            app,
            &ComponentId::SessionViewer,
            Attribute::Custom("message_count"),
            AttrValue::String(self.state.session_messages.len().to_string()),
        )?;
        
        let session_texts: Vec<String> = if self.state.session_filtered_indices.is_empty() {
            self.state.session_messages.clone()
        } else {
            self.state.session_filtered_indices
                .iter()
                .filter_map(|&idx| self.state.session_messages.get(idx).cloned())
                .collect()
        };
        
        helpers::set_type_safe_attr(
            app,
            &ComponentId::SessionViewer,
            Attribute::Custom("session_texts"),
            SessionMessages(session_texts),
        ).map_err(|e| AppError::ComponentUpdateError {
            component: "SessionViewer".to_string(),
            details: e,
        })?;
        
        Self::update_attr(
            app,
            &ComponentId::SessionViewer,
            Attribute::Value,
            AttrValue::String(String::new()),
        )?;
        
        Self::update_attr(
            app,
            &ComponentId::SessionViewer,
            Attribute::Custom("scroll_offset"),
            AttrValue::String(self.state.session_scroll_offset.to_string()),
        )?;
        
        if let Some(order) = &self.state.session_order {
            Self::update_attr(
                app,
                &ComponentId::SessionViewer,
                Attribute::Custom("order"),
                AttrValue::String(match order {
                    SessionOrder::Ascending => "asc",
                    SessionOrder::Descending => "desc",
                    SessionOrder::Original => "original",
                }.to_string()),
            )?;
        }
        
        Self::update_attr(
            app,
            &ComponentId::SessionViewer,
            Attribute::Custom("truncate"),
            AttrValue::Flag(self.state.truncation_enabled),
        )?;
        
        Self::update_attr(
            app,
            &ComponentId::SessionViewer,
            Attribute::Custom("is_searching"),
            AttrValue::Flag(self.state.is_session_searching),
        )?;
        
        Self::update_attr(
            app,
            &ComponentId::SessionViewer,
            Attribute::Custom("search_query"),
            AttrValue::String(self.state.session_query.clone()),
        )?;
        
        if let Some(id) = &self.state.session_id {
            Self::update_attr(
                app,
                &ComponentId::SessionViewer,
                Attribute::Custom("session_id"),
                AttrValue::String(id.clone()),
            )?;
        }
        
        // GlobalShortcuts is not used anymore
        
        Ok(())
    }
    
    /// Execute search
    fn execute_search(&mut self) {
        if self.state.search_query.is_empty() {
            return;
        }
        
        // Don't start a new search if one is already running
        if self.state.is_searching {
            return;
        }
        
        self.state.is_searching = true;
        self.state.clear_message();
        
        // Drop the previous receiver to cancel any ongoing search
        self.search_rx = None;
        
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
                self.state.set_message(format!("Failed to load session: {e}"));
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
    
    /// Check if debounced search is ready
    pub fn check_debounced_search(&mut self) -> Option<AppMessage> {
        if let (Some(last_update), Some(query)) = (&self.state.last_search_update, &self.state.pending_search_query) {
            if last_update.elapsed() >= std::time::Duration::from_millis(300) {
                // Clear the pending query to prevent repeated execution
                let query_clone = query.clone();
                self.state.pending_search_query = None;
                self.state.last_search_update = None;
                return Some(AppMessage::DebouncedSearchReady(query_clone));
            }
        }
        None
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
                self.state.current_result.as_ref()
                    .and_then(|r| r.raw_json.clone())
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
                    self.state.set_message("Copied to clipboard".to_string());
                }
                Err(e) => {
                    self.state.set_message(format!("Copy failed: {e}"));
                }
            }
        }
    }
    
    /// Render the layout based on current mode
    pub fn render_layout(&self, app: &mut Application<ComponentId, AppMessage, NoUserEvent>, f: &mut Frame) {
        match self.state.mode {
            AppMode::Search => {
                // Split into search input and results
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .margin(0)
                    .constraints([
                        Constraint::Length(3),   // Search input
                        Constraint::Min(0),      // Results list
                        Constraint::Length(1),   // Status bar
                    ])
                    .split(f.area());
                
                // Render search input
                app.view(&ComponentId::SearchInput, f, chunks[0]);
                
                // Render results list
                app.view(&ComponentId::ResultList, f, chunks[1]);
                
                // Render status bar (optional)
                // TODO: Add status bar component if needed
            }
            
            AppMode::ResultDetail => {
                // Full screen for result detail
                app.view(&ComponentId::ResultDetail, f, f.area());
            }
            
            AppMode::SessionViewer => {
                // Full screen for session viewer
                app.view(&ComponentId::SessionViewer, f, f.area());
            }
            
            AppMode::Help => {
                // Render help dialog over the current view
                match self.state.previous_mode {
                    Some(AppMode::Search) => {
                        // Render search mode underneath
                        let chunks = Layout::default()
                            .direction(Direction::Vertical)
                            .margin(0)
                            .constraints([
                                Constraint::Length(3),
                                Constraint::Min(0),
                                Constraint::Length(1),
                            ])
                            .split(f.area());
                        
                        app.view(&ComponentId::SearchInput, f, chunks[0]);
                        app.view(&ComponentId::ResultList, f, chunks[1]);
                    }
                    Some(AppMode::ResultDetail) => {
                        app.view(&ComponentId::ResultDetail, f, f.area());
                    }
                    Some(AppMode::SessionViewer) => {
                        app.view(&ComponentId::SessionViewer, f, f.area());
                    }
                    _ => {}
                }
                
                // Render help dialog on top
                app.view(&ComponentId::HelpDialog, f, f.area());
            }
            
            AppMode::Error => {
                // Render error dialog over the current view
                match self.state.previous_mode {
                    Some(AppMode::Search) => {
                        let chunks = Layout::default()
                            .direction(Direction::Vertical)
                            .margin(0)
                            .constraints([
                                Constraint::Length(3),
                                Constraint::Min(0),
                                Constraint::Length(1),
                            ])
                            .split(f.area());
                        
                        app.view(&ComponentId::SearchInput, f, chunks[0]);
                        app.view(&ComponentId::ResultList, f, chunks[1]);
                    }
                    Some(AppMode::ResultDetail) => {
                        app.view(&ComponentId::ResultDetail, f, f.area());
                    }
                    Some(AppMode::SessionViewer) => {
                        app.view(&ComponentId::SessionViewer, f, f.area());
                    }
                    _ => {}
                }
                
                // Render error dialog on top
                app.view(&ComponentId::ErrorDialog, f, f.area());
            }
        }
        
        // Process global shortcuts only when they don't interfere with input
        // GlobalShortcuts is handled through render but not for keyboard events in Search mode
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
                    self.state.search_query = query.clone();
                    self.state.pending_search_query = Some(query);
                    self.state.last_search_update = Some(std::time::Instant::now());
                }
                
                AppMessage::SearchRequested => {
                    self.execute_search();
                }
                
                AppMessage::SearchCompleted => {
                    // Results are already set via the channel check above
                    self.state.is_searching = false;
                    if self.state.search_results.is_empty() {
                        self.state.set_message("No results found".to_string());
                    }
                }
                
                AppMessage::ToggleRoleFilter => {
                    self.state.cycle_role_filter();
                }
                
                AppMessage::ResultUp => {
                    // Ensure index is valid first
                    if self.state.selected_index >= self.state.search_results.len() && !self.state.search_results.is_empty() {
                        self.state.selected_index = self.state.search_results.len() - 1;
                    }
                    // Then try to move up
                    if self.state.selected_index > 0 {
                        self.state.selected_index -= 1;
                    }
                }
                
                AppMessage::ResultDown => {
                    // Ensure index is valid first
                    if self.state.selected_index >= self.state.search_results.len() && !self.state.search_results.is_empty() {
                        self.state.selected_index = self.state.search_results.len() - 1;
                    }
                    // Then try to move down
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
                    } else {
                        self.state.set_message("No result selected".to_string());
                    }
                }
                
                AppMessage::ExitResultDetail => {
                    if self.state.previous_mode.is_none() {
                        self.state.change_mode(AppMode::Search);
                    } else {
                        self.state.return_to_previous_mode();
                    }
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
                    self.state.set_message(format!("Search failed: {err}"));
                }
                
                AppMessage::ResultSelect(index) => {
                    // Use current selection if index is MAX
                    let target_index = if index == usize::MAX {
                        self.state.selected_index
                    } else {
                        index
                    };
                    
                    if let Some(result) = self.state.search_results.get(target_index) {
                        self.state.current_result = Some(result.clone());
                        self.state.detail_scroll_offset = 0;
                        self.state.clear_message();
                        self.state.change_mode(AppMode::ResultDetail);
                    } else {
                        self.state.set_message("No result selected".to_string());
                    }
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
                
                AppMessage::DebouncedSearchReady(query) => {
                    // Only execute search if the query matches what we're expecting
                    if self.state.search_query == query {
                        self.execute_search();
                    }
                }
                
                AppMessage::SessionLoaded(_, _) => {
                    // Already handled elsewhere
                }
                
                AppMessage::SessionLoadFailed(err) => {
                    self.state.set_message(format!("Session load failed: {err}"));
                }
                
                AppMessage::ShowError(_error_type, _details) => {
                    self.state.change_mode(AppMode::Error);
                    // Will be handled by ErrorDialog component
                }
                
                AppMessage::CloseError => {
                    self.state.return_to_previous_mode();
                }
                
                AppMessage::RetryLastOperation => {
                    self.state.return_to_previous_mode();
                    // TODO: Implement retry logic based on last operation
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