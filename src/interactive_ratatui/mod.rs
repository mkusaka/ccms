use anyhow::{Context, Result};
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, poll},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};
use smol::channel::{Receiver, Sender};
use std::io::{self, Stdout};
use std::sync::Arc;
use std::time::Duration;

use crate::SearchOptions;

mod application;
mod constants;
mod debug;
pub mod domain;
pub mod ui;

#[cfg(test)]
mod help_navigation_test;
#[cfg(test)]
mod integration_tests;
// #[cfg(test)]
// mod session_view_integration_test; // No longer used - using unified session viewer
#[cfg(test)]
mod tests;
#[cfg(test)]
mod session_preview_test;

use self::application::search_service::SearchService;
use self::constants::*;
use self::domain::models::{Mode, SearchRequest, SearchResponse, SearchOrder, SessionOrder};
use self::ui::{
    app_state::AppState, commands::Command, components::Component, events::Message,
    renderer::Renderer,
};

pub struct InteractiveSearch {
    state: AppState,
    renderer: Renderer,
    search_service: Arc<SearchService>,
    search_sender: Option<Sender<SearchRequest>>,
    search_receiver: Option<Receiver<SearchResponse>>,
    search_task: Option<smol::Task<()>>,
    current_search_id: u64,
    last_search_timer: Option<std::time::Instant>,
    scheduled_search_delay: Option<u64>,
    pattern: String,
    last_ctrl_c_press: Option<std::time::Instant>,
    message_timer: Option<std::time::Instant>,
    message_clear_delay: u64,
}

impl InteractiveSearch {
    pub fn new(options: SearchOptions) -> Self {
        let search_service = Arc::new(SearchService::new(options.clone()));

        Self {
            state: AppState::new(),
            renderer: Renderer::new(),
            search_service,
            search_sender: None,
            search_receiver: None,
            search_task: None,
            current_search_id: 0,
            last_search_timer: None,
            scheduled_search_delay: None,
            pattern: String::new(),
            last_ctrl_c_press: None,
            message_timer: None,
            message_clear_delay: MESSAGE_CLEAR_DELAY_MS,
        }
    }

    pub fn run(&mut self, pattern: &str) -> Result<()> {
        smol::block_on(self.run_async(pattern))
    }

    async fn run_async(&mut self, pattern: &str) -> Result<()> {
        let _ = crate::interactive_ratatui::debug::clear_debug_log();
        let _ = crate::interactive_ratatui::debug::write_debug_log("=== Starting interactive search session ===");
        
        self.pattern = pattern.to_string();
        let mut terminal = self.setup_terminal()?;

        // Start search worker task
        let (tx, rx, task) = self.start_search_worker();
        self.search_sender = Some(tx);
        self.search_receiver = Some(rx);
        self.search_task = Some(task);

        // Initial search (even with empty pattern to show all results)
        // Note: pattern is stored internally but not shown in search bar
        self.execute_command(Command::ExecuteSearch).await;

        let result = self.run_app(&mut terminal, pattern).await;

        // Clean up search task
        if let Some(task) = self.search_task.take() {
            task.cancel().await;
        }

        self.cleanup_terminal(&mut terminal)?;
        result
    }

    fn setup_terminal(&self) -> Result<Terminal<CrosstermBackend<Stdout>>> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;
        Ok(terminal)
    }

    fn cleanup_terminal(&self, terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<()> {
        disable_raw_mode()?;
        execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
        terminal.show_cursor()?;
        Ok(())
    }

    async fn run_app(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<Stdout>>,
        _pattern: &str,
    ) -> Result<()> {
        loop {
            terminal.draw(|f| {
                self.renderer.render(f, &self.state);
            })?;

            // Check for search results
            if let Some(receiver) = &self.search_receiver {
                if let Ok(response) = receiver.try_recv() {
                    if response.id == self.state.search.current_search_id {
                        let msg = Message::SearchCompleted(response.results);
                        self.handle_message(msg);
                    }
                }
            }

            // Check for scheduled search
            if let Some(delay) = self.scheduled_search_delay {
                if let Some(timer) = self.last_search_timer {
                    if timer.elapsed() >= Duration::from_millis(delay) {
                        self.scheduled_search_delay = None;
                        self.last_search_timer = None;
                        self.execute_command(Command::ExecuteSearch).await;
                    }
                }
            }

            // Check for scheduled message clear
            if let Some(timer) = self.message_timer {
                if timer.elapsed() >= Duration::from_millis(self.message_clear_delay) {
                    self.message_timer = None;
                    self.execute_command(Command::ClearMessage).await;
                }
            }

            if poll(Duration::from_millis(EVENT_POLL_INTERVAL_MS))? {
                if let Event::Key(key) = event::read()? {
                    let should_quit = self.handle_input(key)?;
                    if should_quit {
                        break;
                    }
                }
            }
        }
        Ok(())
    }

    fn handle_input(&mut self, key: KeyEvent) -> Result<bool> {
        use crossterm::event::KeyModifiers;

        // Global Ctrl+C handling for exit
        if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
            if let Some(last_press) = self.last_ctrl_c_press {
                // Check if second press is within 1 second
                if last_press.elapsed() < Duration::from_secs(DOUBLE_CTRL_C_TIMEOUT_SECS) {
                    // Exit application
                    return Ok(true);
                }
            }
            // First press or timeout expired
            self.last_ctrl_c_press = Some(std::time::Instant::now());
            self.state.ui.message = Some("Press Ctrl+C again to exit".to_string());
            // Set timer to clear message
            self.message_timer = Some(std::time::Instant::now());
            // Reset any other Ctrl+C tracking that might be elsewhere
            return Ok(false);
        }

        // Reset Ctrl+C tracking on any other key press
        if self.last_ctrl_c_press.is_some() {
            self.last_ctrl_c_press = None;
        }

        // Global keys
        match key.code {
            KeyCode::Char('?') if self.state.mode != Mode::Help => {
                self.handle_message(Message::ShowHelp);
                return Ok(false);
            }
            KeyCode::Char('t') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                // Send appropriate preview message based on current mode
                let message = match self.state.mode {
                    Mode::Search => Message::TogglePreview,
                    Mode::SessionViewer => Message::ToggleSessionPreview,
                    _ => return Ok(false), // No preview for other modes
                };
                let _ = crate::interactive_ratatui::debug::write_debug_log(
                    &format!("Ctrl+T pressed in mode {:?}, sending {:?}", self.state.mode, message)
                );
                self.handle_message(message);
                return Ok(false);
            }
            // Navigation shortcuts with Alt modifier
            KeyCode::Left if key.modifiers.contains(KeyModifiers::ALT) => {
                self.handle_message(Message::NavigateBack);
                return Ok(false);
            }
            KeyCode::Right if key.modifiers.contains(KeyModifiers::ALT) => {
                self.handle_message(Message::NavigateForward);
                return Ok(false);
            }
            _ => {}
        }

        // Mode-specific input handling
        let message = match self.state.mode {
            Mode::Search => self.handle_search_mode_input(key),
            Mode::MessageDetail => self.renderer.get_message_detail_mut().handle_key(key),
            Mode::SessionViewer => self.renderer.get_session_viewer_mut().handle_key(key),
            Mode::Help => self.renderer.get_help_dialog_mut().handle_key(key),
        };

        if let Some(msg) = message {
            self.handle_message(msg);
        }

        Ok(false)
    }

    fn handle_search_mode_input(&mut self, key: KeyEvent) -> Option<Message> {
        use crossterm::event::KeyModifiers;

        match key.code {
            // Skip Tab key processing if Ctrl is pressed (to allow Ctrl+I navigation)
            KeyCode::Tab if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                Some(Message::ToggleRoleFilter)
            }
            // Handle Ctrl+S specifically for session viewer
            KeyCode::Char('s') if key.modifiers == KeyModifiers::CONTROL => {
                self.renderer.get_result_list_mut().handle_key(key)
            }
            // Handle Ctrl+O for toggling search order
            KeyCode::Char('o') if key.modifiers == KeyModifiers::CONTROL => {
                Some(Message::ToggleSearchOrder)
            }
            KeyCode::Up
            | KeyCode::Down
            | KeyCode::PageUp
            | KeyCode::PageDown
            | KeyCode::Home
            | KeyCode::End
            | KeyCode::Enter => self.renderer.get_result_list_mut().handle_key(key),
            // Ctrl+P/N navigation
            KeyCode::Char('p') | KeyCode::Char('n') if key.modifiers == KeyModifiers::CONTROL => {
                self.renderer.get_result_list_mut().handle_key(key)
            }
            // Handle Ctrl+u/d for half-page scrolling
            KeyCode::Char('u') | KeyCode::Char('d') if key.modifiers == KeyModifiers::CONTROL => {
                self.renderer.get_result_list_mut().handle_key(key)
            }
            KeyCode::Esc => {
                // Try result list first (for closing preview), then fall back to search bar
                self.renderer
                    .get_result_list_mut()
                    .handle_key(key)
                    .or_else(|| self.renderer.get_search_bar_mut().handle_key(key))
            }
            _ => self.renderer.get_search_bar_mut().handle_key(key),
        }
    }

    fn handle_message(&mut self, message: Message) {
        let command = self.state.update(message);
        smol::block_on(self.execute_command(command));
    }

    async fn execute_command(&mut self, command: Command) {
        match command {
            Command::None => {}
            Command::ExecuteSearch => {
                self.execute_search().await;
            }
            Command::ExecuteSessionSearch => {
                self.execute_session_search().await;
            }
            Command::ScheduleSearch(delay) => {
                self.last_search_timer = Some(std::time::Instant::now());
                self.scheduled_search_delay = Some(delay);
            }
            Command::LoadSession(file_path) => {
                self.load_session_messages(&file_path);
            }
            Command::CopyToClipboard(content) => {
                let (text, copy_message) = match content {
                    ui::events::CopyContent::FilePath(path) => {
                        (path, "✓ Copied file path".to_string())
                    }
                    ui::events::CopyContent::ProjectPath(path) => {
                        (path, "✓ Copied project path".to_string())
                    }
                    ui::events::CopyContent::SessionId(id) => {
                        (id, "✓ Copied session ID".to_string())
                    }
                    ui::events::CopyContent::MessageContent(msg) => {
                        (msg, "✓ Copied message text".to_string())
                    }
                    ui::events::CopyContent::JsonData(json) => {
                        (json, "✓ Copied as JSON".to_string())
                    }
                    ui::events::CopyContent::FullMessageDetails(details) => {
                        (details, "✓ Copied full message details".to_string())
                    }
                };

                if let Err(e) = self.copy_to_clipboard(&text) {
                    self.state.ui.message = Some(format!("Failed to copy: {e}"));
                } else {
                    self.state.ui.message = Some(copy_message);

                    // Schedule message clear after delay
                    self.message_timer = Some(std::time::Instant::now());
                }
            }
            Command::ShowMessage(msg) => {
                self.state.ui.message = Some(msg);
            }
            Command::ClearMessage => {
                self.state.ui.message = None;
                self.message_timer = None;
            }
            Command::ScheduleClearMessage(delay) => {
                self.message_timer = Some(std::time::Instant::now());
                self.message_clear_delay = delay;
            }
        }
    }

    async fn execute_search(&mut self) {
        // Allow empty query to show all results
        // if self.state.search.query.is_empty() {
        //     self.state.search.results.clear();
        //     self.state.search.is_searching = false;
        //     return;
        // }

        self.current_search_id += 1;
        self.state.search.current_search_id = self.current_search_id;
        self.state.search.is_searching = true;

        if let Some(sender) = &self.search_sender {
            let request = SearchRequest {
                id: self.current_search_id,
                query: self.state.search.query.clone(),
                role_filter: self.state.search.role_filter.clone(),
                pattern: self.pattern.clone(),
                order: self.state.search.order,
            };
            let _ = sender.send(request).await;
        }
    }

    fn load_session_messages(&mut self, file_path: &str) {
        // Use search service to load session messages with session_id filter
        if let Some(session_id) = &self.state.session.session_id {
            let request = SearchRequest {
                id: self.state.search.current_search_id,
                query: self.state.session.query.clone(),
                pattern: file_path.to_string(),
                role_filter: self.state.session.role_filter.clone(),
                order: match self.state.session.order {
                    SessionOrder::Ascending => SearchOrder::Ascending,
                    SessionOrder::Descending => SearchOrder::Descending,
                },
            };
            
            match self.search_service.search_session(request, session_id.clone()) {
                Ok(response) => {
                    self.state.session.search_results = response.results;
                    // Clear old messages - will be removed later after full migration
                    self.state.session.messages = vec![];
                    self.state.session.filtered_indices = vec![];
                }
                Err(e) => {
                    self.state.ui.message = Some(format!("Failed to load session: {e}"));
                }
            }
        } else {
            self.state.ui.message = Some("No session ID available".to_string());
        }
    }

    async fn execute_session_search(&mut self) {
        // Execute search with session_id filter
        if let Some(session_id) = &self.state.session.session_id {
            if let Some(file_path) = &self.state.session.file_path {
                let request = SearchRequest {
                    id: self.state.search.current_search_id,
                    query: self.state.session.query.clone(),
                    pattern: file_path.clone(),
                    role_filter: self.state.session.role_filter.clone(),
                    order: match self.state.session.order {
                        SessionOrder::Ascending => SearchOrder::Ascending,
                        SessionOrder::Descending => SearchOrder::Descending,
                    },
                };
                
                match self.search_service.search_session(request, session_id.clone()) {
                    Ok(response) => {
                        self.state.session.search_results = response.results;
                        // Clear old messages - will be removed later after full migration
                        self.state.session.messages = vec![];
                        self.state.session.filtered_indices = vec![];
                        self.state.ui.message = None;
                    }
                    Err(e) => {
                        self.state.ui.message = Some(format!("Failed to search session: {e}"));
                    }
                }
            }
        }
    }

    fn start_search_worker(
        &self,
    ) -> (
        Sender<SearchRequest>,
        Receiver<SearchResponse>,
        smol::Task<()>,
    ) {
        let (request_tx, request_rx) = smol::channel::unbounded::<SearchRequest>();
        let (response_tx, response_rx) = smol::channel::unbounded::<SearchResponse>();
        let search_service = self.search_service.clone();

        let task = smol::spawn(async move {
            while let Ok(request) = request_rx.recv().await {
                // Use blocking::unblock to run the synchronous search in a separate thread
                // This prevents deadlock when SmolEngine uses block_on internally
                let result = blocking::unblock({
                    let search_service = search_service.clone();
                    let request = request.clone();
                    move || search_service.search(request)
                })
                .await;

                match result {
                    Ok(response) => {
                        let _ = response_tx.send(response).await;
                    }
                    Err(e) => {
                        eprintln!("Search error: {e}");
                        let _ = response_tx
                            .send(SearchResponse {
                                id: request.id,
                                results: Vec::new(),
                            })
                            .await;
                    }
                }
            }
        });

        (request_tx, response_rx, task)
    }

    fn copy_to_clipboard(&self, text: &str) -> Result<()> {
        #[cfg(target_os = "macos")]
        {
            use std::process::Command;
            let mut child = Command::new("pbcopy")
                .stdin(std::process::Stdio::piped())
                .spawn()
                .context("Failed to spawn pbcopy")?;

            if let Some(mut stdin) = child.stdin.take() {
                use std::io::Write;
                stdin
                    .write_all(text.as_bytes())
                    .context("Failed to write to pbcopy")?;
            }

            child.wait().context("Failed to wait for pbcopy")?;
            Ok(())
        }

        #[cfg(target_os = "linux")]
        {
            use std::process::Command;
            let mut child = Command::new("xclip")
                .arg("-selection")
                .arg("clipboard")
                .stdin(std::process::Stdio::piped())
                .spawn()
                .context("Failed to spawn xclip")?;

            if let Some(mut stdin) = child.stdin.take() {
                use std::io::Write;
                stdin
                    .write_all(text.as_bytes())
                    .context("Failed to write to xclip")?;
            }

            child.wait().context("Failed to wait for xclip")?;
            Ok(())
        }

        #[cfg(not(any(target_os = "macos", target_os = "linux")))]
        {
            Err(anyhow::anyhow!("Clipboard not supported on this platform"))
        }
    }

    #[cfg(test)]
    pub(crate) fn set_mode(&mut self, mode: Mode) {
        self.state.mode = mode;
    }
}
