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
}
