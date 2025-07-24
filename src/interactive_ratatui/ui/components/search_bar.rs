use crate::interactive_ratatui::ui::components::Component;
use crate::interactive_ratatui::ui::events::Message;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

#[derive(Default)]
pub struct SearchBar {
    query: String,
    cursor_position: usize,
    is_searching: bool,
    message: Option<String>,
    role_filter: Option<String>,
}

impl SearchBar {
    pub fn new() -> Self {
        Self {
            query: String::new(),
            cursor_position: 0,
            is_searching: false,
            message: None,
            role_filter: None,
        }
    }

    pub fn set_query(&mut self, query: String) {
        self.query = query;
        self.cursor_position = self.query.chars().count();
    }

    pub fn set_searching(&mut self, is_searching: bool) {
        self.is_searching = is_searching;
    }

    pub fn set_message(&mut self, message: Option<String>) {
        self.message = message;
    }

    pub fn set_role_filter(&mut self, role_filter: Option<String>) {
        self.role_filter = role_filter;
    }

    #[allow(dead_code)]
    pub fn get_query(&self) -> &str {
        &self.query
    }

    #[allow(dead_code)]
    pub fn is_searching(&self) -> bool {
        self.is_searching
    }

    /// Find the previous word boundary from the given position
    fn find_prev_word_boundary(&self, from: usize) -> usize {
        let chars: Vec<char> = self.query.chars().collect();
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
        let chars: Vec<char> = self.query.chars().collect();
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

    /// Delete from start position to end position and return if query changed
    fn delete_range(&mut self, start: usize, end: usize) -> bool {
        if start >= end || end > self.query.chars().count() {
            return false;
        }

        let byte_start = self
            .query
            .chars()
            .take(start)
            .map(|c| c.len_utf8())
            .sum::<usize>();
        let byte_end = self
            .query
            .chars()
            .take(end)
            .map(|c| c.len_utf8())
            .sum::<usize>();

        self.query.drain(byte_start..byte_end);
        self.cursor_position = start;
        true
    }
}

impl Component for SearchBar {
    fn render(&mut self, f: &mut Frame, area: Rect) {
        let input_text = if self.cursor_position < self.query.chars().count() {
            let (before, after) = self
                .query
                .chars()
                .enumerate()
                .partition::<Vec<_>, _>(|(i, _)| *i < self.cursor_position);

            let before: String = before.into_iter().map(|(_, c)| c).collect();
            let after: String = after.into_iter().map(|(_, c)| c).collect();

            vec![
                Span::raw(before),
                Span::styled(
                    after.chars().next().unwrap_or(' ').to_string(),
                    Style::default().bg(Color::White).fg(Color::Black),
                ),
                Span::raw(after.chars().skip(1).collect::<String>()),
            ]
        } else {
            vec![
                Span::raw(&self.query),
                Span::styled(" ", Style::default().bg(Color::White).fg(Color::Black)),
            ]
        };

        let mut title = "Search".to_string();
        if let Some(role) = &self.role_filter {
            title.push_str(&format!(" [role:{role}]"));
        }
        if let Some(msg) = &self.message {
            title.push_str(&format!(" - {msg}"));
        }

        let input = Paragraph::new(Line::from(input_text))
            .block(Block::default().title(title).borders(Borders::ALL))
            .style(Style::default().fg(Color::Yellow));

        f.render_widget(input, area);
    }

    fn handle_key(&mut self, key: KeyEvent) -> Option<Message> {
        // Handle Control key combinations
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            match key.code {
                // Ctrl+A - Move cursor to beginning of line
                KeyCode::Char('a') => {
                    self.cursor_position = 0;
                    return None;
                }
                // Ctrl+E - Move cursor to end of line
                KeyCode::Char('e') => {
                    self.cursor_position = self.query.chars().count();
                    return None;
                }
                // Ctrl+B - Move cursor backward one character
                KeyCode::Char('b') => {
                    if self.cursor_position > 0 {
                        self.cursor_position -= 1;
                    }
                    return None;
                }
                // Ctrl+F - Move cursor forward one character
                KeyCode::Char('f') => {
                    if self.cursor_position < self.query.chars().count() {
                        self.cursor_position += 1;
                    }
                    return None;
                }
                // Ctrl+H - Delete character before cursor (same as backspace)
                KeyCode::Char('h') => {
                    if self.cursor_position > 0 {
                        let char_pos = self.cursor_position - 1;
                        let byte_start = self
                            .query
                            .chars()
                            .take(char_pos)
                            .map(|c| c.len_utf8())
                            .sum::<usize>();
                        let ch = self.query.chars().nth(char_pos).unwrap();
                        let byte_end = byte_start + ch.len_utf8();

                        self.query.drain(byte_start..byte_end);
                        self.cursor_position -= 1;
                        return Some(Message::QueryChanged(self.query.clone()));
                    }
                    return None;
                }
                // Ctrl+D - Delete character under cursor
                KeyCode::Char('d') => {
                    if self.cursor_position < self.query.chars().count() {
                        let byte_start = self
                            .query
                            .chars()
                            .take(self.cursor_position)
                            .map(|c| c.len_utf8())
                            .sum::<usize>();
                        let ch = self.query.chars().nth(self.cursor_position).unwrap();
                        let byte_end = byte_start + ch.len_utf8();

                        self.query.drain(byte_start..byte_end);
                        return Some(Message::QueryChanged(self.query.clone()));
                    }
                    return None;
                }
                // Ctrl+W - Delete word before cursor
                KeyCode::Char('w') => {
                    if self.cursor_position > 0 {
                        let new_pos = self.find_prev_word_boundary(self.cursor_position);
                        if self.delete_range(new_pos, self.cursor_position) {
                            return Some(Message::QueryChanged(self.query.clone()));
                        }
                    }
                    return None;
                }
                // Ctrl+U - Delete from cursor to beginning of line
                KeyCode::Char('u') => {
                    if self.cursor_position > 0 && self.delete_range(0, self.cursor_position) {
                        return Some(Message::QueryChanged(self.query.clone()));
                    }
                    return None;
                }
                // Ctrl+K - Delete from cursor to end of line
                KeyCode::Char('k') => {
                    let len = self.query.chars().count();
                    if self.cursor_position < len && self.delete_range(self.cursor_position, len) {
                        return Some(Message::QueryChanged(self.query.clone()));
                    }
                    return None;
                }
                _ => {}
            }
        }

        // Handle Alt key combinations
        if key.modifiers.contains(KeyModifiers::ALT) {
            match key.code {
                // Alt+B - Move cursor backward one word
                KeyCode::Char('b') => {
                    self.cursor_position = self.find_prev_word_boundary(self.cursor_position);
                    return None;
                }
                // Alt+F - Move cursor forward one word
                KeyCode::Char('f') => {
                    self.cursor_position = self.find_next_word_boundary(self.cursor_position);
                    return None;
                }
                _ => {}
            }
        }

        // Handle regular keys
        match key.code {
            KeyCode::Char(c) => {
                // Skip if it was a control character we already handled
                if key.modifiers.contains(KeyModifiers::CONTROL)
                    || key.modifiers.contains(KeyModifiers::ALT)
                {
                    return None;
                }

                let char_pos = self.cursor_position;
                let byte_pos = self
                    .query
                    .chars()
                    .take(char_pos)
                    .map(|c| c.len_utf8())
                    .sum::<usize>();

                self.query.insert(byte_pos, c);
                self.cursor_position += 1;
                Some(Message::QueryChanged(self.query.clone()))
            }
            KeyCode::Backspace => {
                if self.cursor_position > 0 {
                    let char_pos = self.cursor_position - 1;
                    let byte_start = self
                        .query
                        .chars()
                        .take(char_pos)
                        .map(|c| c.len_utf8())
                        .sum::<usize>();
                    let ch = self.query.chars().nth(char_pos).unwrap();
                    let byte_end = byte_start + ch.len_utf8();

                    self.query.drain(byte_start..byte_end);
                    self.cursor_position -= 1;
                    Some(Message::QueryChanged(self.query.clone()))
                } else {
                    None
                }
            }
            KeyCode::Delete => {
                if self.cursor_position < self.query.chars().count() {
                    let byte_start = self
                        .query
                        .chars()
                        .take(self.cursor_position)
                        .map(|c| c.len_utf8())
                        .sum::<usize>();
                    let ch = self.query.chars().nth(self.cursor_position).unwrap();
                    let byte_end = byte_start + ch.len_utf8();

                    self.query.drain(byte_start..byte_end);
                    Some(Message::QueryChanged(self.query.clone()))
                } else {
                    None
                }
            }
            KeyCode::Left => {
                if self.cursor_position > 0 {
                    self.cursor_position -= 1;
                }
                None
            }
            KeyCode::Right => {
                if self.cursor_position < self.query.chars().count() {
                    self.cursor_position += 1;
                }
                None
            }
            KeyCode::Home => {
                self.cursor_position = 0;
                None
            }
            KeyCode::End => {
                self.cursor_position = self.query.chars().count();
                None
            }
            _ => None,
        }
    }
}
