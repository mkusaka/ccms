//! Constants for the interactive TUI module
//! 
//! This module centralizes magic numbers and configuration values
//! to improve maintainability and make the codebase more self-documenting.

// Timing constants
/// Message auto-clear delay in milliseconds
pub const MESSAGE_CLEAR_DELAY_MS: u64 = 3000;

/// Event polling interval in milliseconds
pub const EVENT_POLL_INTERVAL_MS: u64 = 50;

/// Search debounce delay in milliseconds
#[allow(dead_code)]
pub const SEARCH_DEBOUNCE_DELAY_MS: u64 = 300;

/// Double Ctrl+C timeout in seconds
pub const DOUBLE_CTRL_C_TIMEOUT_SECS: u64 = 1;

// UI Layout constants
/// Height of the search bar component
pub const SEARCH_BAR_HEIGHT: u16 = 3;

/// Height of the status bar
pub const STATUS_BAR_HEIGHT: u16 = 1;

/// Number of lines to keep visible when scrolling
#[allow(dead_code)]
pub const SCROLL_MARGIN: usize = 10;

/// Page size for PageUp/PageDown navigation
pub const PAGE_SIZE: usize = 10;

// Buffer sizes
/// Buffer size for file reading (32KB)
pub const FILE_READ_BUFFER_SIZE: usize = 32 * 1024;

/// Default maximum search results
#[allow(dead_code)]
pub const DEFAULT_MAX_RESULTS: usize = 100;

// Help dialog dimensions
/// Maximum width for help dialog
pub const HELP_DIALOG_MAX_WIDTH: u16 = 85;

/// Minimum margin around help dialog
pub const HELP_DIALOG_MARGIN: u16 = 4;

// List viewer constants
/// Timestamp column width
pub const TIMESTAMP_COLUMN_WIDTH: u16 = 19;

/// Role column width (with padding)
pub const ROLE_COLUMN_WIDTH: u16 = 11;

/// Separators and spacing width
pub const SEPARATOR_WIDTH: u16 = 5;

/// Minimum message content width
pub const MIN_MESSAGE_WIDTH: u16 = 20;

// Navigation history
/// Maximum navigation history entries
pub const MAX_NAVIGATION_HISTORY: usize = 50;