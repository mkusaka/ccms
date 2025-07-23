use anyhow::Result;

pub mod application;
pub mod domain;
pub mod ui;
pub mod simple_mod;

use crate::SearchOptions;

pub async fn run_interactive_iocraft(_pattern: &str, _options: SearchOptions) -> Result<()> {
    // Use simple implementation for now
    simple_mod::run_simple_interactive().await
}
