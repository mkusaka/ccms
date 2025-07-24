#[cfg(test)]
mod tests {
    use super::super::Component;
    use super::super::search_bar::*;
    use crate::interactive_ratatui::ui::events::Message;
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
    fn test_search_bar_creation() {
        let search_bar = SearchBar::new();

        assert_eq!(search_bar.get_query(), "");
        assert!(!search_bar.is_searching());
    }

    #[test]
    fn test_character_input() {
        let mut search_bar = SearchBar::new();

        let msg = search_bar.handle_key(create_key_event(KeyCode::Char('h')));
        assert!(matches!(msg, Some(Message::QueryChanged(q)) if q == "h"));

        let msg = search_bar.handle_key(create_key_event(KeyCode::Char('i')));
        assert!(matches!(msg, Some(Message::QueryChanged(q)) if q == "hi"));

        assert_eq!(search_bar.get_query(), "hi");
    }

    #[test]
    fn test_backspace() {
        let mut search_bar = SearchBar::new();
        search_bar.set_query("hello".to_string());

        let msg = search_bar.handle_key(create_key_event(KeyCode::Backspace));
        assert!(matches!(msg, Some(Message::QueryChanged(q)) if q == "hell"));

        // Backspace at beginning should do nothing
        search_bar.set_query("".to_string());
        let msg = search_bar.handle_key(create_key_event(KeyCode::Backspace));
        assert!(msg.is_none());
    }

    #[test]
    fn test_cursor_movement() {
        let mut search_bar = SearchBar::new();
        search_bar.set_query("hello".to_string());

        // Move to beginning
        let msg = search_bar.handle_key(create_key_event(KeyCode::Home));
        assert!(msg.is_none());

        // Type at beginning
        let msg = search_bar.handle_key(create_key_event(KeyCode::Char('X')));
        assert!(matches!(msg, Some(Message::QueryChanged(q)) if q == "Xhello"));

        // Move to end
        let msg = search_bar.handle_key(create_key_event(KeyCode::End));
        assert!(msg.is_none());

        // Type at end
        let msg = search_bar.handle_key(create_key_event(KeyCode::Char('Y')));
        assert!(matches!(msg, Some(Message::QueryChanged(q)) if q == "XhelloY"));
    }

    #[test]
    fn test_delete_key() {
        let mut search_bar = SearchBar::new();
        search_bar.set_query("hello".to_string());

        // Move to beginning and delete
        search_bar.handle_key(create_key_event(KeyCode::Home));
        let msg = search_bar.handle_key(create_key_event(KeyCode::Delete));
        assert!(matches!(msg, Some(Message::QueryChanged(q)) if q == "ello"));

        // Delete at end should do nothing
        search_bar.handle_key(create_key_event(KeyCode::End));
        let msg = search_bar.handle_key(create_key_event(KeyCode::Delete));
        assert!(msg.is_none());
    }

    #[test]
    fn test_unicode_input() {
        let mut search_bar = SearchBar::new();

        // Japanese characters
        let msg = search_bar.handle_key(create_key_event(KeyCode::Char('こ')));
        assert!(matches!(msg, Some(Message::QueryChanged(q)) if q == "こ"));

        let msg = search_bar.handle_key(create_key_event(KeyCode::Char('ん')));
        assert!(matches!(msg, Some(Message::QueryChanged(q)) if q == "こん"));

        // Emoji
        let msg = search_bar.handle_key(create_key_event(KeyCode::Char('🔍')));
        assert!(matches!(msg, Some(Message::QueryChanged(q)) if q == "こん🔍"));

        assert_eq!(search_bar.get_query(), "こん🔍");
    }

    #[test]
    fn test_searching_state() {
        let mut search_bar = SearchBar::new();

        assert!(!search_bar.is_searching());

        search_bar.set_searching(true);
        assert!(search_bar.is_searching());

        search_bar.set_searching(false);
        assert!(!search_bar.is_searching());
    }

    #[test]
    fn test_message_display() {
        let mut search_bar = SearchBar::new();

        search_bar.set_message(Some("Loading...".to_string()));
        // Message should be set (would be displayed in render)

        search_bar.set_message(None);
        // Message should be cleared
    }

    #[test]
    fn test_role_filter_display() {
        let mut search_bar = SearchBar::new();

        search_bar.set_role_filter(Some("user".to_string()));
        // Role filter should be set (would be displayed in render)

        search_bar.set_role_filter(None);
        // Role filter should be cleared
    }

    #[test]
    fn test_arrow_keys() {
        let mut search_bar = SearchBar::new();
        search_bar.set_query("hello".to_string());

        // Move cursor left
        search_bar.handle_key(create_key_event(KeyCode::End));
        let msg = search_bar.handle_key(create_key_event(KeyCode::Left));
        assert!(msg.is_none());

        // Should be at position 4 now, type something
        let msg = search_bar.handle_key(create_key_event(KeyCode::Char('X')));
        assert!(matches!(msg, Some(Message::QueryChanged(q)) if q == "hellXo"));

        // Move right
        let msg = search_bar.handle_key(create_key_event(KeyCode::Right));
        assert!(msg.is_none());

        // Should be at end now
        let msg = search_bar.handle_key(create_key_event(KeyCode::Char('Y')));
        assert!(matches!(msg, Some(Message::QueryChanged(q)) if q == "hellXoY"));
    }

    #[test]
    fn test_ctrl_a_move_to_beginning() {
        let mut search_bar = SearchBar::new();
        search_bar.set_query("hello world".to_string());

        // Move cursor to beginning with Ctrl+A
        let msg = search_bar.handle_key(create_key_event_with_modifiers(
            KeyCode::Char('a'),
            KeyModifiers::CONTROL,
        ));
        assert!(msg.is_none());

        // Type at beginning to verify cursor position
        let msg = search_bar.handle_key(create_key_event(KeyCode::Char('X')));
        assert!(matches!(msg, Some(Message::QueryChanged(q)) if q == "Xhello world"));
    }

    #[test]
    fn test_ctrl_e_move_to_end() {
        let mut search_bar = SearchBar::new();
        search_bar.set_query("hello world".to_string());

        // Move to beginning first
        search_bar.handle_key(create_key_event(KeyCode::Home));

        // Move cursor to end with Ctrl+E
        let msg = search_bar.handle_key(create_key_event_with_modifiers(
            KeyCode::Char('e'),
            KeyModifiers::CONTROL,
        ));
        assert!(msg.is_none());

        // Type at end to verify cursor position
        let msg = search_bar.handle_key(create_key_event(KeyCode::Char('X')));
        assert!(matches!(msg, Some(Message::QueryChanged(q)) if q == "hello worldX"));
    }

    #[test]
    fn test_ctrl_b_move_backward() {
        let mut search_bar = SearchBar::new();
        search_bar.set_query("hello".to_string());

        // Move cursor backward with Ctrl+B
        let msg = search_bar.handle_key(create_key_event_with_modifiers(
            KeyCode::Char('b'),
            KeyModifiers::CONTROL,
        ));
        assert!(msg.is_none());

        // Type to verify cursor moved backward
        let msg = search_bar.handle_key(create_key_event(KeyCode::Char('X')));
        assert!(matches!(msg, Some(Message::QueryChanged(q)) if q == "hellXo"));
    }

    #[test]
    fn test_ctrl_f_move_forward() {
        let mut search_bar = SearchBar::new();
        search_bar.set_query("hello".to_string());
        search_bar.handle_key(create_key_event(KeyCode::Home));

        // Move cursor forward with Ctrl+F
        let msg = search_bar.handle_key(create_key_event_with_modifiers(
            KeyCode::Char('f'),
            KeyModifiers::CONTROL,
        ));
        assert!(msg.is_none());

        // Type to verify cursor moved forward
        let msg = search_bar.handle_key(create_key_event(KeyCode::Char('X')));
        assert!(matches!(msg, Some(Message::QueryChanged(q)) if q == "hXello"));
    }

    #[test]
    fn test_ctrl_h_delete_before_cursor() {
        let mut search_bar = SearchBar::new();
        search_bar.set_query("hello".to_string());

        // Delete character before cursor with Ctrl+H
        let msg = search_bar.handle_key(create_key_event_with_modifiers(
            KeyCode::Char('h'),
            KeyModifiers::CONTROL,
        ));
        assert!(matches!(msg, Some(Message::QueryChanged(q)) if q == "hell"));

        // Ctrl+H at beginning should do nothing
        search_bar.set_query("a".to_string());
        search_bar.handle_key(create_key_event(KeyCode::Home));
        let msg = search_bar.handle_key(create_key_event_with_modifiers(
            KeyCode::Char('h'),
            KeyModifiers::CONTROL,
        ));
        assert!(msg.is_none());
    }

    #[test]
    fn test_ctrl_d_delete_under_cursor() {
        let mut search_bar = SearchBar::new();
        search_bar.set_query("hello".to_string());
        search_bar.handle_key(create_key_event(KeyCode::Home));

        // Delete character under cursor with Ctrl+D
        let msg = search_bar.handle_key(create_key_event_with_modifiers(
            KeyCode::Char('d'),
            KeyModifiers::CONTROL,
        ));
        assert!(matches!(msg, Some(Message::QueryChanged(q)) if q == "ello"));

        // Ctrl+D at end should do nothing
        search_bar.handle_key(create_key_event(KeyCode::End));
        let msg = search_bar.handle_key(create_key_event_with_modifiers(
            KeyCode::Char('d'),
            KeyModifiers::CONTROL,
        ));
        assert!(msg.is_none());
    }

    #[test]
    fn test_ctrl_w_delete_word_before_cursor() {
        let mut search_bar = SearchBar::new();
        search_bar.set_query("hello world test".to_string());

        // Delete word before cursor with Ctrl+W
        let msg = search_bar.handle_key(create_key_event_with_modifiers(
            KeyCode::Char('w'),
            KeyModifiers::CONTROL,
        ));
        assert!(matches!(msg, Some(Message::QueryChanged(q)) if q == "hello world "));

        // Delete another word
        let msg = search_bar.handle_key(create_key_event_with_modifiers(
            KeyCode::Char('w'),
            KeyModifiers::CONTROL,
        ));
        assert!(matches!(msg, Some(Message::QueryChanged(q)) if q == "hello "));

        // Ctrl+W at beginning should do nothing
        search_bar.set_query("test".to_string());
        search_bar.handle_key(create_key_event(KeyCode::Home));
        let msg = search_bar.handle_key(create_key_event_with_modifiers(
            KeyCode::Char('w'),
            KeyModifiers::CONTROL,
        ));
        assert!(msg.is_none());
    }

    #[test]
    fn test_ctrl_u_delete_to_beginning() {
        let mut search_bar = SearchBar::new();
        search_bar.set_query("hello world".to_string());

        // Move cursor to middle
        search_bar.handle_key(create_key_event(KeyCode::Home));
        for _ in 0..6 {
            search_bar.handle_key(create_key_event(KeyCode::Right));
        }

        // Delete to beginning with Ctrl+U
        let msg = search_bar.handle_key(create_key_event_with_modifiers(
            KeyCode::Char('u'),
            KeyModifiers::CONTROL,
        ));
        assert!(matches!(msg, Some(Message::QueryChanged(q)) if q == "world"));

        // Ctrl+U at beginning should do nothing
        search_bar.handle_key(create_key_event(KeyCode::Home));
        let msg = search_bar.handle_key(create_key_event_with_modifiers(
            KeyCode::Char('u'),
            KeyModifiers::CONTROL,
        ));
        assert!(msg.is_none());
    }

    #[test]
    fn test_ctrl_k_delete_to_end() {
        let mut search_bar = SearchBar::new();
        search_bar.set_query("hello world".to_string());

        // Move cursor to middle
        search_bar.handle_key(create_key_event(KeyCode::Home));
        for _ in 0..6 {
            search_bar.handle_key(create_key_event(KeyCode::Right));
        }

        // Delete to end with Ctrl+K
        let msg = search_bar.handle_key(create_key_event_with_modifiers(
            KeyCode::Char('k'),
            KeyModifiers::CONTROL,
        ));
        assert!(matches!(msg, Some(Message::QueryChanged(q)) if q == "hello "));

        // Ctrl+K at end should do nothing
        search_bar.handle_key(create_key_event(KeyCode::End));
        let msg = search_bar.handle_key(create_key_event_with_modifiers(
            KeyCode::Char('k'),
            KeyModifiers::CONTROL,
        ));
        assert!(msg.is_none());
    }

    #[test]
    fn test_alt_b_move_word_backward() {
        let mut search_bar = SearchBar::new();
        search_bar.set_query("hello world test".to_string());

        // Move backward by word with Alt+B
        let msg = search_bar.handle_key(create_key_event_with_modifiers(
            KeyCode::Char('b'),
            KeyModifiers::ALT,
        ));
        assert!(msg.is_none());

        // Type to verify cursor position
        let msg = search_bar.handle_key(create_key_event(KeyCode::Char('X')));
        assert!(matches!(msg, Some(Message::QueryChanged(q)) if q == "hello world Xtest"));

        // Clear the 'X' we just added by using Ctrl+H
        search_bar.handle_key(create_key_event_with_modifiers(
            KeyCode::Char('h'),
            KeyModifiers::CONTROL,
        ));

        // Move backward again
        let msg = search_bar.handle_key(create_key_event_with_modifiers(
            KeyCode::Char('b'),
            KeyModifiers::ALT,
        ));
        assert!(msg.is_none());

        let msg = search_bar.handle_key(create_key_event(KeyCode::Char('Y')));
        assert!(matches!(msg, Some(Message::QueryChanged(q)) if q == "hello Yworld test"));
    }

    #[test]
    fn test_alt_f_move_word_forward() {
        let mut search_bar = SearchBar::new();
        search_bar.set_query("hello world test".to_string());
        search_bar.handle_key(create_key_event(KeyCode::Home));

        // Move forward by word with Alt+F
        let msg = search_bar.handle_key(create_key_event_with_modifiers(
            KeyCode::Char('f'),
            KeyModifiers::ALT,
        ));
        assert!(msg.is_none());

        // Type to verify cursor position
        let msg = search_bar.handle_key(create_key_event(KeyCode::Char('X')));
        assert!(matches!(msg, Some(Message::QueryChanged(q)) if q == "hello Xworld test"));

        // Move forward again
        let msg = search_bar.handle_key(create_key_event_with_modifiers(
            KeyCode::Char('f'),
            KeyModifiers::ALT,
        ));
        assert!(msg.is_none());

        let msg = search_bar.handle_key(create_key_event(KeyCode::Char('Y')));
        assert!(matches!(msg, Some(Message::QueryChanged(q)) if q == "hello Xworld Ytest"));
    }

    #[test]
    fn test_shortcuts_with_unicode() {
        let mut search_bar = SearchBar::new();
        search_bar.set_query("こんにちは 世界 🌍".to_string());

        // Test Ctrl+W with unicode
        let msg = search_bar.handle_key(create_key_event_with_modifiers(
            KeyCode::Char('w'),
            KeyModifiers::CONTROL,
        ));
        assert!(matches!(msg, Some(Message::QueryChanged(q)) if q == "こんにちは 世界 "));

        // Test Alt+B with unicode
        let msg = search_bar.handle_key(create_key_event_with_modifiers(
            KeyCode::Char('b'),
            KeyModifiers::ALT,
        ));
        assert!(msg.is_none());

        // Type to verify position
        let msg = search_bar.handle_key(create_key_event(KeyCode::Char('X')));
        assert!(matches!(msg, Some(Message::QueryChanged(q)) if q == "こんにちは X世界 "));

        // Test Ctrl+U with unicode
        let msg = search_bar.handle_key(create_key_event_with_modifiers(
            KeyCode::Char('u'),
            KeyModifiers::CONTROL,
        ));
        assert!(matches!(msg, Some(Message::QueryChanged(q)) if q == "世界 "));
    }

    #[test]
    fn test_control_chars_dont_insert() {
        let mut search_bar = SearchBar::new();
        search_bar.set_query("hello".to_string());

        // Control+character combinations should not insert the character
        let msg = search_bar.handle_key(create_key_event_with_modifiers(
            KeyCode::Char('x'),
            KeyModifiers::CONTROL,
        ));
        assert!(msg.is_none());
        assert_eq!(search_bar.get_query(), "hello");

        // Alt+character combinations should not insert the character
        let msg = search_bar.handle_key(create_key_event_with_modifiers(
            KeyCode::Char('x'),
            KeyModifiers::ALT,
        ));
        assert!(msg.is_none());
        assert_eq!(search_bar.get_query(), "hello");
    }
}
