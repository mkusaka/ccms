use anyhow::Result;
use r3bl_tui::{
    tui::terminal_window::*, 
    tui::*,
    HasFocus, FlexBoxId, App, Component, ComponentRegistryMap,
    GlobalData, InputEvent, KeyEvent, KeyCode, KeyModifiers,
    EventPropagation, CommonResult, RenderPipeline, Surface,
    render_ops, RenderOp, Position, ZOrder, BoxedSafeComponent,
    CommonInputEventHandler, TuiMode,
};
use r3bl_ansi_color::{Style as TuiStyle, Color as TuiColor};
use r3bl_rs_utils_core::*;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;

use crate::{SearchOptions, schemas::SessionMessage};

pub mod app;
pub mod components;
pub mod state;
pub mod search_service;

#[cfg(test)]
mod tests;

pub use self::app::SearchApp;
pub use self::state::{AppState, AppSignal};

pub async fn run_interactive_search(pattern: &str, options: SearchOptions) -> Result<()> {
    // Create the app
    let (tx, rx) = mpsc::channel::<AppSignal>(100);
    let app = SearchApp::new(options.clone(), pattern.to_string(), tx.clone());
    
    // Create the state with empty initial query
    let state = AppState::new("".to_string(), options);
    
    // Create terminal and run
    let result = TerminalWindow::main_event_loop(
        app,
        state,
        tx,
        rx,
    ).await;
    
    match result {
        Ok(_) => Ok(()),
        Err(e) => Err(anyhow::anyhow!("TUI error: {}", e)),
    }
}