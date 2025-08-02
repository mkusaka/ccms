use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};

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

    /// Creates the display lines for truncated mode
    fn create_truncated_line(&self, query: &str) -> Line<'static>;

    /// Creates the display lines for full text mode
    fn create_full_lines(&self, max_width: usize, query: &str) -> Vec<Line<'static>>;

    /// Creates column data (text and style) for proper layout
    fn create_column_data(&self, query: &str, truncate: bool, max_content_width: Option<usize>) -> (String, Style, String, Style, Vec<Span<'static>>) {
        let timestamp_text = self.format_timestamp();
        let timestamp_style = Style::default().fg(Color::DarkGray);
        
        let role_text = self.get_role().to_string();
        let role_style = Style::default().fg(self.get_role_color());
        
        let content = self.get_content().replace('\n', " ");
        let content_spans = if truncate {
            if let Some(max_width) = max_content_width {
                let truncated = truncate_message(&content, max_width);
                highlight_text(&truncated, query)
            } else {
                highlight_text(&content, query)
            }
        } else {
            highlight_text(&content, query)
        };
        
        (timestamp_text, timestamp_style, role_text, role_style, content_spans)
    }
}

pub fn truncate_message(text: &str, max_width: usize) -> String {
    let text = text.replace('\n', " ");
    let chars: Vec<char> = text.chars().collect();

    if chars.len() <= max_width {
        text
    } else {
        let truncated: String = chars.into_iter().take(max_width - 3).collect();
        format!("{truncated}...")
    }
}

pub fn wrap_text(text: &str, max_width: usize) -> Vec<String> {
    if max_width == 0 {
        return vec![];
    }

    let text = text.replace('\n', " ");
    let mut lines = Vec::new();
    let mut current_line = String::new();
    let mut current_width = 0;

    for word in text.split_whitespace() {
        let word_width = word.chars().count();

        if current_width > 0 && current_width + 1 + word_width > max_width {
            // Start a new line
            lines.push(current_line.clone());
            current_line = word.to_string();
            current_width = word_width;
        } else {
            // Add to current line
            if current_width > 0 {
                current_line.push(' ');
                current_width += 1;
            }
            current_line.push_str(word);
            current_width += word_width;
        }
    }

    if !current_line.is_empty() {
        lines.push(current_line);
    }

    if lines.is_empty() {
        vec![String::new()]
    } else {
        lines
    }
}

// New helper function for highlighting
pub fn highlight_text(text: &str, query: &str) -> Vec<Span<'static>> {
    if query.is_empty() {
        return vec![Span::raw(text.to_string())];
    }

    let mut spans = Vec::new();
    let mut last_end = 0;

    for (start, matched_text) in text.match_indices(query) {
        if start > last_end {
            spans.push(Span::raw(text[last_end..start].to_string()));
        }
        spans.push(Span::styled(
            matched_text.to_string(),
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        ));
        last_end = start + matched_text.len();
    }

    if last_end < text.len() {
        spans.push(Span::raw(text[last_end..].to_string()));
    }

    spans
}
