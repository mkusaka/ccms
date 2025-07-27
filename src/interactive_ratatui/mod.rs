use anyhow::{Context, Result};
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, poll},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};
use std::io::{self, Stdout};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use crate::SearchOptions;

mod application;
mod domain;
pub mod ui;

#[cfg(test)]
mod help_navigation_test;
#[cfg(test)]
mod integration_tests;
#[cfg(test)]
mod session_view_integration_test;
#[cfg(test)]
mod tests;

use self::application::{
    cache_service::CacheService, search_service::SearchService, session_service::SessionService,
};
use self::domain::models::{Mode, SearchRequest, SearchResponse};
use self::ui::{
    app_state::AppState, commands::Command, components::Component, events::Message,
    renderer::Renderer,
};

pub struct InteractiveSearch {
    state: AppState,
    renderer: Renderer,
    search_service: Arc<SearchService>,
    session_service: Arc<SessionService>,
    search_sender: Option<Sender<SearchRequest>>,
    search_receiver: Option<Receiver<SearchResponse>>,
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
        let max_results = options.max_results.unwrap_or(100);
        let cache = Arc::new(Mutex::new(CacheService::new()));

        let search_service = Arc::new(SearchService::new(options.clone()));
        let session_service = Arc::new(SessionService::new(cache));

        Self {
            state: AppState::new(options, max_results),
            renderer: Renderer::new(),
            search_service,
            session_service,
            search_sender: None,
            search_receiver: None,
            current_search_id: 0,
            last_search_timer: None,
            scheduled_search_delay: None,
            pattern: String::new(),
            last_ctrl_c_press: None,
            message_timer: None,
            message_clear_delay: 3000, // 3秒後に消える
        }
    }

    pub fn run(&mut self, pattern: &str) -> Result<()> {
        self.pattern = pattern.to_string();
        let mut terminal = self.setup_terminal()?;

        // Start search worker thread
        let (tx, rx) = self.start_search_worker();
        self.search_sender = Some(tx);
        self.search_receiver = Some(rx);

        // Initial search (even with empty pattern to show all results)
        // Note: pattern is stored internally but not shown in search bar
        self.execute_command(Command::ExecuteSearch);

        let result = self.run_app(&mut terminal, pattern);

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

    fn run_app(
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
                        self.execute_command(Command::ExecuteSearch);
                    }
                }
            }

            // Check for scheduled message clear
            if let Some(timer) = self.message_timer {
                if timer.elapsed() >= Duration::from_millis(self.message_clear_delay) {
                    self.message_timer = None;
                    self.execute_command(Command::ClearMessage);
                }
            }

            if poll(Duration::from_millis(50))? {
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
                if last_press.elapsed() < Duration::from_secs(1) {
                    // Exit application
                    return Ok(true);
                }
            }
            // First press or timeout expired
            self.last_ctrl_c_press = Some(std::time::Instant::now());
            self.state.ui.message = Some("Press Ctrl+C again to exit".to_string());
            return Ok(false);
        }

        // Global keys
        match key.code {
            KeyCode::Char('?') if self.state.mode != Mode::Help => {
                self.handle_message(Message::ShowHelp);
                return Ok(false);
            }
            KeyCode::Char('t') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.handle_message(Message::ToggleTruncation);
                return Ok(false);
            }
            _ => {}
        }

        // Mode-specific input handling
        let message = match self.state.mode {
            Mode::Search => self.handle_search_mode_input(key),
            Mode::ResultDetail => self.renderer.get_result_detail_mut().handle_key(key),
            Mode::SessionViewer => self.renderer.get_session_viewer_mut().handle_key(key),
            Mode::Help => self.renderer.get_help_dialog_mut().handle_key(key),
        };

        if let Some(msg) = message {
            self.handle_message(msg);
        }

        Ok(false)
    }

    fn handle_search_mode_input(&mut self, key: KeyEvent) -> Option<Message> {
        match key.code {
            KeyCode::Tab => Some(Message::ToggleRoleFilter),
            KeyCode::Up
            | KeyCode::Down
            | KeyCode::PageUp
            | KeyCode::PageDown
            | KeyCode::Home
            | KeyCode::End
            | KeyCode::Enter => self.renderer.get_result_list_mut().handle_key(key),
            _ => self.renderer.get_search_bar_mut().handle_key(key),
        }
    }

    fn handle_message(&mut self, message: Message) {
        let command = self.state.update(message);
        self.execute_command(command);
    }

    fn execute_command(&mut self, command: Command) {
        match command {
            Command::None => {}
            Command::ExecuteSearch => {
                self.execute_search();
            }
            Command::ScheduleSearch(delay) => {
                self.last_search_timer = Some(std::time::Instant::now());
                self.scheduled_search_delay = Some(delay);
            }
            Command::LoadSession(file_path) => {
                self.load_session_messages(&file_path);
            }
            Command::CopyToClipboard(text) => {
                if let Err(e) = self.copy_to_clipboard(&text) {
                    self.state.ui.message = Some(format!("Failed to copy: {e}"));
                } else {
                    // Determine what was copied for better feedback
                    let copy_message = if text.starts_with("File: ") && text.contains("\nUUID: ") {
                        "✓ Copied full result details"
                    } else if text.starts_with('/')
                        && (text.ends_with(".jsonl") || text.contains('/'))
                    {
                        "✓ Copied file path"
                    } else if text.len() == 36 && text.chars().filter(|&c| c == '-').count() == 4 {
                        // UUID format check
                        "✓ Copied session ID"
                    } else if text.len() < 100 {
                        &format!("✓ Copied: {}", text.chars().take(50).collect::<String>())
                    } else {
                        "✓ Copied message text"
                    };
                    self.state.ui.message = Some(copy_message.to_string());

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

    fn execute_search(&mut self) {
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
            };
            let _ = sender.send(request);
        }
    }

    fn load_session_messages(&mut self, file_path: &str) {
        match self.session_service.load_session(file_path) {
            Ok(_messages) => {
                let raw_lines = self
                    .session_service
                    .get_raw_lines(file_path)
                    .unwrap_or_default();
                self.state.session.messages = raw_lines;
                self.state.session.filtered_indices =
                    (0..self.state.session.messages.len()).collect();
            }
            Err(e) => {
                self.state.ui.message = Some(format!("Failed to load session: {e}"));
            }
        }
    }

    fn start_search_worker(&self) -> (Sender<SearchRequest>, Receiver<SearchResponse>) {
        let (request_tx, request_rx) = mpsc::channel::<SearchRequest>();
        let (response_tx, response_rx) = mpsc::channel::<SearchResponse>();
        let search_service = self.search_service.clone();

        thread::spawn(move || {
            while let Ok(request) = request_rx.recv() {
                match search_service.search(request.clone()) {
                    Ok(response) => {
                        let _ = response_tx.send(response);
                    }
                    Err(e) => {
                        eprintln!("Search error: {e}");
                        let _ = response_tx.send(SearchResponse {
                            id: request.id,
                            results: Vec::new(),
                        });
                    }
                }
            }
        });

        (request_tx, response_rx)
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

    // For compatibility with existing tests
    #[allow(dead_code)]
    pub(crate) fn current_mode(&self) -> Mode {
        self.state.mode
    }

    #[allow(dead_code)]
    pub(crate) fn push_screen(&mut self, mode: Mode) {
        self.state.mode = mode;
    }

    #[cfg(test)]
    pub(crate) fn set_mode(&mut self, mode: Mode) {
        self.state.mode = mode;
    }
}
