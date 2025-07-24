use tuirealm::command::{Cmd, CmdResult, Direction};
use tuirealm::event::{Key, KeyEvent};
use tuirealm::props::{AttrValue, Attribute, Props};
use tuirealm::{Component, Event, MockComponent, NoUserEvent, State, StateValue};

use ratatui::layout::{Constraint, Direction as RatatuiDirection, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::interactive_ratatui::ui::tuirealm_components::messages::AppMessage;
use crate::query::condition::SearchResult;

/// Result detail component for tui-realm
pub struct ResultDetail {
    props: Props,
    /// The search result being displayed
    result: Option<SearchResult>,
    /// Scroll offset for content
    scroll_offset: usize,
    /// Message to display
    message: Option<String>,
    /// Total lines of content
    total_lines: usize,
}

impl Default for ResultDetail {
    fn default() -> Self {
        Self::new()
    }
}

impl ResultDetail {
    pub fn new() -> Self {
        Self {
            props: Props::default(),
            result: None,
            scroll_offset: 0,
            message: None,
            total_lines: 0,
        }
    }

    pub fn set_result(&mut self, result: SearchResult) {
        self.result = Some(result);
        self.scroll_offset = 0;
        self.total_lines = 0; // Will be calculated during render
    }

    pub fn clear(&mut self) {
        self.result = None;
        self.scroll_offset = 0;
        self.message = None;
        self.total_lines = 0;
    }

    pub fn set_message(&mut self, message: Option<String>) {
        self.message = message;
    }

    pub fn get_result(&self) -> Option<&SearchResult> {
        self.result.as_ref()
    }

    fn scroll_up(&mut self) {
        if self.scroll_offset > 0 {
            self.scroll_offset -= 1;
        }
    }

    fn scroll_down(&mut self) {
        self.scroll_offset += 1;
        // We don't know the max scroll here, it will be clamped during render
    }

    fn page_up(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_sub(10);
    }

    fn page_down(&mut self) {
        self.scroll_offset += 10;
        // We don't know the max scroll here, it will be clamped during render
    }

    fn create_content_lines(&self, result: &SearchResult, available_width: usize) -> Vec<Line<'static>> {
        // Format timestamp
        let timestamp = if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(&result.timestamp) {
            dt.format("%Y-%m-%d %H:%M:%S %Z").to_string()
        } else {
            result.timestamp.clone()
        };

        let mut lines = vec![
            Line::from(vec![
                Span::styled("Role: ", Style::default().fg(Color::Yellow)),
                Span::raw(result.role.clone()),
            ]),
            Line::from(vec![
                Span::styled("Time: ", Style::default().fg(Color::Yellow)),
                Span::raw(timestamp),
            ]),
            Line::from(vec![
                Span::styled("File: ", Style::default().fg(Color::Yellow)),
                Span::raw(result.file.clone()),
            ]),
            Line::from(vec![
                Span::styled("Project: ", Style::default().fg(Color::Yellow)),
                Span::raw(result.project_path.clone()),
            ]),
            Line::from(vec![
                Span::styled("UUID: ", Style::default().fg(Color::Yellow)),
                Span::raw(result.uuid.clone()),
            ]),
            Line::from(vec![
                Span::styled("Session: ", Style::default().fg(Color::Yellow)),
                Span::raw(result.session_id.clone()),
            ]),
            Line::from(""),
            Line::from("─".repeat(80.min(available_width))),
            Line::from(""),
        ];

        // Wrap message text to fit width
        for line in result.text.lines() {
            if line.is_empty() {
                lines.push(Line::from(""));
            } else {
                // Wrap long lines
                let mut remaining = line.to_string();
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

                    lines.push(Line::from(remaining[..end_idx].to_string()));
                    remaining = remaining[end_idx..].to_string();
                }
            }
        }

        lines
    }

    fn create_actions_content() -> Vec<Line<'static>> {
        vec![
            Line::from(vec![Span::styled(
                "Actions:",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from(vec![
                Span::styled("[S]", Style::default().fg(Color::Yellow)),
                Span::raw(" - View full session"),
            ]),
            Line::from(vec![
                Span::styled("[F]", Style::default().fg(Color::Yellow)),
                Span::raw(" - Copy file path"),
            ]),
            Line::from(vec![
                Span::styled("[I]", Style::default().fg(Color::Yellow)),
                Span::raw(" - Copy session ID"),
            ]),
            Line::from(vec![
                Span::styled("[P]", Style::default().fg(Color::Yellow)),
                Span::raw(" - Copy project path"),
            ]),
            Line::from(vec![
                Span::styled("[M]", Style::default().fg(Color::Yellow)),
                Span::raw(" - Copy message text"),
            ]),
            Line::from(vec![
                Span::styled("[R]", Style::default().fg(Color::Yellow)),
                Span::raw(" - Copy raw JSON"),
            ]),
            Line::from(vec![
                Span::styled("[Esc]", Style::default().fg(Color::Yellow)),
                Span::raw(" - Back to search"),
            ]),
            Line::from(vec![
                Span::styled("[↑/↓ or j/k]", Style::default().fg(Color::Yellow)),
                Span::raw(" - Scroll message"),
            ]),
        ]
    }
}

impl MockComponent for ResultDetail {
    fn view(&mut self, frame: &mut Frame, area: Rect) {
        let Some(result) = &self.result else {
            let empty = Paragraph::new("No result selected")
                .block(Block::default().title("Result Detail").borders(Borders::ALL))
                .style(Style::default().fg(Color::DarkGray));
            frame.render_widget(empty, area);
            return;
        };

        let chunks = Layout::default()
            .direction(RatatuiDirection::Vertical)
            .constraints([
                Constraint::Min(0),     // Content
                Constraint::Length(10), // Actions
                Constraint::Length(2),  // Status/Message
            ])
            .split(area);

        // Calculate available width for content
        let inner_area = Block::default().borders(Borders::ALL).inner(chunks[0]);
        let available_width = inner_area.width as usize;
        let visible_height = inner_area.height as usize;

        // Create content lines
        let all_lines = self.create_content_lines(result, available_width);
        self.total_lines = all_lines.len();

        // Clamp scroll offset
        let max_scroll = self.total_lines.saturating_sub(visible_height);
        if self.scroll_offset > max_scroll {
            self.scroll_offset = max_scroll;
        }

        // Apply scroll offset
        let display_lines: Vec<Line> = all_lines
            .into_iter()
            .skip(self.scroll_offset)
            .take(visible_height)
            .collect();

        let detail = Paragraph::new(display_lines).block(
            Block::default().borders(Borders::ALL).title(format!(
                "Result Detail (↑/↓ or j/k to scroll, line {}/{})",
                self.scroll_offset + 1,
                self.total_lines
            )),
        );
        frame.render_widget(detail, chunks[0]);

        // Actions
        let actions = Self::create_actions_content();
        let actions_widget = Paragraph::new(actions).block(Block::default().borders(Borders::ALL));
        frame.render_widget(actions_widget, chunks[1]);

        // Show message if any
        if let Some(ref msg) = self.message {
            let style = if msg.starts_with('✓') {
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD)
            } else if msg.starts_with('⚠') {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().add_modifier(Modifier::BOLD)
            };

            let message_widget = Paragraph::new(msg.clone())
                .style(style)
                .alignment(ratatui::layout::Alignment::Center);
            frame.render_widget(message_widget, chunks[2]);
        }
    }

    fn query(&self, attr: Attribute) -> Option<AttrValue> {
        self.props.get(attr)
    }

    fn attr(&mut self, attr: Attribute, value: AttrValue) {
        self.props.set(attr, value);
    }

    fn state(&self) -> State {
        State::One(StateValue::Usize(self.scroll_offset))
    }

    fn perform(&mut self, cmd: Cmd) -> CmdResult {
        match cmd {
            Cmd::Move(Direction::Up) => {
                self.scroll_up();
                CmdResult::Changed(self.state())
            }
            Cmd::Move(Direction::Down) => {
                self.scroll_down();
                CmdResult::Changed(self.state())
            }
            Cmd::Scroll(Direction::Up) => {
                self.page_up();
                CmdResult::Changed(self.state())
            }
            Cmd::Scroll(Direction::Down) => {
                self.page_down();
                CmdResult::Changed(self.state())
            }
            _ => CmdResult::None,
        }
    }
}

impl Component<AppMessage, NoUserEvent> for ResultDetail {
    fn on(&mut self, ev: Event<NoUserEvent>) -> Option<AppMessage> {
        match ev {
            Event::Keyboard(KeyEvent {
                code: Key::Up, ..
            })
            | Event::Keyboard(KeyEvent {
                code: Key::Char('k'),
                ..
            }) => {
                self.scroll_up();
                None
            }
            Event::Keyboard(KeyEvent {
                code: Key::Down, ..
            })
            | Event::Keyboard(KeyEvent {
                code: Key::Char('j'),
                ..
            }) => {
                self.scroll_down();
                None
            }
            Event::Keyboard(KeyEvent {
                code: Key::PageUp, ..
            }) => {
                self.page_up();
                None
            }
            Event::Keyboard(KeyEvent {
                code: Key::PageDown, ..
            }) => {
                self.page_down();
                None
            }
            Event::Keyboard(KeyEvent {
                code: Key::Char('s'),
                ..
            })
            | Event::Keyboard(KeyEvent {
                code: Key::Char('S'),
                ..
            }) => {
                if let Some(result) = &self.result {
                    Some(AppMessage::EnterSessionViewer(result.session_id.clone()))
                } else {
                    None
                }
            }
            Event::Keyboard(KeyEvent {
                code: Key::Char('f'),
                ..
            })
            | Event::Keyboard(KeyEvent {
                code: Key::Char('F'),
                ..
            }) => self
                .result
                .as_ref()
                .map(|result| AppMessage::CopyToClipboard(result.file.clone())),
            Event::Keyboard(KeyEvent {
                code: Key::Char('i'),
                ..
            })
            | Event::Keyboard(KeyEvent {
                code: Key::Char('I'),
                ..
            }) => self
                .result
                .as_ref()
                .map(|result| AppMessage::CopyToClipboard(result.session_id.clone())),
            Event::Keyboard(KeyEvent {
                code: Key::Char('p'),
                ..
            })
            | Event::Keyboard(KeyEvent {
                code: Key::Char('P'),
                ..
            }) => self
                .result
                .as_ref()
                .map(|result| AppMessage::CopyToClipboard(result.project_path.clone())),
            Event::Keyboard(KeyEvent {
                code: Key::Char('m'),
                ..
            })
            | Event::Keyboard(KeyEvent {
                code: Key::Char('M'),
                ..
            }) => self
                .result
                .as_ref()
                .map(|result| AppMessage::CopyToClipboard(result.text.clone())),
            Event::Keyboard(KeyEvent {
                code: Key::Char('r'),
                ..
            })
            | Event::Keyboard(KeyEvent {
                code: Key::Char('R'),
                ..
            }) => {
                if let Some(result) = &self.result {
                    if let Some(raw_json) = &result.raw_json {
                        Some(AppMessage::CopyToClipboard(raw_json.clone()))
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
                        Some(AppMessage::CopyToClipboard(formatted))
                    }
                } else {
                    None
                }
            }
            Event::Keyboard(KeyEvent {
                code: Key::Char('c'),
                ..
            })
            | Event::Keyboard(KeyEvent {
                code: Key::Char('C'),
                ..
            }) => self
                .result
                .as_ref()
                .map(|result| AppMessage::CopyToClipboard(result.text.clone())),
            Event::Keyboard(KeyEvent {
                code: Key::Esc, ..
            }) => Some(AppMessage::ExitResultDetail),
            _ => None,
        }
    }
}