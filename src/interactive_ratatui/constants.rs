//! Constants for the interactive TUI module
//!
//! This module centralizes magic numbers and configuration values
//! to improve maintainability and make the codebase more self-documenting.

// Timing constants
/// Message auto-clear delay in milliseconds
pub const MESSAGE_CLEAR_DELAY_MS: u64 = 3000;

/// Event polling interval in milliseconds
pub const EVENT_POLL_INTERVAL_MS: u64 = 50;

/// Double Ctrl+C timeout in seconds
pub const DOUBLE_CTRL_C_TIMEOUT_SECS: u64 = 1;

// UI Layout constants
/// Height of the search bar component
pub const SEARCH_BAR_HEIGHT: u16 = 3;

/// Page size for PageUp/PageDown navigation
pub const PAGE_SIZE: usize = 10;

// Buffer sizes
// /// Buffer size for file reading (32KB)
// pub const FILE_READ_BUFFER_SIZE: usize = 32 * 1024; // No longer used - cache service removed

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

// Message detail layout constants
/// Height of the details header section (role, time, file, project, UUID, session)
pub const MESSAGE_DETAIL_HEADER_HEIGHT: u16 = 8;

/// Height of the status bar in message detail view
pub const MESSAGE_DETAIL_STATUS_HEIGHT: u16 = 1;

// General UI layout constants
/// Height of the exit prompt displayed at the bottom
pub const EXIT_PROMPT_HEIGHT: u16 = 1;

// Result list layout constants
/// Height of the title area in result list
pub const RESULT_LIST_TITLE_HEIGHT: u16 = 2;

// List viewer default values
/// Default viewport height for list viewer
pub const DEFAULT_VIEWPORT_HEIGHT: u16 = 10;

/// Estimated viewport size for truncated mode
pub const TRUNCATED_VIEWPORT_ESTIMATE: usize = 20;

/// Estimated viewport size for full text mode
pub const FULL_TEXT_VIEWPORT_ESTIMATE: usize = 10;

// View layout border and sizing constants
/// Width adjustment for borders (left and right)
pub const BORDER_WIDTH_ADJUSTMENT: u16 = 2;

/// Height adjustment for bottom border
pub const BORDER_HEIGHT_ADJUSTMENT: u16 = 1;

/// Minimum height for status bar
pub const STATUS_BAR_MIN_HEIGHT: u16 = 1;

/// Maximum height for status bar
pub const STATUS_BAR_MAX_HEIGHT: u16 = 3;
