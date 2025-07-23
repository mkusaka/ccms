use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};

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
    fn create_truncated_line(&self, max_width: usize) -> Line {
        let timestamp = self.format_timestamp();
        let content = truncate_message(self.get_content(), max_width);

        vec![
            Span::styled(
                format!("{timestamp:16} "),
                Style::default().fg(Color::DarkGray),
            ),
            Span::styled(
                format!("{:10} ", self.get_role()),
                Style::default().fg(self.get_role_color()),
            ),
            Span::raw(content),
        ]
        .into()
    }

    /// Creates the display lines for full text mode
    fn create_full_lines(&self, max_width: usize) -> Vec<Line> {
        let timestamp = self.format_timestamp();
        let wrapped_lines = wrap_text(self.get_content(), max_width);
        let mut lines = Vec::new();

        // First line with metadata
        let first_line_spans = vec![
            Span::styled(
                format!("{timestamp:16} "),
                Style::default().fg(Color::DarkGray),
            ),
            Span::styled(
                format!("{:10} ", self.get_role()),
                Style::default().fg(self.get_role_color()),
            ),
            Span::raw(wrapped_lines.first().cloned().unwrap_or_default()),
        ];
        lines.push(Line::from(first_line_spans));

        // Additional lines (indented)
        for line in wrapped_lines.iter().skip(1) {
            let indent = " ".repeat(29); // 16 + 1 + 10 + 1 + 1 spaces
            lines.push(Line::from(format!("{indent}{line}")));
        }

        lines
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
