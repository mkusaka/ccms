use crate::interactive_ratatui::ui::components::{
    Component,
    view_layout::{Styles, ViewLayout},
};
use crate::interactive_ratatui::ui::events::Message;
use crate::query::condition::SearchResult;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::Modifier,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
};

#[derive(Default)]
pub struct ResultDetail {
    pub(super) result: Option<SearchResult>,
    pub(super) scroll_offset: usize,
    pub(super) message: Option<String>,
}

impl ResultDetail {
    pub fn new() -> Self {
        Self {
            result: None,
            scroll_offset: 0,
            message: None,
        }
    }

    pub fn set_result(&mut self, result: SearchResult) {
        self.result = Some(result);
        self.scroll_offset = 0;
    }

    #[allow(dead_code)]
    pub fn clear(&mut self) {
        self.result = None;
        self.scroll_offset = 0;
    }

    pub fn set_message(&mut self, message: Option<String>) {
        self.message = message;
    }

    fn render_content(&mut self, f: &mut Frame, area: Rect) {
        let Some(result) = &self.result else {
            return;
        };

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(0),     // Content
                Constraint::Length(10), // Actions
                Constraint::Length(2),  // Status/Message
            ])
            .split(area);

        // Format timestamp
        let timestamp = if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(&result.timestamp) {
            dt.format("%Y-%m-%d %H:%M:%S %Z").to_string()
        } else {
            result.timestamp.clone()
        };

        let content = vec![
            Line::from(vec![
                Span::styled("Role: ", Styles::label()),
                Span::raw(&result.role),
            ]),
            Line::from(vec![
                Span::styled("Time: ", Styles::label()),
                Span::raw(&timestamp),
            ]),
            Line::from(vec![
                Span::styled("File: ", Styles::label()),
                Span::raw(&result.file),
            ]),
            Line::from(vec![
                Span::styled("Project: ", Styles::label()),
                Span::raw(&result.project_path),
            ]),
            Line::from(vec![
                Span::styled("UUID: ", Styles::label()),
                Span::raw(&result.uuid),
            ]),
            Line::from(vec![
                Span::styled("Session: ", Styles::label()),
                Span::raw(&result.session_id),
            ]),
            Line::from(""),
            Line::from("─".repeat(80)),
            Line::from(""),
        ];

        // Build all lines including the message content
        let mut all_lines = content;

        // Calculate visible area for wrapping
        let inner_area = Block::default().borders(Borders::ALL).inner(chunks[0]);
        let visible_height = inner_area.height as usize;
        let available_width = inner_area.width as usize;

        // Wrap message text to fit width
        for line in result.text.lines() {
            if line.is_empty() {
                all_lines.push(Line::from(""));
            } else {
                // Wrap long lines
                let mut remaining = line;
                while !remaining.is_empty() {
                    let mut end_idx = remaining.len().min(available_width);

                    // Find safe break point at character boundary
                    while end_idx > 0 && !remaining.is_char_boundary(end_idx) {
                        end_idx -= 1;
                    }

                    // If we're not at the end, try to break at a word boundary
                    if end_idx < remaining.len() && end_idx > 0 {
                        if let Some(space_pos) = remaining[..end_idx].rfind(' ') {
                            if space_pos > available_width / 2 {
                                end_idx = space_pos + 1; // Include the space
                            }
                        }
                    }

                    all_lines.push(Line::from(&remaining[..end_idx]));
                    remaining = &remaining[end_idx..];
                }
            }
        }

        // Apply scroll offset
        let display_lines: Vec<Line> = all_lines
            .iter()
            .skip(self.scroll_offset)
            .take(visible_height)
            .cloned()
            .collect();

        let total_lines = all_lines.len();
        let detail = Paragraph::new(display_lines)
            .block(Block::default().borders(Borders::ALL).title(format!(
                "Result Detail (↑/↓ or j/k to scroll, line {}/{})",
                self.scroll_offset + 1,
                total_lines
            )))
            .wrap(Wrap { trim: true });
        f.render_widget(detail, chunks[0]);

        // Actions
        let actions = vec![
            Line::from(vec![Span::styled("Actions:", Styles::title())]),
            Line::from(vec![
                Span::styled("[S]", Styles::action_key()),
                Span::styled(" - View full session", Styles::action_description()),
            ]),
            Line::from(vec![
                Span::styled("[F]", Styles::action_key()),
                Span::styled(" - Copy file path", Styles::action_description()),
            ]),
            Line::from(vec![
                Span::styled("[I]", Styles::action_key()),
                Span::styled(" - Copy session ID", Styles::action_description()),
            ]),
            Line::from(vec![
                Span::styled("[P]", Styles::action_key()),
                Span::styled(" - Copy project path", Styles::action_description()),
            ]),
            Line::from(vec![
                Span::styled("[M]", Styles::action_key()),
                Span::styled(" - Copy message text", Styles::action_description()),
            ]),
            Line::from(vec![
                Span::styled("[R]", Styles::action_key()),
                Span::styled(" - Copy raw JSON", Styles::action_description()),
            ]),
            Line::from(vec![
                Span::styled("[Esc]", Styles::action_key()),
                Span::styled(" - Back to search", Styles::action_description()),
            ]),
            Line::from(vec![
                Span::styled("[↑/↓ or j/k]", Styles::action_key()),
                Span::styled(" - Scroll message", Styles::action_description()),
            ]),
        ];

        let actions_widget = Paragraph::new(actions).block(Block::default().borders(Borders::ALL));
        f.render_widget(actions_widget, chunks[1]);

        // Show message if any
        if let Some(ref msg) = self.message {
            let style = if msg.starts_with('✓') {
                Styles::success()
            } else if msg.starts_with('⚠') {
                Styles::warning()
            } else {
                Styles::normal().add_modifier(Modifier::BOLD)
            };

            let message_widget = Paragraph::new(msg.clone())
                .style(style)
                .alignment(ratatui::layout::Alignment::Center);
            f.render_widget(message_widget, chunks[2]);
        }
    }
}

impl Component for ResultDetail {
    fn render(&mut self, f: &mut Frame, area: Rect) {
        let Some(_result) = &self.result else {
            return;
        };

        let layout = ViewLayout::new("Result Detail".to_string()).with_status_bar(false); // We'll handle status manually for now

        layout.render(f, area, |f, content_area| {
            self.render_content(f, content_area);
        });
    }

    fn handle_key(&mut self, key: KeyEvent) -> Option<Message> {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                if self.scroll_offset > 0 {
                    self.scroll_offset -= 1;
                }
                None
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.scroll_offset += 1;
                None
            }
            KeyCode::PageUp => {
                self.scroll_offset = self.scroll_offset.saturating_sub(10);
                None
            }
            KeyCode::PageDown => {
                self.scroll_offset += 10;
                None
            }
            KeyCode::Char('s') | KeyCode::Char('S') => Some(Message::EnterSessionViewer),
            KeyCode::Char('f') | KeyCode::Char('F') => self
                .result
                .as_ref()
                .map(|result| Message::CopyToClipboard(result.file.clone())),
            KeyCode::Char('i') | KeyCode::Char('I') => self
                .result
                .as_ref()
                .map(|result| Message::CopyToClipboard(result.session_id.clone())),
            KeyCode::Char('p') | KeyCode::Char('P') => self
                .result
                .as_ref()
                .map(|result| Message::CopyToClipboard(result.project_path.clone())),
            KeyCode::Char('m') | KeyCode::Char('M') => self
                .result
                .as_ref()
                .map(|result| Message::CopyToClipboard(result.text.clone())),
            KeyCode::Char('r') | KeyCode::Char('R') => {
                if let Some(result) = &self.result {
                    if let Some(raw_json) = &result.raw_json {
                        Some(Message::CopyToClipboard(raw_json.clone()))
                    } else {
                        let formatted = format!(
                            "File: {}\nUUID: {}\nTimestamp: {}\nSession ID: {}\nRole: {}\nText: {}\nProject: {}",
                            result.file,
                            result.uuid,
                            result.timestamp,
                            result.session_id,
                            result.role,
                            result.text,
                            result.project_path
                        );
                        Some(Message::CopyToClipboard(formatted))
                    }
                } else {
                    None
                }
            }
            KeyCode::Char('c') | KeyCode::Char('C') => self
                .result
                .as_ref()
                .map(|result| Message::CopyToClipboard(result.text.clone())),
            KeyCode::Esc => Some(Message::ExitToSearch),
            _ => None,
        }
    }
}
