//! Theme context for UI styling

use iocraft::prelude::*;

#[derive(Clone, Debug)]
pub struct Theme {
    // Colors
    pub primary_color: Color,
    pub secondary_color: Color,
    pub accent_color: Color,
    pub background_color: Color,
    pub text_color: Color,
    pub error_color: Color,
    pub warning_color: Color,
    pub success_color: Color,
    pub info_color: Color,
    pub highlight_color: Color,
    
    // Border styles
    pub default_border: BorderStyle,
    pub focused_border: BorderStyle,
    pub error_border: BorderStyle,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            // Colors
            primary_color: Color::Blue,
            secondary_color: Color::Cyan,
            accent_color: Color::Yellow,
            background_color: Color::Reset,
            text_color: Color::Reset,
            error_color: Color::Red,
            warning_color: Color::Yellow,
            success_color: Color::Green,
            info_color: Color::Blue,
            highlight_color: Color::DarkGrey,
            
            // Border styles
            default_border: BorderStyle::Single,
            focused_border: BorderStyle::Double,
            error_border: BorderStyle::Single,
        }
    }
}

impl Theme {
    /// Get the color for a specific message role
    pub fn role_color(&self, role: &str) -> Color {
        match role {
            "user" => Color::Green,
            "assistant" => Color::Blue,
            "system" => Color::Yellow,
            _ => Color::Reset,
        }
    }
    
    /// Get the color for search status
    pub fn status_color(&self, status: &str) -> Color {
        match status {
            "typing..." => Color::DarkGrey,
            "searching..." => self.info_color,
            "error" => self.error_color,
            _ => Color::Reset,
        }
    }
}