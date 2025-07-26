use anyhow::Result;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::Duration;
use std::io::{self, Write};

use crate::SearchOptions;

pub mod app;
pub mod state;
pub mod utils;

#[cfg(test)]
mod test_state;
#[cfg(test)]
mod test_app;
#[cfg(test)]
mod test_japanese;
#[cfg(test)]
mod test_layout;
#[cfg(test)]
mod test_ctrl_c;
#[cfg(test)]
mod test_app_ctrl_c;

pub use app::SearchApp;
pub use state::{AppState, SearchSignal};

pub async fn run_interactive_search(pattern: &str, options: SearchOptions) -> Result<()> {
    // Enable raw mode
    crossterm::terminal::enable_raw_mode()?;
    
    // Show cursor
    print!("\x1b[?25h");
    io::stdout().flush()?;
    
    // Create state and app
    let state = Arc::new(Mutex::new(AppState::new()));
    let mut app = SearchApp::new(pattern.to_string(), options, state.clone());
    
    // Initial render
    {
        let mut state_lock = state.lock().await;
        let output = app.render(&mut state_lock).await?;
        state_lock.needs_render = false;
        drop(state_lock);
        
        print!("{output}");
        io::stdout().flush()?;
    }
    
    // Main event loop
    loop {
        // Only render if needed
        {
            let mut state_lock = state.lock().await;
            if state_lock.needs_render {
                let output = app.render(&mut state_lock).await?;
                state_lock.needs_render = false;
                drop(state_lock);
                
                print!("{output}");
                io::stdout().flush()?;
            } else {
                drop(state_lock);
            }
        }
        
        // Handle input with timeout
        if crossterm::event::poll(Duration::from_millis(50))? {
            match crossterm::event::read()? {
                crossterm::event::Event::Key(key_event) => {
                    let mut state_lock = state.lock().await;
                    
                    let should_exit = match key_event.code {
                        crossterm::event::KeyCode::Char('c') 
                            if key_event.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) => {
                            app.handle_input('\x03', &mut state_lock).await?
                        }
                        crossterm::event::KeyCode::Char('u') 
                            if key_event.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) => {
                            app.handle_input('\x15', &mut state_lock).await?
                        }
                        crossterm::event::KeyCode::Char(c) => {
                            app.handle_input(c, &mut state_lock).await?
                        }
                        crossterm::event::KeyCode::Up => {
                            app.handle_input('k', &mut state_lock).await?
                        }
                        crossterm::event::KeyCode::Down => {
                            app.handle_input('j', &mut state_lock).await?
                        }
                        crossterm::event::KeyCode::Enter => {
                            app.handle_input('\n', &mut state_lock).await?
                        }
                        crossterm::event::KeyCode::Backspace => {
                            app.handle_input('\x08', &mut state_lock).await?
                        }
                        crossterm::event::KeyCode::Esc => {
                            app.handle_input('\x1b', &mut state_lock).await?
                        }
                        _ => false,
                    };
                    
                    drop(state_lock);
                    
                    if should_exit {
                        break;
                    }
                }
                // Ignore mouse events and other non-keyboard events
                crossterm::event::Event::Mouse(_) => {
                    // Do nothing - ignore mouse events
                }
                _ => {
                    // Ignore other events (resize, etc.)
                }
            }
        }
        
        // Process any pending search results
        let mut state_lock = state.lock().await;
        app.process_signals(&mut state_lock).await?;
        drop(state_lock);
    }
    
    // Cleanup
    crossterm::terminal::disable_raw_mode()?;
    print!("\x1b[2J\x1b[H"); // Clear screen
    print!("\x1b[?25h"); // Show cursor (ensure it's visible after exit)
    io::stdout().flush()?;
    
    Ok(())
}