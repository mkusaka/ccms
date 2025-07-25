use ratatui::style::Color;

/// Trait for items that can be displayed in a generic list viewer
pub trait ListItem: Clone {
    /// Returns the role/type of the item (e.g., "user", "assistant", "system")
    fn get_role(&self) -> &str;

    /// Returns the timestamp as a string
    fn get_timestamp(&self) -> &str;

    /// Returns the main content text
    fn get_content(&self) -> &str;

    /// Returns the color for the role
    fn get_role_color(&self) -> Color {
        match self.get_role() {
            "user" => Color::Green,
            "assistant" => Color::Blue,
            "system" => Color::Yellow,
            "summary" => Color::Magenta,
            _ => Color::White,
        }
    }

    /// Formats the timestamp for display
    fn format_timestamp(&self) -> String {
        let timestamp = self.get_timestamp();
        if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(timestamp) {
            dt.format("%m/%d %H:%M").to_string()
        } else if timestamp.len() >= 16 {
            timestamp.chars().take(16).collect()
        } else {
            "N/A".to_string()
        }
    }
}
