use tuirealm::command::{Cmd, CmdResult, Direction, Position};
use tuirealm::event::{Key, KeyEvent, KeyModifiers};
use tuirealm::props::{Alignment, AttrValue, Attribute, Props};
use tuirealm::{Component, Event, MockComponent, NoUserEvent, State, StateValue};

use ratatui::layout::Rect;
use ratatui::style::{Color, Style as RatatuiStyle};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::interactive_ratatui::ui::tuirealm_components::messages::AppMessage;

/// Text input component for tui-realm
pub struct TextInput {
    props: Props,
    /// The text content
    text: String,
    /// Current cursor position (in characters, not bytes)
    cursor_position: usize,
}

impl Default for TextInput {
    fn default() -> Self {
        Self {
            props: Props::default(),
            text: String::new(),
            cursor_position: 0,
        }
    }
}

impl TextInput {
    pub fn new(initial_text: String) -> Self {
        let cursor_pos = initial_text.chars().count();
        Self {
            props: Props::default(),
            text: initial_text,
            cursor_position: cursor_pos,
        }
    }

    /// Get the current text
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Get the current cursor position
    pub fn cursor_position(&self) -> usize {
        self.cursor_position
    }

    /// Set the text and move cursor to the end
    pub fn set_text(&mut self, text: String) {
        self.cursor_position = text.chars().count();
        self.text = text;
    }

    /// Find the previous word boundary from the given position
    fn find_prev_word_boundary(&self, from: usize) -> usize {
        let chars: Vec<char> = self.text.chars().collect();
        let mut pos = from;

        // Skip whitespace backwards
        while pos > 0 && chars.get(pos - 1).is_some_and(|c| c.is_whitespace()) {
            pos -= 1;
        }

        // Skip non-whitespace backwards
        while pos > 0 && chars.get(pos - 1).is_some_and(|c| !c.is_whitespace()) {
            pos -= 1;
        }

        pos
    }

    /// Find the next word boundary from the given position
    fn find_next_word_boundary(&self, from: usize) -> usize {
        let chars: Vec<char> = self.text.chars().collect();
        let mut pos = from;
        let len = chars.len();

        // Skip non-whitespace forwards
        while pos < len && chars.get(pos).is_some_and(|c| !c.is_whitespace()) {
            pos += 1;
        }

        // Skip whitespace forwards
        while pos < len && chars.get(pos).is_some_and(|c| c.is_whitespace()) {
            pos += 1;
        }

        pos
    }

    /// Delete from start position to end position
    fn delete_range(&mut self, start: usize, end: usize) -> bool {
        if start >= end || end > self.text.chars().count() {
            return false;
        }

        let byte_start = self
            .text
            .chars()
            .take(start)
            .map(|c| c.len_utf8())
            .sum::<usize>();
        let byte_end = self
            .text
            .chars()
            .take(end)
            .map(|c| c.len_utf8())
            .sum::<usize>();

        self.text.drain(byte_start..byte_end);
        self.cursor_position = start;
        true
    }

    /// Render the text with cursor as styled spans
    pub fn render_cursor_spans(&self) -> Vec<Span> {
        if self.text.is_empty() {
            // Show cursor on empty space
            vec![Span::styled(
                " ",
                RatatuiStyle::default().bg(Color::White).fg(Color::Black),
            )]
        } else if self.cursor_position < self.text.chars().count() {
            // Cursor is in the middle of text
            let (before, after) = self
                .text
                .chars()
                .enumerate()
                .partition::<Vec<_>, _>(|(i, _)| *i < self.cursor_position);

            let before: String = before.into_iter().map(|(_, c)| c).collect();
            let after: String = after.into_iter().map(|(_, c)| c).collect();

            let mut spans = Vec::new();

            // Only add before span if it's not empty
            if !before.is_empty() {
                spans.push(Span::raw(before));
            }

            // Add cursor span
            spans.push(Span::styled(
                after.chars().next().unwrap_or(' ').to_string(),
                RatatuiStyle::default().bg(Color::White).fg(Color::Black),
            ));

            // Add remaining text if any
            let remaining = after.chars().skip(1).collect::<String>();
            if !remaining.is_empty() {
                spans.push(Span::raw(remaining));
            }

            spans
        } else {
            // Cursor is at the end
            vec![
                Span::raw(self.text.clone()),
                Span::styled(" ", RatatuiStyle::default().bg(Color::White).fg(Color::Black)),
            ]
        }
    }
}

impl MockComponent for TextInput {
    fn view(&mut self, frame: &mut Frame, area: Rect) {
        let title = self
            .props
            .get_or(Attribute::Title, AttrValue::Title((String::new(), Alignment::Left)))
            .unwrap_title()
            .0;

        let spans = self.render_cursor_spans();
        let paragraph = Paragraph::new(Line::from(spans))
            .block(
                Block::default()
                    .title(title)
                    .borders(Borders::ALL)
                    .border_style(RatatuiStyle::default().fg(Color::Yellow)),
            )
            .style(RatatuiStyle::default().fg(Color::Yellow));

        frame.render_widget(paragraph, area);
    }

    fn query(&self, attr: Attribute) -> Option<AttrValue> {
        self.props.get(attr)
    }

    fn attr(&mut self, attr: Attribute, value: AttrValue) {
        self.props.set(attr, value);
    }

    fn state(&self) -> State {
        State::One(StateValue::String(self.text.clone()))
    }

    fn perform(&mut self, cmd: Cmd) -> CmdResult {
        match cmd {
            Cmd::Type(c) => {
                let char_pos = self.cursor_position;
                let byte_pos = self
                    .text
                    .chars()
                    .take(char_pos)
                    .map(|ch| ch.len_utf8())
                    .sum::<usize>();

                self.text.insert(byte_pos, c);
                self.cursor_position += 1;
                CmdResult::Changed(self.state())
            }
            Cmd::Move(Direction::Left) => {
                if self.cursor_position > 0 {
                    self.cursor_position -= 1;
                }
                CmdResult::None
            }
            Cmd::Move(Direction::Right) => {
                if self.cursor_position < self.text.chars().count() {
                    self.cursor_position += 1;
                }
                CmdResult::None
            }
            Cmd::GoTo(Position::Begin) => {
                self.cursor_position = 0;
                CmdResult::None
            }
            Cmd::GoTo(Position::End) => {
                self.cursor_position = self.text.chars().count();
                CmdResult::None
            }
            Cmd::Cancel => {
                self.text.clear();
                self.cursor_position = 0;
                CmdResult::Changed(self.state())
            }
            _ => CmdResult::None,
        }
    }
}

impl Component<AppMessage, NoUserEvent> for TextInput {
    fn on(&mut self, ev: Event<NoUserEvent>) -> Option<AppMessage> {
        match ev {
            Event::Keyboard(KeyEvent {
                code: Key::Backspace,
                ..
            }) => {
                if self.cursor_position > 0 {
                    let char_pos = self.cursor_position - 1;
                    let byte_start = self
                        .text
                        .chars()
                        .take(char_pos)
                        .map(|c| c.len_utf8())
                        .sum::<usize>();
                    let ch = self.text.chars().nth(char_pos).unwrap();
                    let byte_end = byte_start + ch.len_utf8();

                    self.text.drain(byte_start..byte_end);
                    self.cursor_position -= 1;
                    
                    // Return appropriate message based on component usage
                    if let Some(AttrValue::String(id)) = self.props.get(Attribute::Custom("id")) {
                        match id.as_str() {
                            "search_bar" => Some(AppMessage::QueryChanged(self.text.clone())),
                            "session_search" => Some(AppMessage::SessionQueryChanged(self.text.clone())),
                            _ => None,
                        }
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            Event::Keyboard(KeyEvent {
                code: Key::Delete,
                ..
            }) => {
                if self.cursor_position < self.text.chars().count() {
                    let byte_start = self
                        .text
                        .chars()
                        .take(self.cursor_position)
                        .map(|c| c.len_utf8())
                        .sum::<usize>();
                    let ch = self.text.chars().nth(self.cursor_position).unwrap();
                    let byte_end = byte_start + ch.len_utf8();

                    self.text.drain(byte_start..byte_end);
                    
                    // Return appropriate message
                    if let Some(AttrValue::String(id)) = self.props.get(Attribute::Custom("id")) {
                        match id.as_str() {
                            "search_bar" => Some(AppMessage::QueryChanged(self.text.clone())),
                            "session_search" => Some(AppMessage::SessionQueryChanged(self.text.clone())),
                            _ => None,
                        }
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            Event::Keyboard(KeyEvent {
                code: Key::Left,
                ..
            }) => {
                if self.cursor_position > 0 {
                    self.cursor_position -= 1;
                }
                None
            }
            Event::Keyboard(KeyEvent {
                code: Key::Right,
                ..
            }) => {
                if self.cursor_position < self.text.chars().count() {
                    self.cursor_position += 1;
                }
                None
            }
            Event::Keyboard(KeyEvent {
                code: Key::Home,
                ..
            }) => {
                self.cursor_position = 0;
                None
            }
            Event::Keyboard(KeyEvent {
                code: Key::End,
                ..
            }) => {
                self.cursor_position = self.text.chars().count();
                None
            }
            Event::Keyboard(KeyEvent {
                code: Key::Char('a'),
                modifiers: KeyModifiers::CONTROL,
            }) => {
                self.cursor_position = 0;
                None
            }
            Event::Keyboard(KeyEvent {
                code: Key::Char('e'),
                modifiers: KeyModifiers::CONTROL,
            }) => {
                self.cursor_position = self.text.chars().count();
                None
            }
            Event::Keyboard(KeyEvent {
                code: Key::Char('b'),
                modifiers: KeyModifiers::CONTROL,
            }) => {
                if self.cursor_position > 0 {
                    self.cursor_position -= 1;
                }
                None
            }
            Event::Keyboard(KeyEvent {
                code: Key::Char('f'),
                modifiers: KeyModifiers::CONTROL,
            }) => {
                if self.cursor_position < self.text.chars().count() {
                    self.cursor_position += 1;
                }
                None
            }
            Event::Keyboard(KeyEvent {
                code: Key::Char('h'),
                modifiers: KeyModifiers::CONTROL,
            }) => {
                // Same as backspace
                if self.cursor_position > 0 {
                    let char_pos = self.cursor_position - 1;
                    let byte_start = self
                        .text
                        .chars()
                        .take(char_pos)
                        .map(|c| c.len_utf8())
                        .sum::<usize>();
                    let ch = self.text.chars().nth(char_pos).unwrap();
                    let byte_end = byte_start + ch.len_utf8();

                    self.text.drain(byte_start..byte_end);
                    self.cursor_position -= 1;
                    
                    // Return appropriate message
                    if let Some(AttrValue::String(id)) = self.props.get(Attribute::Custom("id")) {
                        match id.as_str() {
                            "search_bar" => Some(AppMessage::QueryChanged(self.text.clone())),
                            "session_search" => Some(AppMessage::SessionQueryChanged(self.text.clone())),
                            _ => None,
                        }
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            Event::Keyboard(KeyEvent {
                code: Key::Char('d'),
                modifiers: KeyModifiers::CONTROL,
            }) => {
                // Delete character under cursor
                if self.cursor_position < self.text.chars().count() {
                    let byte_start = self
                        .text
                        .chars()
                        .take(self.cursor_position)
                        .map(|c| c.len_utf8())
                        .sum::<usize>();
                    let ch = self.text.chars().nth(self.cursor_position).unwrap();
                    let byte_end = byte_start + ch.len_utf8();

                    self.text.drain(byte_start..byte_end);
                    
                    // Return appropriate message
                    if let Some(AttrValue::String(id)) = self.props.get(Attribute::Custom("id")) {
                        match id.as_str() {
                            "search_bar" => Some(AppMessage::QueryChanged(self.text.clone())),
                            "session_search" => Some(AppMessage::SessionQueryChanged(self.text.clone())),
                            _ => None,
                        }
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            Event::Keyboard(KeyEvent {
                code: Key::Char('w'),
                modifiers: KeyModifiers::CONTROL,
            }) => {
                // Delete word before cursor
                if self.cursor_position > 0 {
                    let new_pos = self.find_prev_word_boundary(self.cursor_position);
                    let changed = self.delete_range(new_pos, self.cursor_position);
                    
                    if changed {
                        // Return appropriate message
                        if let Some(AttrValue::String(id)) = self.props.get(Attribute::Custom("id")) {
                            match id.as_str() {
                                "search_bar" => Some(AppMessage::QueryChanged(self.text.clone())),
                                "session_search" => Some(AppMessage::SessionQueryChanged(self.text.clone())),
                                _ => None,
                            }
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            Event::Keyboard(KeyEvent {
                code: Key::Char('u'),
                modifiers: KeyModifiers::CONTROL,
            }) => {
                // Delete from cursor to beginning of line
                if self.cursor_position > 0 {
                    let changed = self.delete_range(0, self.cursor_position);
                    
                    if changed {
                        // Return appropriate message
                        if let Some(AttrValue::String(id)) = self.props.get(Attribute::Custom("id")) {
                            match id.as_str() {
                                "search_bar" => Some(AppMessage::QueryChanged(self.text.clone())),
                                "session_search" => Some(AppMessage::SessionQueryChanged(self.text.clone())),
                                _ => None,
                            }
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            Event::Keyboard(KeyEvent {
                code: Key::Char('k'),
                modifiers: KeyModifiers::CONTROL,
            }) => {
                // Delete from cursor to end of line
                let len = self.text.chars().count();
                if self.cursor_position < len {
                    let changed = self.delete_range(self.cursor_position, len);
                    
                    if changed {
                        // Return appropriate message
                        if let Some(AttrValue::String(id)) = self.props.get(Attribute::Custom("id")) {
                            match id.as_str() {
                                "search_bar" => Some(AppMessage::QueryChanged(self.text.clone())),
                                "session_search" => Some(AppMessage::SessionQueryChanged(self.text.clone())),
                                _ => None,
                            }
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            Event::Keyboard(KeyEvent {
                code: Key::Char('b'),
                modifiers: KeyModifiers::ALT,
            }) => {
                self.cursor_position = self.find_prev_word_boundary(self.cursor_position);
                None
            }
            Event::Keyboard(KeyEvent {
                code: Key::Char('f'),
                modifiers: KeyModifiers::ALT,
            }) => {
                self.cursor_position = self.find_next_word_boundary(self.cursor_position);
                None
            }
            Event::Keyboard(KeyEvent {
                code: Key::Char(c),
                modifiers,
            }) => {
                // Skip if it was a control character we already handled
                if modifiers.contains(KeyModifiers::CONTROL) || modifiers.contains(KeyModifiers::ALT)
                {
                    return None;
                }

                let char_pos = self.cursor_position;
                let byte_pos = self
                    .text
                    .chars()
                    .take(char_pos)
                    .map(|c| c.len_utf8())
                    .sum::<usize>();

                self.text.insert(byte_pos, c);
                self.cursor_position += 1;
                
                // Return appropriate message
                if let Some(AttrValue::String(id)) = self.props.get(Attribute::Custom("id")) {
                    match id.as_str() {
                        "search_bar" => Some(AppMessage::QueryChanged(self.text.clone())),
                        "session_search" => Some(AppMessage::SessionQueryChanged(self.text.clone())),
                        _ => None,
                    }
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}