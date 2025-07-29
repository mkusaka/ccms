use crate::interactive_ratatui::ui::components::list_item::{ListItem, highlight_text, wrap_text};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};

#[derive(Debug, Clone)]
pub struct SessionListItem {
    pub raw_json: String,
    pub role: String,
    pub timestamp: String,
    pub content: String,
}

impl SessionListItem {
    /// Generates searchable text that matches what the user sees in the display
    /// Format: "{formatted_timestamp} {role} {content}"
    pub fn to_search_text(&self) -> String {
        let formatted_timestamp = self.format_timestamp();
        format!("{} {} {}", formatted_timestamp, self.role, self.content)
    }

    pub fn from_json_line(json_line: &str) -> Option<Self> {
        if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(json_line) {
            // Extract role/type
            let role = json_value
                .get("type")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string();

            // Extract timestamp
            let timestamp = json_value
                .get("timestamp")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            // Extract content based on message type
            let content = match role.as_str() {
                "summary" => json_value
                    .get("summary")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                "system" => json_value
                    .get("content")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                _ => {
                    // For user and assistant messages
                    if let Some(content) = json_value
                        .get("message")
                        .and_then(|m| m.get("content"))
                        .and_then(|c| c.as_str())
                    {
                        content.to_string()
                    } else if let Some(arr) = json_value
                        .get("message")
                        .and_then(|m| m.get("content"))
                        .and_then(|c| c.as_array())
                    {
                        let texts: Vec<String> = arr
                            .iter()
                            .filter_map(|item| {
                                item.get("text")
                                    .and_then(|t| t.as_str())
                                    .map(|s| s.to_string())
                            })
                            .collect();
                        texts.join(" ")
                    } else {
                        String::new()
                    }
                }
            };

            Some(Self {
                raw_json: json_line.to_string(),
                role,
                timestamp,
                content,
            })
        } else {
            None
        }
    }
}

impl ListItem for SessionListItem {
    fn get_role(&self) -> &str {
        &self.role
    }

    fn get_timestamp(&self) -> &str {
        &self.timestamp
    }

    fn get_content(&self) -> &str {
        &self.content
    }

    fn create_truncated_line(&self, query: &str) -> Line<'static> {
        let timestamp = self.format_timestamp();
        // Let ratatui handle truncation - just remove newlines
        let content = self.get_content().replace('\n', " ");
        let highlighted_content = highlight_text(&content, query);

        let mut spans = vec![
            Span::styled(
                format!("{timestamp:16} "),
                Style::default().fg(Color::DarkGray),
            ),
            Span::styled(
                format!("{:10} ", self.get_role()),
                Style::default().fg(self.get_role_color()),
            ),
        ];
        spans.extend(highlighted_content);

        Line::from(spans)
    }

    fn create_full_lines(&self, max_width: usize, query: &str) -> Vec<Line<'static>> {
        let timestamp = self.format_timestamp();
        let wrapped_lines = wrap_text(self.get_content(), max_width);
        let mut lines = Vec::new();

        // First line with metadata
        let first_line_content = wrapped_lines.first().cloned().unwrap_or_default();
        let highlighted_first_line = highlight_text(&first_line_content, query);

        let mut first_line_spans = vec![
            Span::styled(
                format!("{timestamp:16} "),
                Style::default().fg(Color::DarkGray),
            ),
            Span::styled(
                format!("{:10} ", self.get_role()),
                Style::default().fg(self.get_role_color()),
            ),
        ];
        first_line_spans.extend(highlighted_first_line);
        lines.push(Line::from(first_line_spans));

        // Additional lines (indented)
        for line in wrapped_lines.iter().skip(1) {
            let indent = " ".repeat(29); // 16 + 1 + 10 + 1 + 1 spaces
            let highlighted_line = highlight_text(line, query);
            let mut line_spans = vec![Span::raw(indent)];
            line_spans.extend(highlighted_line);
            lines.push(Line::from(line_spans));
        }

        lines
    }
}
