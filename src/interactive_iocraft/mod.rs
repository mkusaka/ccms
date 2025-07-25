//! Interactive search interface using iocraft
//!
//! This module provides a React-like TUI for searching Claude session messages.

pub mod domain;
pub mod application;
pub mod ui;

#[cfg(test)]
mod integration_tests;

#[cfg(test)]
mod feature_tests;

#[cfg(test)]
mod workflow_tests;

use anyhow::Result;
// Re-export types from the parent module for use in submodules
pub(crate) use ccms::{Args, SessionMessage, SearchResult, QueryCondition, SearchEngine, SearchOptions, parse_query, default_claude_pattern};
use iocraft::prelude::*;
use std::sync::{Arc, Mutex};

use self::application::{SearchService, SessionService, CacheService};
use self::ui::components::App;

/// Entry point for the interactive iocraft interface
pub fn run(args: Args) -> Result<()> {
    // Initialize services
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
            )
        }
        .fullscreen()
        .await
    })?;
    
    Ok(())
}