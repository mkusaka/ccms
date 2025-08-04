use crate::interactive_ratatui::constants::*;
use crate::interactive_ratatui::ui::components::{
    Component, is_exit_prompt,
    view_layout::{Styles, ViewLayout},
};
use crate::interactive_ratatui::ui::events::{CopyContent, Message};
use crate::query::condition::SearchResult;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
};

#[derive(Default)]
pub struct MessageDetail {
    pub(super) result: Option<SearchResult>,
    pub(super) scroll_offset: usize,
    pub(super) message: Option<String>,
    pub(super) current_uuid: Option<String>,
}

impl MessageDetail {
    pub fn new() -> Self {
        Self {
            result: None,
            scroll_offset: 0,
            message: None,
            current_uuid: None,
        }
    }

    pub fn set_result(&mut self, result: SearchResult) {
        // Only reset scroll if it's a different message
        let should_reset_scroll = self.current_uuid.as_ref() != Some(&result.uuid);

        if should_reset_scroll {
            self.scroll_offset = 0;
            self.current_uuid = Some(result.uuid.clone());
        }

        self.result = Some(result);
    }

    pub fn clear(&mut self) {
        self.result = None;
        self.scroll_offset = 0;
        self.current_uuid = None;
    }

    pub fn set_message(&mut self, message: Option<String>) {
        self.message = message;
    }

    fn render_content(&mut self, f: &mut Frame, area: Rect) {
        let Some(result) = &self.result else {
            return;
        };

        // Check if message is exit prompt
        let is_exit = is_exit_prompt(&self.message);
        let non_exit_message = if is_exit { None } else { self.message.clone() };

        // Calculate the actual height needed for the shortcuts bar
        let shortcuts_text = "↑/↓: Scroll | Ctrl+S: View full session | c: Copy message text | C: Copy as JSON | i: Copy session ID | f: Copy file path | p: Copy project path | Alt+←/→: Navigate history | Esc: Back";
        let shortcuts_paragraph = Paragraph::new(shortcuts_text).wrap(Wrap { trim: true });
        let shortcuts_height = (shortcuts_paragraph.line_count(area.width) as u16).clamp(1, 3);

        // Split the main area into header, message, shortcuts, and optionally status/exit prompt
        let chunks = if is_exit || non_exit_message.is_some() {
            Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(MESSAGE_DETAIL_HEADER_HEIGHT), // Header (fixed)
                    Constraint::Min(5), // Message content (scrollable)
                    Constraint::Length(shortcuts_height), // Shortcuts (dynamic height)
                    Constraint::Length(MESSAGE_DETAIL_STATUS_HEIGHT), // Status/Exit prompt at bottom
                ])
                .split(area)
        } else {
            Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(MESSAGE_DETAIL_HEADER_HEIGHT), // Header (fixed)
                    Constraint::Min(5), // Message content (scrollable)
                    Constraint::Length(shortcuts_height), // Shortcuts (dynamic height)
                ])
                .split(area)
        };

        // Format timestamp
        let timestamp = if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(&result.timestamp) {
            // Convert to local timezone
            use chrono::{Local, TimeZone};
            let local_dt = Local.from_utc_datetime(&dt.naive_utc());
            local_dt.format("%Y-%m-%d %H:%M:%S %Z").to_string()
        } else {
            result.timestamp.clone()
        };

        // Render header information (fixed)
        let header_lines = vec![
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
                Span::styled("CWD: ", Styles::label()),
                Span::raw(&result.cwd),
            ]),
            Line::from(vec![
                Span::styled("Message ID: ", Styles::label()),
                Span::raw(&result.uuid),
            ]),
            Line::from(vec![
                Span::styled("Session: ", Styles::label()),
                Span::raw(&result.session_id),
            ]),
        ];

        let header = Paragraph::new(header_lines)
            .block(Block::default().borders(Borders::ALL).title("Details"));
        f.render_widget(header, chunks[0]);

        // Process message content for the scrollable area
        let mut message_lines = Vec::new();

        // Calculate visible area for wrapping
        let inner_area = Block::default().borders(Borders::ALL).inner(chunks[1]);
        let visible_height = inner_area.height as usize;
        let available_width = inner_area.width as usize;

        // Wrap message text to fit width
        for line in result.text.lines() {
            if line.is_empty() {
                message_lines.push(Line::from(""));
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
                    if end_idx < remaining.len() && end_idx > 0
                        && let Some(space_pos) = remaining[..end_idx].rfind(' ')
                            && space_pos > available_width / 2 {
                                end_idx = space_pos + 1; // Include the space
                            }

                    message_lines.push(Line::from(&remaining[..end_idx]));
                    remaining = &remaining[end_idx..];
                }
            }
        }

        // Calculate the maximum scroll offset
        let max_scroll = message_lines.len().saturating_sub(visible_height);

        // Ensure scroll offset doesn't exceed bounds
        if self.scroll_offset > max_scroll {
            self.scroll_offset = max_scroll;
        }

        // Apply scroll offset to message lines only
        let display_lines: Vec<Line> = message_lines
            .iter()
            .skip(self.scroll_offset)
            .take(visible_height)
            .cloned()
            .collect();

        let total_lines = message_lines.len();
        let message_widget = Paragraph::new(display_lines)
            .block(Block::default().borders(Borders::ALL).title(format!(
                "Message (↑/↓ to scroll, line {}-{} of {})",
                if total_lines > 0 {
                    self.scroll_offset + 1
                } else {
                    0
                },
                if total_lines > 0 {
                    (self.scroll_offset + visible_height).min(total_lines)
                } else {
                    0
                },
                total_lines
            )))
            .wrap(Wrap { trim: true });
        f.render_widget(message_widget, chunks[1]);

        // Render shortcuts bar (similar to Session Viewer style)
        let shortcuts_text = "↑/↓: Scroll | Ctrl+S: View full session | c: Copy message text | C: Copy as JSON | i: Copy session ID | f: Copy file path | p: Copy project path | Alt+←/→: Navigate history | Esc: Back";
        let shortcuts_bar = Paragraph::new(shortcuts_text)
            .style(Style::default().fg(Color::DarkGray))
            .alignment(ratatui::layout::Alignment::Center)
            .wrap(Wrap { trim: true });
        f.render_widget(shortcuts_bar, chunks[2]);

        // Show non-exit message if any
        if let Some(ref msg) = non_exit_message {
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
            f.render_widget(message_widget, chunks[3]);
        }

        // Render exit prompt at the very bottom if needed
        if is_exit {
            let exit_prompt = Paragraph::new("Press Ctrl+C again to exit")
                .style(
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                )
                .alignment(ratatui::layout::Alignment::Center);
            f.render_widget(exit_prompt, chunks[3]);
        }
    }
}

impl Component for MessageDetail {
    fn render(&mut self, f: &mut Frame, area: Rect) {
        let Some(_result) = &self.result else {
            return;
        };

        let layout = ViewLayout::new("Message Detail".to_string()).with_status_bar(false); // We'll handle status manually for now

        layout.render(f, area, |f, content_area| {
            self.render_content(f, content_area);
        });
    }

    fn handle_key(&mut self, key: KeyEvent) -> Option<Message> {
        match key.code {
            KeyCode::Up => {
                if self.scroll_offset > 0 {
                    self.scroll_offset -= 1;
                }
                None
            }
            KeyCode::Down => {
                // Only scroll if there's content to scroll
                if let Some(result) = &self.result
                    && !result.text.is_empty() {
                        self.scroll_offset += 1;
                    }
                None
            }
            KeyCode::PageUp => {
                self.scroll_offset = self.scroll_offset.saturating_sub(PAGE_SIZE);
                None
            }
            KeyCode::PageDown => {
                // Only scroll if there's content to scroll
                if let Some(result) = &self.result
                    && !result.text.is_empty() {
                        self.scroll_offset += PAGE_SIZE;
                    }
                None
            }
            KeyCode::Char('u') if key.modifiers == KeyModifiers::CONTROL => {
                self.scroll_offset = self.scroll_offset.saturating_sub(PAGE_SIZE);
                None
            }
            KeyCode::Char('d') if key.modifiers == KeyModifiers::CONTROL => {
                // Only scroll if there's content to scroll
                if let Some(result) = &self.result
                    && !result.text.is_empty() {
                        self.scroll_offset += PAGE_SIZE;
                    }
                None
            }
            KeyCode::Char('s') if key.modifiers == KeyModifiers::CONTROL => {
                Some(Message::EnterSessionViewer) // Ctrl+S
            }
            // Unified copy operations
            KeyCode::Char('c') => self.result.as_ref().map(|result| {
                Message::CopyToClipboard(CopyContent::MessageContent(result.text.clone()))
            }),
            KeyCode::Char('C') => {
                if let Some(result) = &self.result {
                    if let Some(raw_json) = &result.raw_json {
                        Some(Message::CopyToClipboard(CopyContent::JsonData(
                            raw_json.clone(),
                        )))
                    } else {
                        let formatted = format!(
                            "File: {}\nUUID: {}\nTimestamp: {}\nSession ID: {}\nRole: {}\nText: {}\nCWD: {}",
                            result.file,
                            result.uuid,
                            result.timestamp,
                            result.session_id,
                            result.role,
                            result.text,
                            result.cwd
                        );
                        Some(Message::CopyToClipboard(CopyContent::FullMessageDetails(
                            formatted,
                        )))
                    }
                } else {
                    None
                }
            }
            KeyCode::Char('i') => self.result.as_ref().map(|result| {
                Message::CopyToClipboard(CopyContent::SessionId(result.session_id.clone()))
            }),
            KeyCode::Char('f') => self
                .result
                .as_ref()
                .map(|result| Message::CopyToClipboard(CopyContent::FilePath(result.file.clone()))),
            KeyCode::Char('p') => self.result.as_ref().map(|result| {
                Message::CopyToClipboard(CopyContent::ProjectPath(result.cwd.clone()))
            }),
            KeyCode::Esc => Some(Message::ExitToSearch),
            _ => None,
        }
    }
}
