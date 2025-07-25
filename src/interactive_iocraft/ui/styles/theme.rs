//! Theme utilities

use iocraft::prelude::*;

/// Helper function to create padding/margin edges
/// Note: In iocraft, padding and margin are set individually, not as Edges
pub fn edge(top: u16, right: u16, bottom: u16, left: u16) -> (u16, u16, u16, u16) {
    (top, right, bottom, left)
}

/// Helper to create uniform edges
pub fn edge_all(value: u16) -> (u16, u16, u16, u16) {
    (value, value, value, value)
}