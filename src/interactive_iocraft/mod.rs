use anyhow::Result;
use iocraft::prelude::*;

pub mod application;
pub mod domain;
pub mod ui;

use crate::SearchOptions;

pub async fn run_interactive_iocraft(pattern: &str, options: SearchOptions) -> Result<()> {
    let pattern = pattern.to_string();
    element! {
        ui::App(pattern: pattern, options: options)
    }
    .render_loop()
    .await?;
    Ok(())
}
