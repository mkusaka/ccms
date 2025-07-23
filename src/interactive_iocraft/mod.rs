use anyhow::Result;

pub mod application;
pub mod domain;
pub mod ui;
pub mod simple_mod;
pub mod minimal_mod;

use crate::SearchOptions;

pub async fn run_interactive_iocraft(pattern: &str, options: SearchOptions) -> Result<()> {
    // Fall back to ratatui implementation for now
    let mut interactive = crate::interactive_ratatui::InteractiveSearch::new(options);
    interactive.run(pattern)
}
