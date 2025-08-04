use crate::interactive_ratatui::ui::app_state::SessionInfo;
use crate::interactive_ratatui::ui::components::Component;
use crate::interactive_ratatui::ui::events::Message;
use crossterm::event::KeyEvent;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
};

#[derive(Default)]
pub struct SessionPreview {
    session_info: Option<SessionInfo>,
    query: String,
}

impl SessionPreview {
    pub fn new() -> Self {
        Self {
            session_info: None,
            query: String::new(),
        }
    }

    pub fn set_session(&mut self, session: Option<SessionInfo>) {
        self.session_info = session;
    }

    pub fn set_query(&mut self, query: String) {
        self.query = query;
    }
}

impl Component for SessionPreview {
    fn render(&mut self, f: &mut Frame, area: Rect) {
        let block = Block::default()
            .borders(Borders::ALL)
            .title(" Session Preview ")
            .border_style(Style::default().fg(Color::DarkGray));

        if let Some(session) = &self.session_info {
            let mut lines = vec![];

            // Session ID
            lines.push(Line::from(vec![
                Span::styled("Session ID: ", Style::default().fg(Color::Gray)),
                Span::styled(
                    &session.session_id,
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
            ]));

            // Timestamp
            lines.push(Line::from(vec![
                Span::styled("Time: ", Style::default().fg(Color::Gray)),
                Span::styled(&session.timestamp, Style::default().fg(Color::Yellow)),
            ]));

            // Message count
            lines.push(Line::from(vec![
                Span::styled("Messages: ", Style::default().fg(Color::Gray)),
                Span::styled(
                    format!("{}", session.message_count),
                    Style::default().fg(Color::Green),
                ),
            ]));

            lines.push(Line::from(""));

            // File path (truncated if needed)
            lines.push(Line::from(vec![Span::styled(
                "Path: ",
                Style::default().fg(Color::Gray),
            )]));
            lines.push(Line::from(vec![Span::styled(
                &session.file_path,
                Style::default().fg(Color::Blue).add_modifier(Modifier::DIM),
            )]));

            lines.push(Line::from(""));

            // Summary (if available)
            if let Some(summary) = &session.summary {
                lines.push(Line::from(vec![Span::styled(
                    "Summary:",
                    Style::default()
                        .fg(Color::Magenta)
                        .add_modifier(Modifier::BOLD),
                )]));

                // Highlight matching parts in summary
                let mut summary_spans = vec![];
                if !self.query.is_empty() {
                    let query_lower = self.query.to_lowercase();
                    let summary_lower = summary.to_lowercase();
                    let mut last_end = 0;

                    // Find all occurrences of the query in the summary
                    while let Some(start) = summary_lower[last_end..].find(&query_lower) {
                        let absolute_start = last_end + start;
                        let absolute_end = absolute_start + query_lower.len();

                        // Add text before match
                        if absolute_start > last_end {
                            summary_spans.push(Span::styled(
                                &summary[last_end..absolute_start],
                                Style::default().fg(Color::White),
                            ));
                        }

                        // Add matched text with highlight
                        summary_spans.push(Span::styled(
                            &summary[absolute_start..absolute_end],
                            Style::default()
                                .fg(Color::Yellow)
                                .add_modifier(Modifier::BOLD),
                        ));

                        last_end = absolute_end;
                    }

                    // Add remaining text after last match
                    if last_end < summary.len() {
                        summary_spans.push(Span::styled(
                            &summary[last_end..],
                            Style::default().fg(Color::White),
                        ));
                    }
                } else {
                    // No query, just display summary normally
                    summary_spans.push(Span::styled(summary, Style::default().fg(Color::White)));
                }

                lines.push(Line::from(summary_spans));
                lines.push(Line::from(""));
            }

            // Preview messages
            if !session.preview_messages.is_empty() {
                lines.push(Line::from(vec![Span::styled(
                    "Recent Messages:",
                    Style::default()
                        .fg(Color::Gray)
                        .add_modifier(Modifier::BOLD),
                )]));

                // Separate messages into matching and non-matching
                let mut matching_messages = vec![];
                let mut non_matching_messages = vec![];

                if !self.query.is_empty() {
                    let query_lower = self.query.to_lowercase();
                    for (role, content, timestamp) in &session.preview_messages {
                        if content.to_lowercase().contains(&query_lower) {
                            matching_messages.push((role, content, timestamp, true));
                        } else {
                            non_matching_messages.push((role, content, timestamp, false));
                        }
                    }
                } else {
                    // No query, all messages are non-matching
                    for (role, content, timestamp) in &session.preview_messages {
                        non_matching_messages.push((role, content, timestamp, false));
                    }
                }

                // Display matching messages first
                let matching_count = matching_messages.len();
                for (role, content, timestamp, is_match) in matching_messages {
                    let role_color = match role.as_str() {
                        "user" => Color::Green,
                        "assistant" => Color::Blue,
                        _ => Color::Gray,
                    };

                    // Format timestamp
                    let formatted_time =
                        if let Ok(parsed) = chrono::DateTime::parse_from_rfc3339(timestamp) {
                            parsed.format("%H:%M:%S").to_string()
                        } else {
                            timestamp.chars().take(8).collect::<String>()
                        };

                    // Build spans for content with highlighting
                    let mut content_spans = vec![
                        Span::styled(
                            format!("[{formatted_time}] "),
                            Style::default().fg(Color::DarkGray),
                        ),
                        Span::styled(
                            format!("{role}: "),
                            Style::default().fg(role_color).add_modifier(Modifier::BOLD),
                        ),
                    ];

                    // If there's a match, highlight matching parts
                    if is_match && !self.query.is_empty() {
                        let query_lower = self.query.to_lowercase();
                        let content_lower = content.to_lowercase();
                        let mut last_end = 0;

                        // Find all occurrences of the query in the content
                        while let Some(start) = content_lower[last_end..].find(&query_lower) {
                            let absolute_start = last_end + start;
                            let absolute_end = absolute_start + query_lower.len();

                            // Add text before match
                            if absolute_start > last_end {
                                content_spans.push(Span::styled(
                                    &content[last_end..absolute_start],
                                    Style::default().fg(Color::White),
                                ));
                            }

                            // Add matched text with highlight
                            content_spans.push(Span::styled(
                                &content[absolute_start..absolute_end],
                                Style::default()
                                    .fg(Color::Yellow)
                                    .add_modifier(Modifier::BOLD),
                            ));

                            last_end = absolute_end;
                        }

                        // Add remaining text after last match
                        if last_end < content.len() {
                            content_spans.push(Span::styled(
                                &content[last_end..],
                                Style::default().fg(Color::White),
                            ));
                        }
                    } else {
                        // No match or no query, just display content normally
                        content_spans
                            .push(Span::styled(content, Style::default().fg(Color::White)));
                    }

                    lines.push(Line::from(content_spans));
                }

                // Then display remaining messages (up to limit)
                let remaining_space = 5 - matching_count;
                for (role, content, timestamp, _) in
                    non_matching_messages.into_iter().take(remaining_space)
                {
                    let role_color = match role.as_str() {
                        "user" => Color::Green,
                        "assistant" => Color::Blue,
                        _ => Color::Gray,
                    };

                    // Format timestamp
                    let formatted_time =
                        if let Ok(parsed) = chrono::DateTime::parse_from_rfc3339(timestamp) {
                            parsed.format("%H:%M:%S").to_string()
                        } else {
                            timestamp.chars().take(8).collect::<String>()
                        };

                    lines.push(Line::from(vec![
                        Span::styled(
                            format!("[{formatted_time}] "),
                            Style::default().fg(Color::DarkGray),
                        ),
                        Span::styled(
                            format!("{role}: "),
                            Style::default().fg(role_color).add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(content, Style::default().fg(Color::White)),
                    ]));
                }
            } else {
                // Fallback to first message if no preview messages
                lines.push(Line::from(vec![Span::styled(
                    "First Message:",
                    Style::default().fg(Color::Gray),
                )]));
                lines.push(Line::from(vec![Span::styled(
                    &session.first_message,
                    Style::default().fg(Color::White),
                )]));
            }

            lines.push(Line::from(""));
            lines.push(Line::from(vec![Span::styled(
                "Press Enter to open this session",
                Style::default()
                    .fg(Color::DarkGray)
                    .add_modifier(Modifier::ITALIC),
            )]));

            let preview = Paragraph::new(lines)
                .block(block)
                .wrap(Wrap { trim: true })
                .style(Style::default().fg(Color::White));

            f.render_widget(preview, area);
        } else {
            let preview = Paragraph::new("No session selected")
                .block(block)
                .style(Style::default().fg(Color::DarkGray));

            f.render_widget(preview, area);
        }
    }

    fn handle_key(&mut self, _key: KeyEvent) -> Option<Message> {
        // Preview doesn't handle any keys
        None
    }
}
