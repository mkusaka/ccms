#[cfg(test)]
mod tests {
    use super::super::search_bar::SearchBar;
    use crate::interactive_ratatui::ui::tuirealm_components::messages::AppMessage;
    use tuirealm::event::{Key, KeyEvent, KeyModifiers};
    use tuirealm::{Component, Event, NoUserEvent, MockComponent, State, StateValue};

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
    fn test_search_bar_creation() {
        let search_bar = SearchBar::new();
        assert_eq!(search_bar.get_query(), "");
        assert!(!search_bar.is_searching());
    }

    #[test]
    fn test_set_query() {
        let mut search_bar = SearchBar::new();
        search_bar.set_query("test query".to_string());
        assert_eq!(search_bar.get_query(), "test query");
    }

    #[test]
    fn test_set_searching() {
        let mut search_bar = SearchBar::new();
        search_bar.set_searching(true);
        assert!(search_bar.is_searching());
        
        search_bar.set_searching(false);
        assert!(!search_bar.is_searching());
    }

    #[test]
    fn test_set_message() {
        let mut search_bar = SearchBar::new();
        search_bar.set_message(Some("Searching...".to_string()));
        // Message is internal state, tested through view rendering
        
        search_bar.set_message(None);
        // Message cleared
    }

    #[test]
    fn test_set_role_filter() {
        let mut search_bar = SearchBar::new();
        search_bar.set_role_filter(Some("User".to_string()));
        // Role filter is internal state, tested through view rendering
        
        search_bar.set_role_filter(None);
        // Role filter cleared
    }

    #[test]
    fn test_character_input() {
        let mut search_bar = SearchBar::new();

        let msg = search_bar.on(create_key_event(Key::Char('h')));
        assert_eq!(msg, Some(AppMessage::QueryChanged("h".to_string())));
        assert_eq!(search_bar.get_query(), "h");

        let msg = search_bar.on(create_key_event(Key::Char('e')));
        assert_eq!(msg, Some(AppMessage::QueryChanged("he".to_string())));
        assert_eq!(search_bar.get_query(), "he");

        let msg = search_bar.on(create_key_event(Key::Char('l')));
        assert_eq!(msg, Some(AppMessage::QueryChanged("hel".to_string())));
        assert_eq!(search_bar.get_query(), "hel");
    }

    #[test]
    fn test_backspace() {
        let mut search_bar = SearchBar::new();
        search_bar.set_query("hello".to_string());

        let msg = search_bar.on(create_key_event(Key::Backspace));
        assert_eq!(msg, Some(AppMessage::QueryChanged("hell".to_string())));
        assert_eq!(search_bar.get_query(), "hell");
    }

    #[test]
    fn test_delete() {
        let mut search_bar = SearchBar::new();
        search_bar.set_query("hello".to_string());

        // Move cursor to beginning
        search_bar.on(create_key_event(Key::Home));

        let msg = search_bar.on(create_key_event(Key::Delete));
        assert_eq!(msg, Some(AppMessage::QueryChanged("ello".to_string())));
        assert_eq!(search_bar.get_query(), "ello");
    }

    #[test]
    fn test_cursor_movement() {
        let mut search_bar = SearchBar::new();
        search_bar.set_query("hello".to_string());

        // Test cursor movement keys - they don't generate messages
        let msg = search_bar.on(create_key_event(Key::Left));
        assert_eq!(msg, None);

        let msg = search_bar.on(create_key_event(Key::Right));
        assert_eq!(msg, None);

        let msg = search_bar.on(create_key_event(Key::Home));
        assert_eq!(msg, None);

        let msg = search_bar.on(create_key_event(Key::End));
        assert_eq!(msg, None);
    }

    #[test]
    fn test_ctrl_shortcuts() {
        let mut search_bar = SearchBar::new();
        search_bar.set_query("hello world".to_string());

        // Ctrl+A - move to beginning
        let msg = search_bar.on(create_key_event_with_modifiers(
            Key::Char('a'),
            KeyModifiers::CONTROL,
        ));
        assert_eq!(msg, None);

        // Ctrl+E - move to end
        let msg = search_bar.on(create_key_event_with_modifiers(
            Key::Char('e'),
            KeyModifiers::CONTROL,
        ));
        assert_eq!(msg, None);

        // Ctrl+H - delete character before cursor
        let msg = search_bar.on(create_key_event_with_modifiers(
            Key::Char('h'),
            KeyModifiers::CONTROL,
        ));
        assert_eq!(msg, Some(AppMessage::QueryChanged("hello worl".to_string())));

        // Ctrl+D - delete character under cursor
        search_bar.on(create_key_event(Key::Home));
        let msg = search_bar.on(create_key_event_with_modifiers(
            Key::Char('d'),
            KeyModifiers::CONTROL,
        ));
        assert_eq!(msg, Some(AppMessage::QueryChanged("ello worl".to_string())));
    }

    #[test]
    fn test_ctrl_w_delete_word() {
        let mut search_bar = SearchBar::new();
        search_bar.set_query("hello world test".to_string());

        let msg = search_bar.on(create_key_event_with_modifiers(
            Key::Char('w'),
            KeyModifiers::CONTROL,
        ));
        assert_eq!(msg, Some(AppMessage::QueryChanged("hello world ".to_string())));
        assert_eq!(search_bar.get_query(), "hello world ");
    }

    #[test]
    fn test_ctrl_u_delete_to_beginning() {
        let mut search_bar = SearchBar::new();
        search_bar.set_query("hello world".to_string());
        
        // Move cursor to middle
        for _ in 0..5 {
            search_bar.on(create_key_event(Key::Left));
        }

        let msg = search_bar.on(create_key_event_with_modifiers(
            Key::Char('u'),
            KeyModifiers::CONTROL,
        ));
        assert_eq!(msg, Some(AppMessage::QueryChanged("world".to_string())));
        assert_eq!(search_bar.get_query(), "world");
    }

    #[test]
    fn test_ctrl_k_delete_to_end() {
        let mut search_bar = SearchBar::new();
        search_bar.set_query("hello world".to_string());
        
        // Move cursor to middle
        search_bar.on(create_key_event(Key::Home));
        for _ in 0..6 {
            search_bar.on(create_key_event(Key::Right));
        }

        let msg = search_bar.on(create_key_event_with_modifiers(
            Key::Char('k'),
            KeyModifiers::CONTROL,
        ));
        assert_eq!(msg, Some(AppMessage::QueryChanged("hello ".to_string())));
        assert_eq!(search_bar.get_query(), "hello ");
    }

    #[test]
    fn test_alt_word_navigation() {
        let mut search_bar = SearchBar::new();
        search_bar.set_query("hello world test".to_string());

        // Alt+B - move word backward
        let msg = search_bar.on(create_key_event_with_modifiers(
            Key::Char('b'),
            KeyModifiers::ALT,
        ));
        assert_eq!(msg, None);

        // Alt+F - move word forward  
        search_bar.on(create_key_event(Key::Home));
        let msg = search_bar.on(create_key_event_with_modifiers(
            Key::Char('f'),
            KeyModifiers::ALT,
        ));
        assert_eq!(msg, None);
    }

    #[test]
    fn test_unicode_handling() {
        let mut search_bar = SearchBar::new();
        search_bar.set_query("こんにちは 世界 🌍".to_string());

        // Test Ctrl+W with unicode
        let msg = search_bar.on(create_key_event_with_modifiers(
            Key::Char('w'),
            KeyModifiers::CONTROL,
        ));
        assert_eq!(msg, Some(AppMessage::QueryChanged("こんにちは 世界 ".to_string())));
        assert_eq!(search_bar.get_query(), "こんにちは 世界 ");
    }

    #[test]
    fn test_state() {
        let mut search_bar = SearchBar::new();
        search_bar.set_query("test query".to_string());
        
        match search_bar.state() {
            State::One(StateValue::String(s)) => assert_eq!(s, "test query"),
            _ => panic!("Unexpected state type"),
        }
    }

    #[test]
    fn test_complex_workflow() {
        let mut search_bar = SearchBar::new();
        
        // Type a query
        search_bar.on(create_key_event(Key::Char('t')));
        search_bar.on(create_key_event(Key::Char('e')));
        search_bar.on(create_key_event(Key::Char('s')));
        search_bar.on(create_key_event(Key::Char('t')));
        assert_eq!(search_bar.get_query(), "test");
        
        // Add a space
        search_bar.on(create_key_event(Key::Char(' ')));
        assert_eq!(search_bar.get_query(), "test ");
        
        // Add more text
        search_bar.on(create_key_event(Key::Char('q')));
        search_bar.on(create_key_event(Key::Char('u')));
        search_bar.on(create_key_event(Key::Char('e')));
        search_bar.on(create_key_event(Key::Char('r')));
        search_bar.on(create_key_event(Key::Char('y')));
        assert_eq!(search_bar.get_query(), "test query");
        
        // Delete word
        let msg = search_bar.on(create_key_event_with_modifiers(
            Key::Char('w'),
            KeyModifiers::CONTROL,
        ));
        assert_eq!(msg, Some(AppMessage::QueryChanged("test ".to_string())));
        
        // Clear to beginning
        let msg = search_bar.on(create_key_event_with_modifiers(
            Key::Char('u'),
            KeyModifiers::CONTROL,
        ));
        assert_eq!(msg, Some(AppMessage::QueryChanged("".to_string())));
        assert_eq!(search_bar.get_query(), "");
    }
}