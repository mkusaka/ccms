#[cfg(test)]
mod tests {
    use super::super::text_input::TextInput;
    use crate::interactive_ratatui::ui::tuirealm_components::messages::AppMessage;
    use tuirealm::event::{Key, KeyEvent, KeyModifiers};
    use tuirealm::{Component, Event, NoUserEvent, MockComponent};
    use tuirealm::props::{AttrValue, Attribute};

    fn create_key_event(code: Key) -> Event<NoUserEvent> {
        Event::Keyboard(KeyEvent {
            code,
            modifiers: KeyModifiers::NONE,
        })
    }

    fn create_key_event_with_modifiers(code: Key, modifiers: KeyModifiers) -> Event<NoUserEvent> {
        Event::Keyboard(KeyEvent { code, modifiers })
    }

    #[test]
    fn test_text_input_creation() {
        let input = TextInput::default();
        assert_eq!(input.text(), "");
    }

    #[test]
    fn test_set_text() {
        let mut input = TextInput::new("hello world".to_string());
        assert_eq!(input.text(), "hello world");
        
        input.set_text("new text".to_string());
        assert_eq!(input.text(), "new text");
    }

    #[test]
    fn test_character_input() {
        let mut input = TextInput::default();
        input.attr(Attribute::Custom("id"), AttrValue::String("search_bar".to_string()));

        let msg = input.on(create_key_event(Key::Char('h')));
        assert_eq!(msg, Some(AppMessage::QueryChanged("h".to_string())));
        assert_eq!(input.text(), "h");

        let msg = input.on(create_key_event(Key::Char('i')));
        assert_eq!(msg, Some(AppMessage::QueryChanged("hi".to_string())));
        assert_eq!(input.text(), "hi");
    }

    #[test]
    fn test_backspace() {
        let mut input = TextInput::new("hello".to_string());
        input.attr(Attribute::Custom("id"), AttrValue::String("search_bar".to_string()));

        let msg = input.on(create_key_event(Key::Backspace));
        assert_eq!(msg, Some(AppMessage::QueryChanged("hell".to_string())));
        assert_eq!(input.text(), "hell");
    }

    #[test]
    fn test_delete() {
        let mut input = TextInput::new("hello".to_string());
        input.attr(Attribute::Custom("id"), AttrValue::String("search_bar".to_string()));

        // Move cursor to beginning
        input.on(create_key_event(Key::Home));

        let msg = input.on(create_key_event(Key::Delete));
        assert_eq!(msg, Some(AppMessage::QueryChanged("ello".to_string())));
        assert_eq!(input.text(), "ello");
    }

    #[test]
    fn test_cursor_movement() {
        let mut input = TextInput::new("hello".to_string());

        // Move left
        let msg = input.on(create_key_event(Key::Left));
        assert_eq!(msg, None); // cursor movement doesn't generate messages

        // Move right at end does nothing
        input.on(create_key_event(Key::End));
        let msg = input.on(create_key_event(Key::Right));
        assert_eq!(msg, None);

        // Home and End
        input.on(create_key_event(Key::Home));
        input.on(create_key_event(Key::End));
    }

    #[test]
    fn test_ctrl_a_move_to_beginning() {
        let mut input = TextInput::new("hello world".to_string());

        let msg = input.on(create_key_event_with_modifiers(
            Key::Char('a'),
            KeyModifiers::CONTROL,
        ));
        assert_eq!(msg, None);
    }

    #[test]
    fn test_ctrl_e_move_to_end() {
        let mut input = TextInput::new("hello world".to_string());
        input.on(create_key_event(Key::Home)); // Move to beginning first

        let msg = input.on(create_key_event_with_modifiers(
            Key::Char('e'),
            KeyModifiers::CONTROL,
        ));
        assert_eq!(msg, None);
    }

    #[test]
    fn test_ctrl_b_move_backward() {
        let mut input = TextInput::new("hello".to_string());

        let msg = input.on(create_key_event_with_modifiers(
            Key::Char('b'),
            KeyModifiers::CONTROL,
        ));
        assert_eq!(msg, None);
    }

    #[test]
    fn test_ctrl_f_move_forward() {
        let mut input = TextInput::new("hello".to_string());
        input.on(create_key_event(Key::Home)); // Move to beginning

        let msg = input.on(create_key_event_with_modifiers(
            Key::Char('f'),
            KeyModifiers::CONTROL,
        ));
        assert_eq!(msg, None);
    }

    #[test]
    fn test_alt_b_move_word_backward() {
        let mut input = TextInput::new("hello world test".to_string());

        let msg = input.on(create_key_event_with_modifiers(
            Key::Char('b'),
            KeyModifiers::ALT,
        ));
        assert_eq!(msg, None);
    }

    #[test]
    fn test_alt_f_move_word_forward() {
        let mut input = TextInput::new("hello world test".to_string());
        input.on(create_key_event(Key::Home)); // Move to beginning

        let msg = input.on(create_key_event_with_modifiers(
            Key::Char('f'),
            KeyModifiers::ALT,
        ));
        assert_eq!(msg, None);
    }

    #[test]
    fn test_ctrl_h_delete_before_cursor() {
        let mut input = TextInput::new("hello".to_string());
        input.attr(Attribute::Custom("id"), AttrValue::String("search_bar".to_string()));

        let msg = input.on(create_key_event_with_modifiers(
            Key::Char('h'),
            KeyModifiers::CONTROL,
        ));
        assert_eq!(msg, Some(AppMessage::QueryChanged("hell".to_string())));
        assert_eq!(input.text(), "hell");
    }

    #[test]
    fn test_ctrl_d_delete_under_cursor() {
        let mut input = TextInput::new("hello".to_string());
        input.attr(Attribute::Custom("id"), AttrValue::String("search_bar".to_string()));
        input.on(create_key_event(Key::Home)); // Move to beginning

        let msg = input.on(create_key_event_with_modifiers(
            Key::Char('d'),
            KeyModifiers::CONTROL,
        ));
        assert_eq!(msg, Some(AppMessage::QueryChanged("ello".to_string())));
        assert_eq!(input.text(), "ello");
    }

    #[test]
    fn test_ctrl_w_delete_word_before_cursor() {
        let mut input = TextInput::new("hello world test".to_string());
        input.attr(Attribute::Custom("id"), AttrValue::String("search_bar".to_string()));

        let msg = input.on(create_key_event_with_modifiers(
            Key::Char('w'),
            KeyModifiers::CONTROL,
        ));
        assert_eq!(msg, Some(AppMessage::QueryChanged("hello world ".to_string())));
        assert_eq!(input.text(), "hello world ");
    }

    #[test]
    fn test_ctrl_u_delete_to_beginning() {
        let mut input = TextInput::new("hello world".to_string());
        input.attr(Attribute::Custom("id"), AttrValue::String("search_bar".to_string()));
        
        // Move cursor to middle (after "hello ")
        for _ in 0..5 {
            input.on(create_key_event(Key::Left));
        }

        let msg = input.on(create_key_event_with_modifiers(
            Key::Char('u'),
            KeyModifiers::CONTROL,
        ));
        assert_eq!(msg, Some(AppMessage::QueryChanged("world".to_string())));
        assert_eq!(input.text(), "world");
    }

    #[test]
    fn test_ctrl_k_delete_to_end() {
        let mut input = TextInput::new("hello world".to_string());
        input.attr(Attribute::Custom("id"), AttrValue::String("search_bar".to_string()));
        
        // Move cursor to middle
        input.on(create_key_event(Key::Home));
        for _ in 0..6 {
            input.on(create_key_event(Key::Right));
        }

        let msg = input.on(create_key_event_with_modifiers(
            Key::Char('k'),
            KeyModifiers::CONTROL,
        ));
        assert_eq!(msg, Some(AppMessage::QueryChanged("hello ".to_string())));
        assert_eq!(input.text(), "hello ");
    }

    #[test]
    fn test_unicode_handling() {
        let mut input = TextInput::new("こんにちは 世界 🌍".to_string());
        input.attr(Attribute::Custom("id"), AttrValue::String("search_bar".to_string()));

        // Test Ctrl+W with unicode
        let msg = input.on(create_key_event_with_modifiers(
            Key::Char('w'),
            KeyModifiers::CONTROL,
        ));
        assert_eq!(msg, Some(AppMessage::QueryChanged("こんにちは 世界 ".to_string())));
    }

    #[test]
    fn test_control_chars_dont_insert() {
        let mut input = TextInput::new("hello".to_string());

        // Control+character combinations should not insert the character
        let msg = input.on(create_key_event_with_modifiers(
            Key::Char('x'),
            KeyModifiers::CONTROL,
        ));
        assert_eq!(msg, None);
        assert_eq!(input.text(), "hello");

        // Alt+character combinations should not insert the character
        let msg = input.on(create_key_event_with_modifiers(
            Key::Char('x'),
            KeyModifiers::ALT,
        ));
        assert_eq!(msg, None);
        assert_eq!(input.text(), "hello");
    }

    #[test]
    fn test_session_search_messages() {
        let mut input = TextInput::default();
        input.attr(Attribute::Custom("id"), AttrValue::String("session_search".to_string()));

        let msg = input.on(create_key_event(Key::Char('t')));
        assert_eq!(msg, Some(AppMessage::SessionQueryChanged("t".to_string())));
    }
}