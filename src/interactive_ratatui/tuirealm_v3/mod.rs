mod messages;
mod state;
mod components;
mod services;
mod app;

use std::time::Duration;
use tuirealm::application::{Application, PollStrategy};
use tuirealm::terminal::TerminalBridge;
use tuirealm::event::{Event, Key, KeyEvent, KeyModifiers};
use tuirealm::{EventListenerCfg, NoUserEvent, Update};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use crossterm::event::{self as crossterm_event, Event as CrosstermEvent};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use self::messages::{AppMessage, AppMode, ComponentId};
use self::app::App;

/// Main entry point for tui-realm v3 interactive search
pub fn run_interactive_search(
    pattern: Option<String>,
    timestamp_gte: Option<String>,
    timestamp_lt: Option<String>,
    session_id: Option<String>,
) -> anyhow::Result<()> {
    // Enable raw mode
    enable_raw_mode()?;
    
    // Create terminal bridge
    let mut terminal = TerminalBridge::init_crossterm()?;
    
    // Create application
    let mut app: Application<ComponentId, AppMessage, NoUserEvent> = Application::init(
        EventListenerCfg::default(),
    );
    
    // Create our app logic
    let mut app_logic = App::new(
        pattern.clone(),
        timestamp_gte,
        timestamp_lt,
        session_id,
    );
    
    // Initialize components
    app_logic.init(&mut app)?;
    
    // Main event loop
    let mut should_quit = false;
    
    while !should_quit {
        // Poll for events
        match app.tick(PollStrategy::Once) {
            Ok(messages) => {
                // Process any messages from components
                for msg in messages {
                    if let Some(AppMessage::Quit) = app_logic.update(Some(msg)) {
                        should_quit = true;
                        break;
                    }
                }
            }
            Err(e) => {
                eprintln!("Error during tick: {}", e);
            }
        }
        
        // Check for global keyboard events
        if crossterm_event::poll(Duration::from_millis(0))? {
            if let CrosstermEvent::Key(key_event) = crossterm_event::read()? {
                // Convert crossterm event to tuirealm event
                let event = convert_crossterm_event(key_event);
                
                // Handle global shortcuts
                match &event {
                    Event::Keyboard(KeyEvent { code: Key::Char('q'), modifiers: KeyModifiers::CONTROL }) |
                    Event::Keyboard(KeyEvent { code: Key::Char('c'), modifiers: KeyModifiers::CONTROL }) => {
                        should_quit = true;
                    }
                    
                    Event::Keyboard(KeyEvent { code: Key::Char('?'), modifiers }) |
                    Event::Keyboard(KeyEvent { code: Key::Char('h'), modifiers }) 
                        if modifiers.is_empty() && app_logic.state.mode != AppMode::Help => {
                        app_logic.update(Some(AppMessage::ShowHelp));
                    }
                    
                    _ => {
                        // Forward event to active component
                        // Send event to active component based on mode
                        let active_component = match app_logic.state.mode {
                            AppMode::Search => ComponentId::SearchInput,
                            AppMode::ResultDetail => ComponentId::ResultDetail,
                            AppMode::SessionViewer => ComponentId::SessionViewer,
                            AppMode::Help => ComponentId::HelpDialog,
                        };
                        
                        // TODO: Handle events properly in tui-realm v3
                        // For now, just send keyboard events to the app logic
                        let msg = match (event, &app_logic.state.mode) {
                            (Event::Keyboard(KeyEvent { code: Key::Char('q'), .. }), _) => Some(AppMessage::Quit),
                            (Event::Keyboard(KeyEvent { code: Key::Char('?'), .. }), _) => Some(AppMessage::ShowHelp),
                            (Event::Keyboard(KeyEvent { code: Key::Esc, .. }), AppMode::Help) => Some(AppMessage::ExitHelp),
                            _ => None,
                        };
                        
                        if let Some(msg) = msg {
                            if let Some(AppMessage::Quit) = app_logic.update(Some(msg)) {
                                should_quit = true;
                            }
                        }
                    }
                }
            }
        }
        
        // Update state without new messages (for async operations)
        app_logic.update(None);
        
        // Update components with current state
        let _ = app_logic.update_components(&mut app);
        
        // Update active component based on mode
        let active_component = ComponentId::get_active(&app_logic.state.mode);
        let _ = app.active(&active_component);
        
        // Render
        terminal.raw_mut().draw(|f| {
            let _ = app.view(&active_component, f, f.size());
        })?;
    }
    
    // Cleanup
    terminal.leave_alternate_screen()?;
    terminal.disable_raw_mode()?;
    disable_raw_mode()?;
    
    Ok(())
}

/// Convert crossterm event to tuirealm event
fn convert_crossterm_event(event: crossterm::event::KeyEvent) -> Event<NoUserEvent> {
    use crossterm::event::{KeyCode, KeyModifiers as CrosstermModifiers};
    use tuirealm::event::{Key as TuiKey, KeyModifiers as TuiModifiers};
    
    let key = match event.code {
        KeyCode::Char(c) => TuiKey::Char(c),
        KeyCode::Enter => TuiKey::Enter,
        KeyCode::Esc => TuiKey::Esc,
        KeyCode::Backspace => TuiKey::Backspace,
        KeyCode::Tab => TuiKey::Tab,
        KeyCode::Delete => TuiKey::Delete,
        KeyCode::Insert => TuiKey::Insert,
        KeyCode::F(n) => TuiKey::Function(n),
        KeyCode::Home => TuiKey::Home,
        KeyCode::End => TuiKey::End,
        KeyCode::PageUp => TuiKey::PageUp,
        KeyCode::PageDown => TuiKey::PageDown,
        KeyCode::Left => TuiKey::Left,
        KeyCode::Right => TuiKey::Right,
        KeyCode::Up => TuiKey::Up,
        KeyCode::Down => TuiKey::Down,
        _ => TuiKey::Null,
    };
    
    let mut modifiers = TuiModifiers::empty();
    if event.modifiers.contains(CrosstermModifiers::SHIFT) {
        modifiers.insert(TuiModifiers::SHIFT);
    }
    if event.modifiers.contains(CrosstermModifiers::CONTROL) {
        modifiers.insert(TuiModifiers::CONTROL);
    }
    if event.modifiers.contains(CrosstermModifiers::ALT) {
        modifiers.insert(TuiModifiers::ALT);
    }
    
    Event::Keyboard(KeyEvent {
        code: key,
        modifiers,
    })
}

impl ComponentId {
    /// Get the active component for a given mode
    fn get_active(mode: &AppMode) -> Self {
        match mode {
            AppMode::Search => ComponentId::SearchInput,
            AppMode::ResultDetail => ComponentId::ResultDetail,
            AppMode::SessionViewer => ComponentId::SessionViewer,
            AppMode::Help => ComponentId::HelpDialog,
        }
    }
}