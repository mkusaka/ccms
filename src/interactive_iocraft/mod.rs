use anyhow::Result;

pub mod application;
pub mod domain;
pub mod ui;
pub mod simple_mod;
pub mod minimal_mod;

use crate::SearchOptions;

pub async fn run_interactive_iocraft(_pattern: &str, _options: SearchOptions) -> Result<()> {
    // Use minimal implementation for testing
    minimal_mod::run_minimal_interactive().await
}
