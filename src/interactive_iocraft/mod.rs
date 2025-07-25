//! Interactive search interface using iocraft
//!
//! This module provides a React-like TUI for searching Claude session messages.

pub mod domain;
pub mod application;
pub mod ui;
pub mod error;

#[cfg(test)]
mod integration_tests;

#[cfg(test)]
mod feature_tests;

#[cfg(test)]
mod workflow_tests;

#[cfg(test)]
mod performance_tests;

use anyhow::Result;
// Re-export types from the parent module for use in submodules
pub(crate) use crate::{Args, SessionMessage, SearchResult, QueryCondition, SearchEngine, SearchOptions, parse_query, default_claude_pattern};
use iocraft::prelude::*;
use std::sync::{Arc, Mutex};

use self::application::{SearchService, SessionService, CacheService, SettingsService};
use self::ui::components::App;

/// Entry point for the interactive iocraft interface
pub fn run(args: Args) -> Result<()> {
    eprintln!("Starting iocraft interactive mode...");
    
    // Initialize services
    let settings_service = Arc::new(SettingsService::new()?);
    let ui_settings = settings_service.get_ui_settings()?;
    
    let cache_service = Arc::new(Mutex::new(CacheService::new()));
    let search_service = Arc::new(SearchService::new(
        args.file_patterns.clone(),
        args.verbose,
    )?);
    let session_service = Arc::new(SessionService::new(cache_service.clone()));
    
    // Create and run the app
    smol::block_on(async {
        element! {
            App(
                search_service: Some(search_service),
                session_service: Some(session_service),
                cache_service: Some(cache_service),
                initial_query: args.query.clone(),
                settings: ui_settings,
                settings_service: Some(settings_service),
            )
        }
        .fullscreen()
        .await
    })?;
    
    Ok(())
}