use crate::interactive_ratatui::ui::components::Component;
use crate::interactive_ratatui::ui::components::view_layout::Styles;
use crate::interactive_ratatui::ui::events::Message;
use crate::query::condition::SearchResult;
use crossterm::event::KeyEvent;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
};

pub struct MessagePreview {
    result: Option<SearchResult>,
}

impl MessagePreview {
    pub fn new() -> Self {
        Self { result: None }
    }

    pub fn set_result(&mut self, result: Option<SearchResult>) {
        let _ = crate::interactive_ratatui::debug::write_debug_log(
            &format!("MessagePreview::set_result: has_result = {}", result.is_some())
        );
        self.result = result;
    }

    fn format_timestamp(timestamp: &str) -> String {
        if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(timestamp) {
            dt.format("%Y-%m-%d %H:%M:%S").to_string()
        } else {
            timestamp.to_string()
        }
    }
}

impl Default for MessagePreview {
    fn default() -> Self {
        Self::new()
    }
}

impl Component for MessagePreview {
    fn render(&mut self, f: &mut Frame, area: Rect) {
        let _ = crate::interactive_ratatui::debug::write_debug_log(
            &format!("MessagePreview::render: area = {:?}, has_result = {}", area, self.result.is_some())
        );
        
        let block = Block::default().borders(Borders::ALL).title("Preview");

        let inner = block.inner(area);
        f.render_widget(block, area);

        if let Some(result) = &self.result {
            // Split the inner area into header and content
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(5), // Header info (4 lines + 1 separator)
                    Constraint::Min(0),    // Message content
                ])
                .split(inner);

            // Render header information
            let header_lines = vec![
                Line::from(vec![
                    Span::styled("Role: ", Styles::label()),
                    Span::raw(&result.role),
                ]),
                Line::from(vec![
                    Span::styled("Time: ", Styles::label()),
                    Span::raw(Self::format_timestamp(&result.timestamp)),
                ]),
                Line::from(vec![
                    Span::styled("Message ID: ", Styles::label()),
                    Span::raw(&result.uuid),
                ]),
                Line::from(vec![
                    Span::styled("Session ID: ", Styles::label()),
                    Span::raw(&result.session_id),
                ]),
                Line::from("──────────────────────────────────"),
            ];

            let header = Paragraph::new(header_lines);
            f.render_widget(header, chunks[0]);

            // Calculate available space for message content
            let content_height = chunks[1].height as usize;
            let content_width = chunks[1].width as usize;

            // Process message text with word wrapping
            let mut display_lines = Vec::new();
            let mut total_lines = 0;

            for line in result.text.lines() {
                if total_lines >= content_height.saturating_sub(1) {
                    // Leave room for "..." indicator
                    break;
                }

                if line.is_empty() {
                    display_lines.push(Line::from(""));
                    total_lines += 1;
                } else {
                    // Word wrap long lines
                    let mut remaining = line;
                    while !remaining.is_empty() && total_lines < content_height.saturating_sub(1) {
                        let mut end_idx = remaining.len().min(content_width);

                        // Find safe break point at character boundary
                        while end_idx > 0 && !remaining.is_char_boundary(end_idx) {
                            end_idx -= 1;
                        }

                        // Try to break at word boundary
                        if end_idx < remaining.len() && end_idx > 0 {
                            if let Some(space_pos) = remaining[..end_idx].rfind(' ') {
                                if space_pos > content_width / 2 {
                                    end_idx = space_pos + 1;
                                }
                            }
                        }

                        display_lines.push(Line::from(&remaining[..end_idx]));
                        remaining = &remaining[end_idx..];
                        total_lines += 1;
                    }
                }
            }

            // Add truncation indicator if content was cut off
            if total_lines >= content_height.saturating_sub(1)
                || result.text.lines().count() > display_lines.len()
            {
                display_lines.push(Line::from(vec![
                    Span::styled("... ", Styles::dimmed()),
                    Span::styled(
                        "(Enter for full view)",
                        Styles::dimmed().add_modifier(ratatui::style::Modifier::ITALIC),
                    ),
                ]));
            }

            let content = Paragraph::new(display_lines).wrap(Wrap { trim: true });
            f.render_widget(content, chunks[1]);
        } else {
            // No result selected
            let empty_message = Paragraph::new("No message selected")
                .style(Styles::dimmed())
                .alignment(ratatui::layout::Alignment::Center);
            f.render_widget(empty_message, inner);
        }
    }

    fn handle_key(&mut self, _key: KeyEvent) -> Option<Message> {
        // Preview is read-only, no key handling needed
        None
    }
}
