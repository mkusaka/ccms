//! Persistent settings management for the iocraft interface
//!
//! Provides functionality to save and load user preferences

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use anyhow::{Result, Context};
use std::fs;

/// User configurable settings
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UserSettings {
    /// Number of visible items in result list
    pub visible_items: usize,
    /// Debounce delay for search in milliseconds
    pub search_debounce_ms: u64,
    /// Default truncation state
    pub truncate_by_default: bool,
    /// Default sort order for session viewer
    pub default_session_order: SessionOrderPreference,
    /// Theme preferences
    pub theme: ThemeSettings,
    /// Keyboard shortcuts
    pub keybindings: KeyBindings,
    /// Performance settings
    pub performance: PerformanceSettings,
}

impl Default for UserSettings {
    fn default() -> Self {
        Self {
            visible_items: 20,
            search_debounce_ms: 300,
            truncate_by_default: true,
            default_session_order: SessionOrderPreference::Descending,
            theme: ThemeSettings::default(),
            keybindings: KeyBindings::default(),
            performance: PerformanceSettings::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SessionOrderPreference {
    Ascending,
    Descending,
    Original,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ThemeSettings {
    pub primary_color: String,
    pub accent_color: String,
    pub success_color: String,
    pub error_color: String,
    pub warning_color: String,
    pub border_style: String,
}

impl Default for ThemeSettings {
    fn default() -> Self {
        Self {
            primary_color: "cyan".to_string(),
            accent_color: "yellow".to_string(),
            success_color: "green".to_string(),
            error_color: "red".to_string(),
            warning_color: "yellow".to_string(),
            border_style: "rounded".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct KeyBindings {
    pub quit: Vec<String>,
    pub help: String,
    pub search: String,
    pub navigate_up: Vec<String>,
    pub navigate_down: Vec<String>,
    pub select: String,
    pub back: Vec<String>,
    pub toggle_truncation: String,
    pub toggle_role_filter: String,
    pub copy_text: String,
    pub copy_json: String,
    pub view_session: String,
}

impl Default for KeyBindings {
    fn default() -> Self {
        Self {
            quit: vec!["ctrl-c".to_string()],
            help: "?".to_string(),
            search: "/".to_string(),
            navigate_up: vec!["up".to_string(), "k".to_string()],
            navigate_down: vec!["down".to_string(), "j".to_string()],
            select: "enter".to_string(),
            back: vec!["esc".to_string(), "backspace".to_string()],
            toggle_truncation: "t".to_string(),
            toggle_role_filter: "tab".to_string(),
            copy_text: "c".to_string(),
            copy_json: "r".to_string(),
            view_session: "s".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PerformanceSettings {
    /// Enable virtual scrolling for large lists
    pub enable_virtual_scrolling: bool,
    /// Maximum items to render at once
    pub max_render_items: usize,
    /// Cache size in MB
    pub cache_size_mb: usize,
    /// Enable memoization
    pub enable_memoization: bool,
}

impl Default for PerformanceSettings {
    fn default() -> Self {
        Self {
            enable_virtual_scrolling: true,
            max_render_items: 100,
            cache_size_mb: 100,
            enable_memoization: true,
        }
    }
}

/// Settings persistence manager
pub struct SettingsManager {
    config_path: PathBuf,
}

impl SettingsManager {
    /// Create a new settings manager with the default config path
    pub fn new() -> Result<Self> {
        let config_dir = Self::get_config_dir()?;
        let config_path = config_dir.join("ccms-iocraft.toml");
        Ok(Self { config_path })
    }
    
    /// Create a settings manager with a custom config path (for testing)
    pub fn with_path(path: PathBuf) -> Self {
        Self { config_path: path }
    }
    
    /// Load settings from disk, creating default if not found
    pub fn load(&self) -> Result<UserSettings> {
        if self.config_path.exists() {
            let contents = fs::read_to_string(&self.config_path)
                .context("Failed to read settings file")?;
            let settings = toml::from_str(&contents)
                .context("Failed to parse settings file")?;
            Ok(settings)
        } else {
            // Create default settings
            let settings = UserSettings::default();
            // Try to save them, but don't fail if we can't
            let _ = self.save(&settings);
            Ok(settings)
        }
    }
    
    /// Save settings to disk
    pub fn save(&self, settings: &UserSettings) -> Result<()> {
        // Ensure config directory exists
        if let Some(parent) = self.config_path.parent() {
            fs::create_dir_all(parent)
                .context("Failed to create config directory")?;
        }
        
        let toml_string = toml::to_string_pretty(settings)
            .context("Failed to serialize settings")?;
        
        fs::write(&self.config_path, toml_string)
            .context("Failed to write settings file")?;
        
        Ok(())
    }
    
    /// Reset settings to defaults
    pub fn reset(&self) -> Result<UserSettings> {
        let default_settings = UserSettings::default();
        self.save(&default_settings)?;
        Ok(default_settings)
    }
    
    /// Get the configuration directory path
    fn get_config_dir() -> Result<PathBuf> {
        // Try to get user config directory
        if let Some(config_dir) = dirs::config_dir() {
            Ok(config_dir.join("ccms"))
        } else if let Ok(home) = std::env::var("HOME") {
            // Fallback to ~/.config
            Ok(PathBuf::from(home).join(".config").join("ccms"))
        } else {
            // Last resort - current directory
            Ok(PathBuf::from(".ccms"))
        }
    }
    
    /// Apply a partial update to settings
    pub fn update<F>(&self, updater: F) -> Result<UserSettings>
    where
        F: FnOnce(&mut UserSettings),
    {
        let mut settings = self.load()?;
        updater(&mut settings);
        self.save(&settings)?;
        Ok(settings)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    
    #[test]
    fn test_default_settings() {
        let settings = UserSettings::default();
        assert_eq!(settings.visible_items, 20);
        assert_eq!(settings.search_debounce_ms, 300);
        assert!(settings.truncate_by_default);
    }
    
    #[test]
    fn test_settings_serialization() {
        let settings = UserSettings::default();
        let toml_string = toml::to_string(&settings).unwrap();
        let deserialized: UserSettings = toml::from_str(&toml_string).unwrap();
        assert_eq!(settings, deserialized);
    }
    
    #[test]
    fn test_settings_manager_save_load() {
        let temp_file = NamedTempFile::new().unwrap();
        let manager = SettingsManager::with_path(temp_file.path().to_path_buf());
        
        // Load default settings
        let settings = manager.load().unwrap();
        assert_eq!(settings, UserSettings::default());
        
        // Modify and save
        let modified = UserSettings {
            visible_items: 50,
            search_debounce_ms: 500,
            ..Default::default()
        };
        manager.save(&modified).unwrap();
        
        // Load again and verify
        let loaded = manager.load().unwrap();
        assert_eq!(loaded.visible_items, 50);
        assert_eq!(loaded.search_debounce_ms, 500);
    }
    
    #[test]
    fn test_settings_update() {
        let temp_file = NamedTempFile::new().unwrap();
        let manager = SettingsManager::with_path(temp_file.path().to_path_buf());
        
        // Update specific fields
        let updated = manager.update(|s| {
            s.visible_items = 30;
            s.theme.primary_color = "blue".to_string();
        }).unwrap();
        
        assert_eq!(updated.visible_items, 30);
        assert_eq!(updated.theme.primary_color, "blue");
        
        // Verify persistence
        let loaded = manager.load().unwrap();
        assert_eq!(loaded.visible_items, 30);
        assert_eq!(loaded.theme.primary_color, "blue");
    }
    
    #[test]
    fn test_reset_settings() {
        let temp_file = NamedTempFile::new().unwrap();
        let manager = SettingsManager::with_path(temp_file.path().to_path_buf());
        
        // Modify settings
        manager.update(|s| {
            s.visible_items = 100;
        }).unwrap();
        
        // Reset to defaults
        let reset = manager.reset().unwrap();
        assert_eq!(reset, UserSettings::default());
        
        // Verify persistence
        let loaded = manager.load().unwrap();
        assert_eq!(loaded, UserSettings::default());
    }
}