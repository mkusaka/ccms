#[cfg(test)]
mod tests {
    use super::super::text_input::TextInput;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    fn create_key_event(code: KeyCode) -> KeyEvent {
        KeyEvent {
            code,
            modifiers: KeyModifiers::empty(),
            kind: crossterm::event::KeyEventKind::Press,
            state: crossterm::event::KeyEventState::empty(),
        }
    }

    fn create_key_event_with_modifiers(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
        KeyEvent {
            code,
            modifiers,
            kind: crossterm::event::KeyEventKind::Press,
            state: crossterm::event::KeyEventState::empty(),
        }
    }

    #[test]
    fn test_text_input_creation() {
        let input = TextInput::new();
        assert_eq!(input.text(), "");
        assert_eq!(input.cursor_position(), 0);
    }

    #[test]
    fn test_set_text() {
        let mut input = TextInput::new();
        input.set_text("hello world".to_string());
        assert_eq!(input.text(), "hello world");
        assert_eq!(input.cursor_position(), 11); // cursor should move to end
    }

    #[test]
    fn test_character_input() {
        let mut input = TextInput::new();

        let changed = input.handle_key(create_key_event(KeyCode::Char('h')));
        assert!(changed);
        assert_eq!(input.text(), "h");
        assert_eq!(input.cursor_position(), 1);

        let changed = input.handle_key(create_key_event(KeyCode::Char('i')));
        assert!(changed);
        assert_eq!(input.text(), "hi");
        assert_eq!(input.cursor_position(), 2);
    }

    #[test]
    fn test_backspace() {
        let mut input = TextInput::new();
        input.set_text("hello".to_string());

        let changed = input.handle_key(create_key_event(KeyCode::Backspace));
        assert!(changed);
        assert_eq!(input.text(), "hell");
        assert_eq!(input.cursor_position(), 4);

        // Backspace at beginning should not change
        input.set_cursor_position(0);
        let changed = input.handle_key(create_key_event(KeyCode::Backspace));
        assert!(!changed);
        assert_eq!(input.text(), "hell");
    }

    #[test]
    fn test_delete() {
        let mut input = TextInput::new();
        input.set_text("hello".to_string());
        input.set_cursor_position(0);

        let changed = input.handle_key(create_key_event(KeyCode::Delete));
        assert!(changed);
        assert_eq!(input.text(), "ello");
        assert_eq!(input.cursor_position(), 0);

        // Delete at end should not change
        input.set_cursor_position(4);
        let changed = input.handle_key(create_key_event(KeyCode::Delete));
        assert!(!changed);
        assert_eq!(input.text(), "ello");
    }

    #[test]
    fn test_cursor_movement_left_right() {
        let mut input = TextInput::new();
        input.set_text("hello".to_string());

        // Move left
        let changed = input.handle_key(create_key_event(KeyCode::Left));
        assert!(!changed); // cursor movement doesn't change text
        assert_eq!(input.cursor_position(), 4);

        // Move right at end does nothing
        input.set_cursor_position(5);
        let changed = input.handle_key(create_key_event(KeyCode::Right));
        assert!(!changed);
        assert_eq!(input.cursor_position(), 5);

        // Move right from middle
        input.set_cursor_position(2);
        let changed = input.handle_key(create_key_event(KeyCode::Right));
        assert!(!changed);
        assert_eq!(input.cursor_position(), 3);
    }

    #[test]
    fn test_cursor_movement_home_end() {
        let mut input = TextInput::new();
        input.set_text("hello world".to_string());
        input.set_cursor_position(5);

        // Move to beginning with Home
        let changed = input.handle_key(create_key_event(KeyCode::Home));
        assert!(!changed);
        assert_eq!(input.cursor_position(), 0);

        // Move to end with End
        let changed = input.handle_key(create_key_event(KeyCode::End));
        assert!(!changed);
        assert_eq!(input.cursor_position(), 11);
    }

    #[test]
    fn test_ctrl_a_move_to_beginning() {
        let mut input = TextInput::new();
        input.set_text("hello world".to_string());

        let changed = input.handle_key(create_key_event_with_modifiers(
            KeyCode::Char('a'),
            KeyModifiers::CONTROL,
        ));
        assert!(!changed);
        assert_eq!(input.cursor_position(), 0);
    }

    #[test]
    fn test_ctrl_e_move_to_end() {
        let mut input = TextInput::new();
        input.set_text("hello world".to_string());
        input.set_cursor_position(0);

        let changed = input.handle_key(create_key_event_with_modifiers(
            KeyCode::Char('e'),
            KeyModifiers::CONTROL,
        ));
        assert!(!changed);
        assert_eq!(input.cursor_position(), 11);
    }

    #[test]
    fn test_ctrl_b_move_backward() {
        let mut input = TextInput::new();
        input.set_text("hello".to_string());

        let changed = input.handle_key(create_key_event_with_modifiers(
            KeyCode::Char('b'),
            KeyModifiers::CONTROL,
        ));
        assert!(!changed);
        assert_eq!(input.cursor_position(), 4);

        // At beginning, should not move
        input.set_cursor_position(0);
        let changed = input.handle_key(create_key_event_with_modifiers(
            KeyCode::Char('b'),
            KeyModifiers::CONTROL,
        ));
        assert!(!changed);
        assert_eq!(input.cursor_position(), 0);
    }

    #[test]
    fn test_ctrl_f_move_forward() {
        let mut input = TextInput::new();
        input.set_text("hello".to_string());
        input.set_cursor_position(0);

        let changed = input.handle_key(create_key_event_with_modifiers(
            KeyCode::Char('f'),
            KeyModifiers::CONTROL,
        ));
        assert!(!changed);
        assert_eq!(input.cursor_position(), 1);

        // At end, should not move
        input.set_cursor_position(5);
        let changed = input.handle_key(create_key_event_with_modifiers(
            KeyCode::Char('f'),
            KeyModifiers::CONTROL,
        ));
        assert!(!changed);
        assert_eq!(input.cursor_position(), 5);
    }

    #[test]
    fn test_alt_b_move_word_backward() {
        let mut input = TextInput::new();
        input.set_text("hello world test".to_string());

        // Move backward by word
        let changed = input.handle_key(create_key_event_with_modifiers(
            KeyCode::Char('b'),
            KeyModifiers::ALT,
        ));
        assert!(!changed);
        assert_eq!(input.cursor_position(), 12); // Beginning of "test"

        // Move backward again
        let changed = input.handle_key(create_key_event_with_modifiers(
            KeyCode::Char('b'),
            KeyModifiers::ALT,
        ));
        assert!(!changed);
        assert_eq!(input.cursor_position(), 6); // Beginning of "world"
    }

    #[test]
    fn test_alt_f_move_word_forward() {
        let mut input = TextInput::new();
        input.set_text("hello world test".to_string());
        input.set_cursor_position(0);

        // Move forward by word
        let changed = input.handle_key(create_key_event_with_modifiers(
            KeyCode::Char('f'),
            KeyModifiers::ALT,
        ));
        assert!(!changed);
        assert_eq!(input.cursor_position(), 6); // After "hello "

        // Move forward again
        let changed = input.handle_key(create_key_event_with_modifiers(
            KeyCode::Char('f'),
            KeyModifiers::ALT,
        ));
        assert!(!changed);
        assert_eq!(input.cursor_position(), 12); // After "world "
    }

    #[test]
    fn test_ctrl_h_delete_before_cursor() {
        let mut input = TextInput::new();
        input.set_text("hello".to_string());

        // Ctrl+H - Delete before cursor
        let changed = input.handle_key(create_key_event_with_modifiers(
            KeyCode::Char('h'),
            KeyModifiers::CONTROL,
        ));
        assert!(changed);
        assert_eq!(input.text(), "hell");
        assert_eq!(input.cursor_position(), 4);

        // At beginning, should do nothing
        input.set_cursor_position(0);
        let changed = input.handle_key(create_key_event_with_modifiers(
            KeyCode::Char('h'),
            KeyModifiers::CONTROL,
        ));
        assert!(!changed);
        assert_eq!(input.text(), "hell");
    }

    #[test]
    fn test_ctrl_d_delete_under_cursor() {
        let mut input = TextInput::new();
        input.set_text("hello".to_string());
        input.set_cursor_position(0);

        // Ctrl+D - Delete under cursor
        let changed = input.handle_key(create_key_event_with_modifiers(
            KeyCode::Char('d'),
            KeyModifiers::CONTROL,
        ));
        assert!(changed);
        assert_eq!(input.text(), "ello");
        assert_eq!(input.cursor_position(), 0);

        // At end, should do nothing
        input.set_cursor_position(4);
        let changed = input.handle_key(create_key_event_with_modifiers(
            KeyCode::Char('d'),
            KeyModifiers::CONTROL,
        ));
        assert!(!changed);
        assert_eq!(input.text(), "ello");
    }

    #[test]
    fn test_ctrl_w_delete_word_before_cursor() {
        let mut input = TextInput::new();
        input.set_text("hello world test".to_string());

        // Delete word before cursor
        let changed = input.handle_key(create_key_event_with_modifiers(
            KeyCode::Char('w'),
            KeyModifiers::CONTROL,
        ));
        assert!(changed);
        assert_eq!(input.text(), "hello world ");
        assert_eq!(input.cursor_position(), 12);

        // Delete another word
        let changed = input.handle_key(create_key_event_with_modifiers(
            KeyCode::Char('w'),
            KeyModifiers::CONTROL,
        ));
        assert!(changed);
        assert_eq!(input.text(), "hello ");
        assert_eq!(input.cursor_position(), 6);

        // At beginning, should do nothing
        input.set_cursor_position(0);
        let changed = input.handle_key(create_key_event_with_modifiers(
            KeyCode::Char('w'),
            KeyModifiers::CONTROL,
        ));
        assert!(!changed);
    }

    #[test]
    fn test_ctrl_u_delete_to_beginning() {
        let mut input = TextInput::new();
        input.set_text("hello world".to_string());
        input.set_cursor_position(6);

        // Delete to beginning
        let changed = input.handle_key(create_key_event_with_modifiers(
            KeyCode::Char('u'),
            KeyModifiers::CONTROL,
        ));
        assert!(changed);
        assert_eq!(input.text(), "world");
        assert_eq!(input.cursor_position(), 0);

        // At beginning, should do nothing
        let changed = input.handle_key(create_key_event_with_modifiers(
            KeyCode::Char('u'),
            KeyModifiers::CONTROL,
        ));
        assert!(!changed);
    }

    #[test]
    fn test_ctrl_k_delete_to_end() {
        let mut input = TextInput::new();
        input.set_text("hello world".to_string());
        input.set_cursor_position(6);

        // Delete to end
        let changed = input.handle_key(create_key_event_with_modifiers(
            KeyCode::Char('k'),
            KeyModifiers::CONTROL,
        ));
        assert!(changed);
        assert_eq!(input.text(), "hello ");
        assert_eq!(input.cursor_position(), 6);

        // At end, should do nothing
        let changed = input.handle_key(create_key_event_with_modifiers(
            KeyCode::Char('k'),
            KeyModifiers::CONTROL,
        ));
        assert!(!changed);
    }

    #[test]
    fn test_unicode_handling() {
        let mut input = TextInput::new();
        input.set_text("こんにちは 世界 🌍".to_string());
        input.set_cursor_position(10); // At end

        // Test Ctrl+W with unicode
        let changed = input.handle_key(create_key_event_with_modifiers(
            KeyCode::Char('w'),
            KeyModifiers::CONTROL,
        ));
        assert!(changed);
        assert_eq!(input.text(), "こんにちは 世界 ");

        // Test Alt+B with unicode
        let changed = input.handle_key(create_key_event_with_modifiers(
            KeyCode::Char('b'),
            KeyModifiers::ALT,
        ));
        assert!(!changed);
        assert_eq!(input.cursor_position(), 6); // Beginning of "世界"

        // Test Ctrl+U with unicode
        let changed = input.handle_key(create_key_event_with_modifiers(
            KeyCode::Char('u'),
            KeyModifiers::CONTROL,
        ));
        assert!(changed);
        assert_eq!(input.text(), "世界 ");
        assert_eq!(input.cursor_position(), 0);
    }

    #[test]
    fn test_control_chars_dont_insert() {
        let mut input = TextInput::new();
        input.set_text("hello".to_string());

        // Control+character combinations should not insert the character
        let changed = input.handle_key(create_key_event_with_modifiers(
            KeyCode::Char('x'),
            KeyModifiers::CONTROL,
        ));
        assert!(!changed);
        assert_eq!(input.text(), "hello");

        // Alt+character combinations should not insert the character
        let changed = input.handle_key(create_key_event_with_modifiers(
            KeyCode::Char('x'),
            KeyModifiers::ALT,
        ));
        assert!(!changed);
        assert_eq!(input.text(), "hello");
    }

    #[test]
    fn test_render_cursor_spans() {
        let mut input = TextInput::new();

        // Empty text
        let spans = input.render_cursor_spans();
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].content, " ");

        // Text with cursor at end
        input.set_text("hello".to_string());
        let spans = input.render_cursor_spans();
        assert_eq!(spans.len(), 2);
        assert_eq!(spans[0].content, "hello");
        assert_eq!(spans[1].content, " ");

        // Text with cursor in middle
        input.set_cursor_position(2);
        let spans = input.render_cursor_spans();
        assert_eq!(spans.len(), 3);
        assert_eq!(spans[0].content, "he");
        assert_eq!(spans[1].content, "l");
        assert_eq!(spans[2].content, "lo");

        // Text with cursor at beginning
        input.set_cursor_position(0);
        let spans = input.render_cursor_spans();
        assert_eq!(spans.len(), 2);
        assert_eq!(spans[0].content, "h");
        assert_eq!(spans[1].content, "ello");
    }

    #[test]
    fn test_render_cursor_spans_unicode() {
        let mut input = TextInput::new();
        input.set_text("こんにちは".to_string());

        // Cursor at position 2 (after こん)
        input.set_cursor_position(2);
        let spans = input.render_cursor_spans();
        assert_eq!(spans.len(), 3);
        assert_eq!(spans[0].content, "こん");
        assert_eq!(spans[1].content, "に");
        assert_eq!(spans[2].content, "ちは");
    }
}
