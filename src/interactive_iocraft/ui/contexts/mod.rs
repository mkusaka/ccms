//! Context providers for global state

pub mod theme_context;
pub mod settings;

pub use theme_context::*;
pub use settings::{Settings, SettingsContext};