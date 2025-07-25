//! Advanced text input component with full editing capabilities
//! Supports cursor movement, word boundaries, and various editing shortcuts

use crate::interactive_iocraft::ui::contexts::Theme;
use crate::interactive_iocraft::ui::hooks::use_terminal_events;
use iocraft::prelude::*;
use futures::StreamExt;

/// Advanced text input component state
#[derive(Debug, Clone)]
pub struct TextInputState {
    text: String,
    cursor_position: usize,
}

impl Default for TextInputState {
    fn default() -> Self {
        Self {
            text: String::new(),
            cursor_position: 0,
        }
    }
}

impl TextInputState {
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

    /// Handle a key event and return true if the text changed
    pub fn handle_key(&mut self, key: KeyEvent) -> bool {
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
                        let ch = match self.text.chars().nth(char_pos) {
                            Some(c) => c,
                            None => return false,
                        };
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
                        let ch = match self.text.chars().nth(self.cursor_position) {
                            Some(c) => c,
                            None => return false,
                        };
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
                    let ch = match self.text.chars().nth(char_pos) {
                        Some(c) => c,
                        None => return false,
                    };
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
                    let ch = match self.text.chars().nth(self.cursor_position) {
                        Some(c) => c,
                        None => return false,
                    };
                    let byte_end = byte_start + ch.len_utf8();

                    self.text.drain(byte_start..byte_end);
                    true
                } else {
                    false
                }
            }
            KeyCode::Left => {
                if self.cursor_position > 0 {
                    self.cursor_position -= 1;
                }
                false
            }
            KeyCode::Right => {
                if self.cursor_position < self.text.chars().count() {
                    self.cursor_position += 1;
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

#[derive(Props)]
pub struct AdvancedTextInputProps {
    pub value: String,
    pub on_change: Handler<'static, String>,
    pub has_focus: bool,
}

impl Default for AdvancedTextInputProps {
    fn default() -> Self {
        Self {
            value: String::new(),
            on_change: Handler::default(),
            has_focus: false,
        }
    }
}

/// Advanced text input component with full editing capabilities
#[component]
pub fn AdvancedTextInput(mut hooks: Hooks, props: &mut AdvancedTextInputProps) -> impl Into<AnyElement<'static>> {
    let _theme = hooks.use_context::<Theme>();
    let mut state = hooks.use_state(|| TextInputState::default());
    
    // Update internal state when props change
    {
        let state_read = state.read();
        if state_read.text() != &props.value {
            drop(state_read);
            state.write().set_text(props.value.clone());
        }
    }
    
    // Handle keyboard events when focused
    if props.has_focus {
        let mut events = use_terminal_events(&mut hooks);
        
        hooks.use_future({
            let mut state = state.clone();
            let mut on_change = props.on_change.take();
            
            async move {
                while let Some(event) = events.next().await {
                    if let TerminalEvent::Key(key) = event {
                        let changed = state.write().handle_key(key);
                        if changed {
                            let new_text = state.read().text().to_string();
                            on_change(new_text);
                        }
                    }
                }
            }
        });
    }
    
    // Render the text with cursor
    let (before_cursor, at_cursor, after_cursor) = {
        let state_read = state.read();
        let text = state_read.text();
        let cursor_pos = state_read.cursor_position();
        
        if text.is_empty() {
            (String::new(), ' ', String::new())
        } else if cursor_pos < text.chars().count() {
            let chars: Vec<char> = text.chars().collect();
            let before = chars[..cursor_pos].iter().collect::<String>();
            let at = chars[cursor_pos];
            let after = chars[cursor_pos + 1..].iter().collect::<String>();
            (before, at, after)
        } else {
            (text.to_string(), ' ', String::new())
        }
    };
    
    element! {
        Box(flex_direction: FlexDirection::Row) {
            // Text before cursor
            #(if !before_cursor.is_empty() {
                element! {
                    Text(
                        content: before_cursor,
                        color: Color::Reset,
                    )
                }.into_any()
            } else {
                element! { Box() }.into_any()
            })
            
            // Cursor
            Box(
                background_color: Color::White,
            ) {
                Text(
                    content: at_cursor.to_string(),
                    color: Color::Black,
                    weight: Weight::Bold,
                )
            }
            
            // Text after cursor
            #(if !after_cursor.is_empty() {
                element! {
                    Text(
                        content: after_cursor,
                        color: Color::Reset,
                    )
                }.into_any()
            } else {
                element! { Box() }.into_any()
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_text_input() {
        let mut state = TextInputState::default();
        
        // Test initial state
        assert_eq!(state.text(), "");
        assert_eq!(state.cursor_position(), 0);
        
        // Test setting text
        state.set_text("hello world".to_string());
        assert_eq!(state.text(), "hello world");
        assert_eq!(state.cursor_position(), 11);
        
        // Test cursor position setting
        state.set_cursor_position(5);
        assert_eq!(state.cursor_position(), 5);
    }

    #[test]
    fn test_word_boundaries() {
        let mut state = TextInputState::default();
        state.set_text("hello world test".to_string());
        state.set_cursor_position(13); // After "world t"
        
        // Test previous word boundary
        let prev = state.find_prev_word_boundary(13);
        assert_eq!(prev, 12); // Start of "test"
        
        // Test next word boundary from middle of word
        state.set_cursor_position(7); // In "world"
        let next = state.find_next_word_boundary(7);
        assert_eq!(next, 12); // Start of "test"
    }

    #[test]
    fn test_unicode_text() {
        let mut state = TextInputState::default();
        
        // Test with multibyte text
        state.set_text("こんにちは".to_string());
        assert_eq!(state.text(), "こんにちは");
        assert_eq!(state.cursor_position(), 5); // 5 characters
        
        // Test cursor positioning
        state.set_cursor_position(2);
        assert_eq!(state.cursor_position(), 2);
    }

    #[test]
    fn test_insert_char() {
        let mut state = TextInputState::default();
        
        // Insert at beginning
        let key = KeyEvent {
            code: KeyCode::Char('a'),
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            ..Default::default()
        };
        assert!(state.handle_key(key));
        assert_eq!(state.text(), "a");
        assert_eq!(state.cursor_position(), 1);
        
        // Insert at end
        let key = KeyEvent {
            code: KeyCode::Char('b'),
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            ..Default::default()
        };
        assert!(state.handle_key(key));
        assert_eq!(state.text(), "ab");
        assert_eq!(state.cursor_position(), 2);
        
        // Insert in middle
        state.set_cursor_position(1);
        let key = KeyEvent {
            code: KeyCode::Char('x'),
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            ..Default::default()
        };
        assert!(state.handle_key(key));
        assert_eq!(state.text(), "axb");
        assert_eq!(state.cursor_position(), 2);
    }

    #[test]
    fn test_move_cursor() {
        let mut state = TextInputState::default();
        state.set_text("hello".to_string());
        
        // Test move left
        let key = KeyEvent {
            code: KeyCode::Left,
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            ..Default::default()
        };
        assert!(!state.handle_key(key)); // Movement doesn't change text
        assert_eq!(state.cursor_position(), 4);
        
        // Test move right
        let key = KeyEvent {
            code: KeyCode::Right,
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            ..Default::default()
        };
        assert!(!state.handle_key(key));
        assert_eq!(state.cursor_position(), 5);
        
        // Test boundaries
        assert!(!state.handle_key(key)); // Should not move past end
        assert_eq!(state.cursor_position(), 5);
        
        state.set_cursor_position(0);
        let key = KeyEvent {
            code: KeyCode::Left,
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            ..Default::default()
        };
        assert!(!state.handle_key(key)); // Should not move before start
        assert_eq!(state.cursor_position(), 0);
    }

    #[test]
    fn test_delete_operations() {
        let mut state = TextInputState::default();
        state.set_text("hello".to_string());
        
        // Test backspace key
        let backspace = KeyEvent {
            code: KeyCode::Backspace,
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            ..Default::default()
        };
        assert!(state.handle_key(backspace));
        assert_eq!(state.text(), "hell");
        assert_eq!(state.cursor_position(), 4);
        
        // Test delete char with Ctrl+D
        state.set_cursor_position(2);
        let delete = KeyEvent {
            code: KeyCode::Char('d'),
            modifiers: KeyModifiers::CONTROL,
            kind: KeyEventKind::Press,
            ..Default::default()
        };
        assert!(state.handle_key(delete));
        assert_eq!(state.text(), "hel");
        assert_eq!(state.cursor_position(), 2);
        
        // Test delete at boundaries
        state.set_cursor_position(0);
        let backspace = KeyEvent {
            code: KeyCode::Backspace,
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            ..Default::default()
        };
        assert!(!state.handle_key(backspace)); // Can't backspace at start
        
        state.set_cursor_position(3);
        let delete = KeyEvent {
            code: KeyCode::Char('d'),
            modifiers: KeyModifiers::CONTROL,
            kind: KeyEventKind::Press,
            ..Default::default()
        };
        assert!(!state.handle_key(delete)); // Can't delete at end
    }

    #[test]
    fn test_handle_key_input() {
        let mut state = TextInputState::default();
        
        // Test character input
        let key = KeyEvent {
            code: KeyCode::Char('h'),
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            ..Default::default()
        };
        assert!(state.handle_key(key));
        assert_eq!(state.text(), "h");
        
        // Test backspace
        let key = KeyEvent {
            code: KeyCode::Backspace,
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };
        assert!(state.handle_key(key));
        assert_eq!(state.text(), "");
    }

    #[test]
    fn test_handle_key_movement() {
        let mut state = TextInputState::default();
        state.set_text("hello world".to_string());
        
        // Test left arrow
        let key = KeyEvent {
            code: KeyCode::Left,
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            ..Default::default()
        };
        assert!(!state.handle_key(key)); // Movement doesn't change text
        assert_eq!(state.cursor_position(), 10);
        
        // Test right arrow
        let key = KeyEvent {
            code: KeyCode::Right,
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            ..Default::default()
        };
        assert!(!state.handle_key(key));
        assert_eq!(state.cursor_position(), 11);
        
        // Test home
        let key = KeyEvent {
            code: KeyCode::Home,
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            ..Default::default()
        };
        assert!(!state.handle_key(key));
        assert_eq!(state.cursor_position(), 0);
        
        // Test end
        let key = KeyEvent {
            code: KeyCode::End,
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            ..Default::default()
        };
        assert!(!state.handle_key(key));
        assert_eq!(state.cursor_position(), 11);
    }

    #[test]
    fn test_handle_key_ctrl_shortcuts() {
        let mut state = TextInputState::default();
        state.set_text("hello world test".to_string());
        
        // Test Ctrl+A (beginning)
        let key = KeyEvent {
            code: KeyCode::Char('a'),
            modifiers: KeyModifiers::CONTROL,
            kind: KeyEventKind::Press,
            ..Default::default()
        };
        assert!(!state.handle_key(key));
        assert_eq!(state.cursor_position(), 0);
        
        // Test Ctrl+E (end)
        let key = KeyEvent {
            code: KeyCode::Char('e'),
            modifiers: KeyModifiers::CONTROL,
            kind: KeyEventKind::Press,
            ..Default::default()
        };
        assert!(!state.handle_key(key));
        assert_eq!(state.cursor_position(), 16);
        
        // Test Ctrl+W (delete word)
        let key = KeyEvent {
            code: KeyCode::Char('w'),
            modifiers: KeyModifiers::CONTROL,
            kind: KeyEventKind::Press,
            ..Default::default()
        };
        assert!(state.handle_key(key));
        assert_eq!(state.text(), "hello world ");
        
        // Test Ctrl+U (delete to beginning)
        state.set_cursor_position(6);
        let key = KeyEvent {
            code: KeyCode::Char('u'),
            modifiers: KeyModifiers::CONTROL,
            kind: KeyEventKind::Press,
            ..Default::default()
        };
        assert!(state.handle_key(key));
        assert_eq!(state.text(), "world ");
        
        // Test Ctrl+K (delete to end)
        state.set_text("hello world".to_string());
        state.set_cursor_position(5);
        let key = KeyEvent {
            code: KeyCode::Char('k'),
            modifiers: KeyModifiers::CONTROL,
            kind: KeyEventKind::Press,
            ..Default::default()
        };
        assert!(state.handle_key(key));
        assert_eq!(state.text(), "hello");
    }

    #[test]
    fn test_handle_key_alt_shortcuts() {
        let mut state = TextInputState::default();
        state.set_text("hello world test".to_string());
        state.set_cursor_position(16); // End
        
        // Test Alt+B (word left)
        let key = KeyEvent {
            code: KeyCode::Char('b'),
            modifiers: KeyModifiers::ALT,
            kind: KeyEventKind::Press,
            ..Default::default()
        };
        assert!(!state.handle_key(key));
        assert_eq!(state.cursor_position(), 12); // Beginning of "test"
        
        // Test Alt+F (word right)
        state.set_cursor_position(0);
        let key = KeyEvent {
            code: KeyCode::Char('f'),
            modifiers: KeyModifiers::ALT,
            kind: KeyEventKind::Press,
            ..Default::default()
        };
        assert!(!state.handle_key(key));
        assert_eq!(state.cursor_position(), 5); // After "hello"
    }


}