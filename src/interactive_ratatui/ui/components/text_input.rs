use crate::interactive_ratatui::ui::debug_log::{init_debug_log, write_debug_log};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    style::{Color, Style},
    text::Span,
};

/// A reusable text input component that handles cursor positioning and text editing
#[derive(Debug, Clone, Default)]
pub struct TextInput {
    text: String,
    cursor_position: usize,
}

impl TextInput {
    /// Create a new TextInput
    pub fn new() -> Self {
        Self {
            text: String::new(),
            cursor_position: 0,
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
        init_debug_log();
        write_debug_log(&format!(
            "[TextInput] set_text called: new_text='{}', old_text='{}', old_cursor_pos={}",
            text, self.text, self.cursor_position
        ));
        self.cursor_position = text.chars().count();
        self.text = text;
        write_debug_log(&format!(
            "[TextInput] After set_text: text='{}', cursor_pos={}",
            self.text, self.cursor_position
        ));
    }

    /// Set the cursor position
    pub fn set_cursor_position(&mut self, position: usize) {
        self.cursor_position = position.min(self.text.chars().count());
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

    /// Delete from start position to end position and return if text changed
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
    pub fn render_cursor_spans(&self) -> Vec<Span<'_>> {
        if self.text.is_empty() {
            // Show cursor on empty space
            vec![Span::styled(
                " ",
                Style::default().bg(Color::White).fg(Color::Black),
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
                Style::default().bg(Color::White).fg(Color::Black),
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
                Span::styled(" ", Style::default().bg(Color::White).fg(Color::Black)),
            ]
        }
    }

    /// Handle a key event and return true if the text changed
    pub fn handle_key(&mut self, key: KeyEvent) -> bool {
        init_debug_log();
        write_debug_log(&format!(
            "[TextInput] handle_key: key={:?}, text='{}', cursor_pos={}",
            key, self.text, self.cursor_position
        ));
        // Handle Control key combinations
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            match key.code {
                KeyCode::Char('a') => {
                    self.cursor_position = 0;
                    return false;
                }
                KeyCode::Char('e') => {
                    self.cursor_position = self.text.chars().count();
                    return false;
                }
                KeyCode::Char('b') => {
                    if self.cursor_position > 0 {
                        self.cursor_position -= 1;
                    }
                    return false;
                }
                KeyCode::Char('f') => {
                    if self.cursor_position < self.text.chars().count() {
                        self.cursor_position += 1;
                    }
                    return false;
                }
                KeyCode::Char('h') => {
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
                        return true;
                    }
                    return false;
                }
                KeyCode::Char('d') => {
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
                        return true;
                    }
                    return false;
                }
                KeyCode::Char('w') => {
                    // Delete word before cursor
                    if self.cursor_position > 0 {
                        let new_pos = self.find_prev_word_boundary(self.cursor_position);
                        return self.delete_range(new_pos, self.cursor_position);
                    }
                    return false;
                }
                KeyCode::Char('u') => {
                    // Delete from cursor to beginning of line
                    if self.cursor_position > 0 {
                        return self.delete_range(0, self.cursor_position);
                    }
                    return false;
                }
                KeyCode::Char('k') => {
                    // Delete from cursor to end of line
                    let len = self.text.chars().count();
                    if self.cursor_position < len {
                        return self.delete_range(self.cursor_position, len);
                    }
                    return false;
                }
                _ => {}
            }
        }

        // Handle Alt key combinations
        if key.modifiers.contains(KeyModifiers::ALT) {
            match key.code {
                KeyCode::Char('b') => {
                    self.cursor_position = self.find_prev_word_boundary(self.cursor_position);
                    return false;
                }
                KeyCode::Char('f') => {
                    self.cursor_position = self.find_next_word_boundary(self.cursor_position);
                    return false;
                }
                _ => {}
            }
        }

        match key.code {
            KeyCode::Char(c) => {
                // Skip if it was a control character we already handled
                if key.modifiers.contains(KeyModifiers::CONTROL)
                    || key.modifiers.contains(KeyModifiers::ALT)
                {
                    return false;
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
                true
            }
            KeyCode::Backspace => {
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
                    true
                } else {
                    false
                }
            }
            KeyCode::Delete => {
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
                    true
                } else {
                    false
                }
            }
            KeyCode::Left => {
                write_debug_log(&format!(
                    "[TextInput] Left arrow pressed, cursor_position={}",
                    self.cursor_position
                ));
                if self.cursor_position > 0 {
                    self.cursor_position -= 1;
                    write_debug_log(&format!(
                        "[TextInput] Moved cursor left to {}",
                        self.cursor_position
                    ));
                } else {
                    write_debug_log("[TextInput] Already at beginning");
                }
                false
            }
            KeyCode::Right => {
                write_debug_log(&format!(
                    "[TextInput] Right arrow pressed, cursor_position={}, text_len={}",
                    self.cursor_position,
                    self.text.chars().count()
                ));
                if self.cursor_position < self.text.chars().count() {
                    self.cursor_position += 1;
                    write_debug_log(&format!(
                        "[TextInput] Moved cursor right to {}",
                        self.cursor_position
                    ));
                } else {
                    write_debug_log("[TextInput] Already at end");
                }
                false
            }
            KeyCode::Home => {
                self.cursor_position = 0;
                false
            }
            KeyCode::End => {
                self.cursor_position = self.text.chars().count();
                false
            }
            _ => false,
        }
    }
}
