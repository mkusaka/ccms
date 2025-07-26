use anyhow::Result;
use r3bl_tui::*;
use r3bl_ansi_color::*;
use std::fmt::Debug;
use tokio::sync::mpsc;

pub mod state;
pub mod app;

use state::{AppState, AppSignal};
use app::SearchApp;

pub async fn run_interactive_search(pattern: &str, options: crate::SearchOptions) -> Result<()> {
    // Create the terminal
    let mut terminal = TerminalAsync::try_new(
        TuiMode::FullScreen,
    ).await?;
    
    terminal.enter_raw_mode().await;
    
    // Create app, state, and channels
    let (tx, mut rx) = mpsc::channel::<AppSignal>(100);
    let app = SearchApp::new(pattern.to_string(), options, tx.clone());
    let state = AppState::new();
    
    // Create global data
    let global_data = GlobalData::new(state, tx.clone());
    
    // Create component registry
    let mut component_registry_map = ComponentRegistryMap::new(global_data);
    let mut has_focus = HasFocus::default();
    
    // Initialize the app
    app.app_init(&mut component_registry_map, &mut has_focus).await?;
    
    // Main event loop
    loop {
        // Handle input
        if let Ok(Some(input_event)) = terminal.get_input_event().await {
            let result = app.app_handle_input_event(
                input_event,
                &mut component_registry_map.state.write().await,
                &mut component_registry_map,
                &mut has_focus,
            ).await?;
            
            if matches!(result, EventPropagation::ConsumedReqExit) {
                break;
            }
        }
        
        // Handle signals
        while let Ok(signal) = rx.try_recv() {
            let result = app.app_handle_signal(
                signal,
                &mut component_registry_map.state.write().await,
                &mut component_registry_map,
                &mut has_focus,
            ).await?;
            
            if matches!(result, EventPropagation::ConsumedReqExit) {
                break;
            }
        }
        
        // Render
        let pipeline = app.app_render(
            &mut component_registry_map.state.write().await,
            &mut component_registry_map,
            &mut has_focus,
        ).await?;
        
        terminal.render(pipeline).await?;
    }
    
    terminal.leave_raw_mode().await;
    terminal.flush().await;
    
    Ok(())
}