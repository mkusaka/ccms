//! Settings service for managing persistent application settings
//!
//! Bridges the domain settings with UI context settings

use crate::interactive_iocraft::domain::settings::{SettingsManager, UserSettings};
use crate::interactive_iocraft::ui::contexts::settings::{
    Settings, DisplaySettings, PerformanceSettings, KeyBindings,
    NavigationKeys, ActionKeys, CopyKeys,
};
use anyhow::Result;
use iocraft::prelude::*;
use std::sync::{Arc, Mutex};

/// Service for managing application settings
pub struct SettingsService {
    manager: Arc<Mutex<SettingsManager>>,
    current_settings: Arc<Mutex<UserSettings>>,
}

impl SettingsService {
    /// Create a new settings service
    pub fn new() -> Result<Self> {
        let manager = SettingsManager::new()?;
        let settings = manager.load()?;
        
        Ok(Self {
            manager: Arc::new(Mutex::new(manager)),
            current_settings: Arc::new(Mutex::new(settings)),
        })
    }
    
    /// Create a settings service with a custom path (for testing)
    pub fn with_path(path: std::path::PathBuf) -> Result<Self> {
        let manager = SettingsManager::with_path(path);
        let settings = manager.load()?;
        
        Ok(Self {
            manager: Arc::new(Mutex::new(manager)),
            current_settings: Arc::new(Mutex::new(settings)),
        })
    }
    
    /// Get current settings as UI context settings
    pub fn get_ui_settings(&self) -> Result<Settings> {
        let user_settings = self.current_settings.lock()
            .map_err(|e| anyhow::anyhow!("Failed to lock settings: {}", e))?
            .clone();
        
        Ok(Self::convert_to_ui_settings(&user_settings))
    }
    
    /// Update settings and persist to disk
    pub fn update<F>(&self, updater: F) -> Result<()>
    where
        F: FnOnce(&mut UserSettings),
    {
        let manager = self.manager.lock()
            .map_err(|e| anyhow::anyhow!("Failed to lock manager: {}", e))?;
        
        let updated = manager.update(updater)?;
        
        let mut current = self.current_settings.lock()
            .map_err(|e| anyhow::anyhow!("Failed to lock settings: {}", e))?;
        *current = updated;
        
        Ok(())
    }
    
    /// Reset settings to defaults
    pub fn reset(&self) -> Result<()> {
        let manager = self.manager.lock()
            .map_err(|e| anyhow::anyhow!("Failed to lock manager: {}", e))?;
        
        let default_settings = manager.reset()?;
        
        let mut current = self.current_settings.lock()
            .map_err(|e| anyhow::anyhow!("Failed to lock settings: {}", e))?;
        *current = default_settings;
        
        Ok(())
    }
    
    /// Convert domain settings to UI context settings
    fn convert_to_ui_settings(user_settings: &UserSettings) -> Settings {
        Settings {
            display: DisplaySettings {
                result_list_rows: user_settings.visible_items,
                session_viewer_rows: user_settings.visible_items,
                detail_view_lines: user_settings.visible_items,
                truncate_by_default: user_settings.truncate_by_default,
                truncate_length: 100, // Default, could be added to UserSettings
                virtual_scroll_overscan: 3, // Default
            },
            performance: PerformanceSettings {
                search_debounce_ms: user_settings.search_debounce_ms,
                event_poll_timeout_ms: 50, // Default
                quit_confirmation_timeout_ms: 1000, // Default
                max_cache_entries: user_settings.performance.cache_size_mb * 10, // Approximate
                enable_virtual_scroll: user_settings.performance.enable_virtual_scrolling,
            },
            key_bindings: Self::convert_keybindings(&user_settings.keybindings),
        }
    }
    
    /// Convert domain keybindings to UI keybindings
    fn convert_keybindings(kb: &crate::interactive_iocraft::domain::settings::KeyBindings) -> KeyBindings {
        KeyBindings {
            navigation: NavigationKeys {
                up: Self::parse_keys(&kb.navigate_up),
                down: Self::parse_keys(&kb.navigate_down),
                page_up: vec![KeyCode::PageUp],
                page_down: vec![KeyCode::PageDown],
                home: vec![KeyCode::Home],
                end: vec![KeyCode::End],
            },
            actions: ActionKeys {
                select: vec![KeyCode::Enter],
                back: Self::parse_keys(&kb.back),
                toggle_role_filter: vec![KeyCode::Tab],
                toggle_truncate: vec![KeyCode::Char('t')],
                start_search: vec![KeyCode::Char('/')],
                show_help: vec![KeyCode::Char('?')],
                quit: Self::parse_keys(&kb.quit),
            },
            copy: CopyKeys {
                content: vec![KeyCode::Char('c')],
                file_path: vec![KeyCode::Char('f')],
                session_id: vec![KeyCode::Char('i')],
                project_path: vec![KeyCode::Char('p')],
                raw_json: vec![KeyCode::Char('r')],
                url: vec![KeyCode::Char('u')],
            },
        }
    }
    
    /// Parse key strings to KeyCode
    fn parse_keys(keys: &[String]) -> Vec<KeyCode> {
        keys.iter()
            .filter_map(|k| Self::parse_key(k))
            .collect()
    }
    
    /// Parse a single key string to KeyCode
    fn parse_key(key: &str) -> Option<KeyCode> {
        match key.to_lowercase().as_str() {
            "up" => Some(KeyCode::Up),
            "down" => Some(KeyCode::Down),
            "left" => Some(KeyCode::Left),
            "right" => Some(KeyCode::Right),
            "enter" => Some(KeyCode::Enter),
            "esc" | "escape" => Some(KeyCode::Esc),
            "backspace" => Some(KeyCode::Backspace),
            "tab" => Some(KeyCode::Tab),
            "home" => Some(KeyCode::Home),
            "end" => Some(KeyCode::End),
            "pageup" | "page-up" | "pgup" => Some(KeyCode::PageUp),
            "pagedown" | "page-down" | "pgdn" => Some(KeyCode::PageDown),
            "ctrl-c" => Some(KeyCode::Char('c')), // Will need modifier check
            s if s.len() == 1 => s.chars().next().map(KeyCode::Char),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    
    #[test]
    fn test_settings_service_new() {
        let temp_file = NamedTempFile::new().unwrap();
        let service = SettingsService::with_path(temp_file.path().to_path_buf()).unwrap();
        
        let ui_settings = service.get_ui_settings().unwrap();
        assert_eq!(ui_settings.display.result_list_rows, 20);
        assert_eq!(ui_settings.performance.search_debounce_ms, 300);
    }
    
    #[test]
    fn test_settings_update() {
        let temp_file = NamedTempFile::new().unwrap();
        let service = SettingsService::with_path(temp_file.path().to_path_buf()).unwrap();
        
        // Update settings
        service.update(|s| {
            s.visible_items = 50;
            s.search_debounce_ms = 500;
        }).unwrap();
        
        // Verify update
        let ui_settings = service.get_ui_settings().unwrap();
        assert_eq!(ui_settings.display.result_list_rows, 50);
        assert_eq!(ui_settings.performance.search_debounce_ms, 500);
    }
    
    #[test]
    fn test_key_parsing() {
        assert_eq!(SettingsService::parse_key("up"), Some(KeyCode::Up));
        assert_eq!(SettingsService::parse_key("k"), Some(KeyCode::Char('k')));
        assert_eq!(SettingsService::parse_key("enter"), Some(KeyCode::Enter));
        assert_eq!(SettingsService::parse_key("esc"), Some(KeyCode::Esc));
        assert_eq!(SettingsService::parse_key("invalid"), None);
    }
}