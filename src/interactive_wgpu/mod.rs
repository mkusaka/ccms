use anyhow::{Context, Result};
use std::sync::{Arc, Mutex};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::Duration;
use std::num::NonZeroU32;

use winit::{
    application::ApplicationHandler,
    event::{WindowEvent, KeyEvent, ElementState},
    event_loop::{EventLoop, ControlFlow},
    window::{Window, WindowAttributes},
    keyboard::{KeyCode, PhysicalKey},
};

use ratatui::Terminal;
use ratatui_wgpu::{Builder, Dimensions, Font, WgpuBackend};

use crate::SearchOptions;
use crate::interactive_ratatui::{
    domain::models::{Mode, SearchRequest, SearchResponse},
    application::{
        search_service::SearchService,
        session_service::SessionService,
        cache_service::CacheService,
    },
    ui::{
        app_state::AppState,
        commands::Command,
        events::Message,
        renderer::Renderer,
        components::Component,
    },
};

pub struct InteractiveSearchWgpu {
    window: Option<Arc<Window>>,
    terminal: Option<Terminal<WgpuBackend<'static, 'static>>>,
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
    should_redraw: bool,
    modifiers: winit::keyboard::ModifiersState,
}

impl InteractiveSearchWgpu {
    pub fn new(options: SearchOptions) -> Self {
        let max_results = options.max_results.unwrap_or(100);
        let cache = Arc::new(Mutex::new(CacheService::new()));

        let search_service = Arc::new(SearchService::new(options.clone()));
        let session_service = Arc::new(SessionService::new(cache));

        Self {
            window: None,
            terminal: None,
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
            should_redraw: true,
            modifiers: winit::keyboard::ModifiersState::empty(),
        }
    }

    pub fn run(&mut self, pattern: &str) -> Result<()> {
        self.pattern = pattern.to_string();
        
        // Initialize logger
        env_logger::init();
        
        // Start search worker thread
        let (tx, rx) = self.start_search_worker();
        self.search_sender = Some(tx);
        self.search_receiver = Some(rx);
        
        // Initial search
        self.execute_command(Command::ExecuteSearch);
        
        // Run event loop
        let event_loop = EventLoop::builder().build()?;
        event_loop.set_control_flow(ControlFlow::Poll);
        event_loop.run_app(self)?;
        
        Ok(())
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
                    let copy_message = if text.starts_with("File: ") && text.contains("\nUUID: ") {
                        "✓ Copied full result details"
                    } else if text.starts_with('/')
                        && (text.ends_with(".jsonl") || text.contains('/'))
                    {
                        "✓ Copied file path"
                    } else if text.len() == 36 && text.chars().filter(|&c| c == '-').count() == 4 {
                        "✓ Copied session ID"
                    } else if text.len() < 100 {
                        &format!("✓ Copied: {}", text.chars().take(50).collect::<String>())
                    } else {
                        "✓ Copied message text"
                    };
                    self.state.ui.message = Some(copy_message.to_string());
                }
            }
            Command::ShowMessage(msg) => {
                self.state.ui.message = Some(msg);
            }
            Command::ClearMessage => {
                self.state.ui.message = None;
            }
        }
        self.should_redraw = true;
    }

    fn execute_search(&mut self) {
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

    fn handle_key_event(&mut self, key: KeyEvent, modifiers: winit::keyboard::ModifiersState) -> Result<bool> {
        use winit::keyboard::ModifiersState;
        
        if key.state != ElementState::Pressed {
            return Ok(false);
        }

        // Get modifiers from the event
        let ctrl_pressed = modifiers.contains(ModifiersState::CONTROL);
        
        // Global Ctrl+C handling for exit
        if let PhysicalKey::Code(KeyCode::KeyC) = key.physical_key {
            if ctrl_pressed {
                if let Some(last_press) = self.last_ctrl_c_press {
                    if last_press.elapsed() < Duration::from_secs(1) {
                        return Ok(true);
                    }
                }
                self.last_ctrl_c_press = Some(std::time::Instant::now());
                self.state.ui.message = Some("Press Ctrl+C again to exit".to_string());
                self.should_redraw = true;
                return Ok(false);
            }
        }

        // Convert winit KeyEvent to crossterm KeyEvent for compatibility
        let crossterm_key = self.convert_to_crossterm_key(key, modifiers);
        
        // Global keys
        match crossterm_key.code {
            crossterm::event::KeyCode::Esc => {
                if self.state.mode == Mode::Search {
                    return Ok(true);
                }
            }
            crossterm::event::KeyCode::Char('?') if self.state.mode != Mode::Help => {
                self.handle_message(Message::ShowHelp);
                return Ok(false);
            }
            crossterm::event::KeyCode::Char('t') if crossterm_key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) => {
                self.handle_message(Message::ToggleTruncation);
                return Ok(false);
            }
            _ => {}
        }

        // Mode-specific input handling
        let message = match self.state.mode {
            Mode::Search => self.handle_search_mode_input(crossterm_key),
            Mode::ResultDetail => self.renderer.get_result_detail_mut().handle_key(crossterm_key),
            Mode::SessionViewer => self.renderer.get_session_viewer_mut().handle_key(crossterm_key),
            Mode::Help => self.renderer.get_help_dialog_mut().handle_key(crossterm_key),
        };

        if let Some(msg) = message {
            self.handle_message(msg);
        }

        Ok(false)
    }

    fn handle_search_mode_input(&mut self, key: crossterm::event::KeyEvent) -> Option<Message> {
        match key.code {
            crossterm::event::KeyCode::Tab => Some(Message::ToggleRoleFilter),
            crossterm::event::KeyCode::Up
            | crossterm::event::KeyCode::Down
            | crossterm::event::KeyCode::PageUp
            | crossterm::event::KeyCode::PageDown
            | crossterm::event::KeyCode::Home
            | crossterm::event::KeyCode::End
            | crossterm::event::KeyCode::Enter => self.renderer.get_result_list_mut().handle_key(key),
            _ => self.renderer.get_search_bar_mut().handle_key(key),
        }
    }

    fn convert_to_crossterm_key(&self, key: KeyEvent, modifiers: winit::keyboard::ModifiersState) -> crossterm::event::KeyEvent {
        use crossterm::event::{KeyCode as CKeyCode, KeyModifiers};
        use winit::keyboard::{KeyCode as WKeyCode, ModifiersState};
        
        let code = match key.physical_key {
            PhysicalKey::Code(code) => match code {
                WKeyCode::Enter => CKeyCode::Enter,
                WKeyCode::Escape => CKeyCode::Esc,
                WKeyCode::Backspace => CKeyCode::Backspace,
                WKeyCode::Tab => CKeyCode::Tab,
                WKeyCode::Delete => CKeyCode::Delete,
                WKeyCode::Home => CKeyCode::Home,
                WKeyCode::End => CKeyCode::End,
                WKeyCode::PageUp => CKeyCode::PageUp,
                WKeyCode::PageDown => CKeyCode::PageDown,
                WKeyCode::ArrowUp => CKeyCode::Up,
                WKeyCode::ArrowDown => CKeyCode::Down,
                WKeyCode::ArrowLeft => CKeyCode::Left,
                WKeyCode::ArrowRight => CKeyCode::Right,
                _ => {
                    // Try to convert character keys
                    if let Some(text) = &key.text {
                        if let Some(ch) = text.chars().next() {
                            CKeyCode::Char(ch)
                        } else {
                            CKeyCode::Null
                        }
                    } else {
                        CKeyCode::Null
                    }
                }
            },
            _ => CKeyCode::Null,
        };
        
        let mut key_modifiers = KeyModifiers::empty();
        
        if modifiers.contains(ModifiersState::SHIFT) {
            key_modifiers |= KeyModifiers::SHIFT;
        }
        if modifiers.contains(ModifiersState::CONTROL) {
            key_modifiers |= KeyModifiers::CONTROL;
        }
        if modifiers.contains(ModifiersState::ALT) {
            key_modifiers |= KeyModifiers::ALT;
        }
        
        crossterm::event::KeyEvent::new(code, key_modifiers)
    }

    fn check_updates(&mut self) {
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
    }
}

impl ApplicationHandler for InteractiveSearchWgpu {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        self.window = Some(Arc::new(
            event_loop
                .create_window(WindowAttributes::default()
                    .with_title("CCMS - Interactive Search"))
                .unwrap(),
        ));

        let size = self.window.as_ref().unwrap().inner_size();
        
        self.terminal = Some(
            Terminal::new(
                futures_lite::future::block_on(
                    Builder::from_font(
                        Font::new(include_bytes!(concat!(
                            env!("CARGO_MANIFEST_DIR"),
                            "/src/backend/fonts/CascadiaMono-Regular.ttf"
                        )))
                        .unwrap(),
                    )
                    .with_width_and_height(Dimensions {
                        width: NonZeroU32::new(size.width).unwrap(),
                        height: NonZeroU32::new(size.height).unwrap(),
                    })
                    .build_with_target(self.window.as_ref().unwrap().clone()),
                )
                .unwrap(),
            )
            .unwrap(),
        );

        self.window.as_ref().unwrap().request_redraw();
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::Resized(size) => {
                if let Some(terminal) = self.terminal.as_mut() {
                    terminal.backend_mut().resize(size.width, size.height);
                    self.should_redraw = true;
                }
            }
            WindowEvent::ModifiersChanged(new_modifiers) => {
                self.modifiers = new_modifiers.state();
            }
            WindowEvent::KeyboardInput { event: key_event, .. } => {
                if let Ok(should_quit) = self.handle_key_event(key_event, self.modifiers) {
                    if should_quit {
                        event_loop.exit();
                    }
                }
            }
            WindowEvent::RedrawRequested => {
                self.check_updates();
                
                if let Some(terminal) = self.terminal.as_mut() {
                    terminal
                        .draw(|f| {
                            self.renderer.render(f, &self.state);
                        })
                        .unwrap();
                }
                
                // Request next frame
                self.window.as_ref().unwrap().request_redraw();
            }
            _ => {}
        }
    }
}