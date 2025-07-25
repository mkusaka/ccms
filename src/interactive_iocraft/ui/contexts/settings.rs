//! Application settings context

use iocraft::prelude::*;

/// Application settings for the interactive interface
#[derive(Clone, Debug)]
pub struct Settings {
    /// Display settings
    pub display: DisplaySettings,
    
    /// Performance settings
    pub performance: PerformanceSettings,
    
    /// Key bindings
    pub key_bindings: KeyBindings,
}

/// Display-related settings
#[derive(Clone, Debug)]
pub struct DisplaySettings {
    /// Number of visible rows in result list
    pub result_list_rows: usize,
    
    /// Number of visible rows in session viewer
    pub session_viewer_rows: usize,
    
    /// Number of lines to show in detail view
    pub detail_view_lines: usize,
    
    /// Default truncation setting
    pub truncate_by_default: bool,
    
    /// Maximum characters to show when truncating
    pub truncate_length: usize,
    
    /// Number of items to overscan in virtual scrolling
    pub virtual_scroll_overscan: usize,
}

/// Performance-related settings
#[derive(Clone, Debug)]
pub struct PerformanceSettings {
    /// Debounce delay for search input (milliseconds)
    pub search_debounce_ms: u64,
    
    /// Timeout for terminal events (milliseconds)
    pub event_poll_timeout_ms: u64,
    
    /// Timeout for quit confirmation (milliseconds)
    pub quit_confirmation_timeout_ms: u64,
    
    /// Maximum cache size for file caching
    pub max_cache_entries: usize,
    
    /// Enable virtual scrolling for large lists
    pub enable_virtual_scroll: bool,
}

/// Key binding configuration
#[derive(Clone, Debug)]
pub struct KeyBindings {
    /// Navigation keys
    pub navigation: NavigationKeys,
    
    /// Action keys
    pub actions: ActionKeys,
    
    /// Copy keys
    pub copy: CopyKeys,
}

/// Navigation key bindings
#[derive(Clone, Debug)]
pub struct NavigationKeys {
    /// Move up
    pub up: Vec<KeyCode>,
    
    /// Move down
    pub down: Vec<KeyCode>,
    
    /// Page up
    pub page_up: Vec<KeyCode>,
    
    /// Page down
    pub page_down: Vec<KeyCode>,
    
    /// Go to start
    pub home: Vec<KeyCode>,
    
    /// Go to end
    pub end: Vec<KeyCode>,
}

/// Action key bindings
#[derive(Clone, Debug)]
pub struct ActionKeys {
    /// Select/Enter
    pub select: Vec<KeyCode>,
    
    /// Go back/Escape
    pub back: Vec<KeyCode>,
    
    /// Toggle role filter
    pub toggle_role_filter: Vec<KeyCode>,
    
    /// Toggle truncation
    pub toggle_truncate: Vec<KeyCode>,
    
    /// Start search
    pub start_search: Vec<KeyCode>,
    
    /// Show help
    pub show_help: Vec<KeyCode>,
    
    /// Quit application
    pub quit: Vec<KeyCode>,
}

/// Copy action key bindings
#[derive(Clone, Debug)]
pub struct CopyKeys {
    /// Copy content/message
    pub content: Vec<KeyCode>,
    
    /// Copy file path
    pub file_path: Vec<KeyCode>,
    
    /// Copy session ID
    pub session_id: Vec<KeyCode>,
    
    /// Copy project path
    pub project_path: Vec<KeyCode>,
    
    /// Copy raw JSON
    pub raw_json: Vec<KeyCode>,
    
    /// Copy URL
    pub url: Vec<KeyCode>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            display: DisplaySettings {
                result_list_rows: 20,
                session_viewer_rows: 20,
                detail_view_lines: 20,
                truncate_by_default: true,
                truncate_length: 100,
                virtual_scroll_overscan: 3,
            },
            performance: PerformanceSettings {
                search_debounce_ms: 300,
                event_poll_timeout_ms: 50,
                quit_confirmation_timeout_ms: 1000,
                max_cache_entries: 100,
                enable_virtual_scroll: true,
            },
            key_bindings: KeyBindings {
                navigation: NavigationKeys {
                    up: vec![KeyCode::Up, KeyCode::Char('k')],
                    down: vec![KeyCode::Down, KeyCode::Char('j')],
                    page_up: vec![KeyCode::PageUp],
                    page_down: vec![KeyCode::PageDown],
                    home: vec![KeyCode::Home],
                    end: vec![KeyCode::End],
                },
                actions: ActionKeys {
                    select: vec![KeyCode::Enter],
                    back: vec![KeyCode::Esc],
                    toggle_role_filter: vec![KeyCode::Tab],
                    toggle_truncate: vec![KeyCode::Char('t')],
                    start_search: vec![KeyCode::Char('/')],
                    show_help: vec![KeyCode::Char('?')],
                    quit: vec![KeyCode::Char('c'), KeyCode::Char('C')], // With Ctrl modifier
                },
                copy: CopyKeys {
                    content: vec![KeyCode::Char('c'), KeyCode::Char('C')],
                    file_path: vec![KeyCode::Char('f'), KeyCode::Char('F')],
                    session_id: vec![KeyCode::Char('i'), KeyCode::Char('I')],
                    project_path: vec![KeyCode::Char('p'), KeyCode::Char('P')],
                    raw_json: vec![KeyCode::Char('r'), KeyCode::Char('R')],
                    url: vec![KeyCode::Char('u'), KeyCode::Char('U')],
                },
            },
        }
    }
}

impl Settings {
    /// Create settings from a configuration file
    pub fn from_config(_config_path: &str) -> Result<Self, anyhow::Error> {
        // TODO: Implement config file parsing
        Ok(Self::default())
    }
    
    /// Check if a key matches a binding
    pub fn key_matches(&self, binding: &[KeyCode], key: KeyCode) -> bool {
        binding.contains(&key)
    }
    
    /// Get navigation key binding
    pub fn is_navigation_key(&self, key: KeyCode, action: NavigationAction) -> bool {
        match action {
            NavigationAction::Up => self.key_matches(&self.key_bindings.navigation.up, key),
            NavigationAction::Down => self.key_matches(&self.key_bindings.navigation.down, key),
            NavigationAction::PageUp => self.key_matches(&self.key_bindings.navigation.page_up, key),
            NavigationAction::PageDown => self.key_matches(&self.key_bindings.navigation.page_down, key),
            NavigationAction::Home => self.key_matches(&self.key_bindings.navigation.home, key),
            NavigationAction::End => self.key_matches(&self.key_bindings.navigation.end, key),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum NavigationAction {
    Up,
    Down,
    PageUp,
    PageDown,
    Home,
    End,
}

// Context type for dependency injection
pub type SettingsContext = Settings;