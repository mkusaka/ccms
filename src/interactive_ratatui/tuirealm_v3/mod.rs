mod messages;
mod state;
mod components;
mod services;
mod app;
mod models;
mod error;
mod type_safe_wrapper;

#[cfg(test)]
mod integration_test;

#[cfg(test)]
mod payload_example;

use std::time::Duration;
use tuirealm::application::{Application, PollStrategy};
use tuirealm::terminal::TerminalBridge;
use tuirealm::{EventListenerCfg, NoUserEvent, Update};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};

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
    
    // Create application with proper event listener config
    let event_cfg = EventListenerCfg::default()
        .crossterm_input_listener(Duration::from_millis(50), 10);
    
    let mut app: Application<ComponentId, AppMessage, NoUserEvent> = Application::init(
        event_cfg,
    );
    
    // Create our app logic
    let mut app_logic = App::new(
        pattern.clone(),
        timestamp_gte,
        timestamp_lt,
        session_id,
    );
    
    // Initialize components
    app_logic.init(&mut app).map_err(|e| anyhow::anyhow!("Failed to initialize app: {}", e))?;
    
    // Main event loop
    let mut should_quit = false;
    
    while !should_quit {
        // Process events through tui-realm
        match app.tick(PollStrategy::Once) {
            Ok(messages) => {
                for msg in messages {
                    match &msg {
                        AppMessage::Quit => {
                            should_quit = true;
                            break;
                        }
                        _ => {
                            if let Some(AppMessage::Quit) = app_logic.update(Some(msg)) {
                                should_quit = true;
                                break;
                            }
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("Error during tick: {e}");
            }
        }
        
        // Check for debounced search only when not already searching
        if !app_logic.state.is_searching {
            if let Some(msg) = app_logic.check_debounced_search() {
                if let Some(AppMessage::Quit) = app_logic.update(Some(msg)) {
                    should_quit = true;
                }
            }
        }
        
        // Check for async operations (search results)
        if app_logic.state.is_searching {
            // Only update when actively searching
            if let Some(msg) = app_logic.update(None) {
                if msg == AppMessage::Quit {
                    should_quit = true;
                }
            }
        }
        
        // Update active component based on mode
        let active_component = ComponentId::get_active(&app_logic.state.mode);
        app.active(&active_component).ok();
        
        // Render
        terminal.raw_mut().draw(|f| {
            // Use layout to render multiple components based on mode
            app_logic.render_layout(&mut app, f);
        })?;
    }
    
    // Cleanup
    terminal.leave_alternate_screen()?;
    terminal.disable_raw_mode()?;
    disable_raw_mode()?;
    
    Ok(())
}

impl ComponentId {
    /// Get the active component for a given mode
    fn get_active(mode: &AppMode) -> Self {
        match mode {
            AppMode::Search => ComponentId::SearchInput,
            AppMode::ResultDetail => ComponentId::ResultDetail,
            AppMode::SessionViewer => ComponentId::SessionViewer,
            AppMode::Help => ComponentId::HelpDialog,
            AppMode::Error => ComponentId::ErrorDialog,
        }
    }
}

#[cfg(test)]
#[path = "edge_case_test.rs"]
mod edge_case_test;

#[cfg(test)]
#[path = "error_handling_test.rs"]
mod error_handling_test;

#[cfg(test)]
#[path = "e2e_test.rs"]
mod e2e_test;

#[cfg(test)]
#[path = "feature_test.rs"]
mod feature_test;

#[cfg(test)]
#[path = "performance_benchmark.rs"]
mod performance_benchmark;

#[cfg(test)]
#[path = "error_handling_improvements_test.rs"]
mod error_handling_improvements_test;