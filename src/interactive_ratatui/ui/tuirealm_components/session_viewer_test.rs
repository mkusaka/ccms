#[cfg(test)]
mod tests {
    use super::super::session_viewer::SessionViewer;
    use crate::interactive_ratatui::ui::tuirealm_components::messages::AppMessage;
    use crate::interactive_ratatui::domain::models::SessionOrder;
    use tuirealm::event::{Key, KeyEvent, KeyModifiers};
    use tuirealm::{Component, Event, NoUserEvent, MockComponent, State, StateValue};

    fn create_key_event(code: Key) -> Event<NoUserEvent> {
        Event::Keyboard(KeyEvent {
            code,
            modifiers: KeyModifiers::NONE,
        })
    }

    fn create_sample_messages() -> Vec<String> {
        vec![
            r#"{"type":"user","timestamp":"2024-01-01T12:00:00Z","message":{"content":"Hello"}}"#.to_string(),
            r#"{"type":"assistant","timestamp":"2024-01-01T12:01:00Z","message":{"content":"Hi there!"}}"#.to_string(),
            r#"{"type":"system","timestamp":"2024-01-01T12:02:00Z","content":"System message"}"#.to_string(),
            r#"{"type":"summary","timestamp":"2024-01-01T12:03:00Z","summary":"Summary text"}"#.to_string(),
        ]
    }

    #[test]
    fn test_session_viewer_creation() {
        let session_viewer = SessionViewer::new();
        assert_eq!(session_viewer.items_count(), 0);
        assert_eq!(session_viewer.filtered_count(), 0);
        assert!(!session_viewer.is_searching());
    }

    #[test]
    fn test_set_messages() {
        let mut session_viewer = SessionViewer::new();
        let messages = create_sample_messages();
        
        session_viewer.set_messages(messages);
        assert_eq!(session_viewer.items_count(), 4);
        assert_eq!(session_viewer.filtered_count(), 4);
    }

    #[test]
    fn test_set_filtered_indices() {
        let mut session_viewer = SessionViewer::new();
        session_viewer.set_messages(create_sample_messages());
        
        // Filter to show only first two messages
        session_viewer.set_filtered_indices(vec![0, 1]);
        assert_eq!(session_viewer.filtered_count(), 2);
    }

    #[test]
    fn test_navigation() {
        let mut session_viewer = SessionViewer::new();
        session_viewer.set_messages(create_sample_messages());
        
        // Move down with Down arrow
        let msg = session_viewer.on(create_key_event(Key::Down));
        assert_eq!(msg, Some(AppMessage::SessionScrollDown));
        
        // Move down with 'j'
        let msg = session_viewer.on(create_key_event(Key::Char('j')));
        assert_eq!(msg, Some(AppMessage::SessionScrollDown));
        
        // Move up with Up arrow
        let msg = session_viewer.on(create_key_event(Key::Up));
        assert_eq!(msg, Some(AppMessage::SessionScrollUp));
        
        // Move up with 'k'
        let msg = session_viewer.on(create_key_event(Key::Char('k')));
        assert_eq!(msg, Some(AppMessage::SessionScrollUp));
        
        // Try to move up at the beginning
        let msg = session_viewer.on(create_key_event(Key::Up));
        assert_eq!(msg, None);
    }

    #[test]
    fn test_search_mode() {
        let mut session_viewer = SessionViewer::new();
        session_viewer.set_messages(create_sample_messages());
        
        // Enter search mode
        let msg = session_viewer.on(create_key_event(Key::Char('/')));
        assert_eq!(msg, None);
        assert!(session_viewer.is_searching());
        
        // Type a search query
        let msg = session_viewer.on(create_key_event(Key::Char('t')));
        assert_eq!(msg, Some(AppMessage::SessionQueryChanged("t".to_string())));
        
        let msg = session_viewer.on(create_key_event(Key::Char('e')));
        assert_eq!(msg, Some(AppMessage::SessionQueryChanged("te".to_string())));
        
        let msg = session_viewer.on(create_key_event(Key::Char('s')));
        assert_eq!(msg, Some(AppMessage::SessionQueryChanged("tes".to_string())));
        
        let msg = session_viewer.on(create_key_event(Key::Char('t')));
        assert_eq!(msg, Some(AppMessage::SessionQueryChanged("test".to_string())));
        
        assert_eq!(session_viewer.get_query(), "test");
        
        // Exit search with Enter
        let msg = session_viewer.on(create_key_event(Key::Enter));
        assert_eq!(msg, None);
        assert!(!session_viewer.is_searching());
    }

    #[test]
    fn test_search_cancel() {
        let mut session_viewer = SessionViewer::new();
        session_viewer.set_messages(create_sample_messages());
        
        // Enter search mode
        session_viewer.on(create_key_event(Key::Char('/')));
        
        // Type something
        session_viewer.on(create_key_event(Key::Char('a')));
        session_viewer.on(create_key_event(Key::Char('b')));
        session_viewer.on(create_key_event(Key::Char('c')));
        
        // Cancel with Esc
        let msg = session_viewer.on(create_key_event(Key::Esc));
        assert_eq!(msg, Some(AppMessage::SessionQueryChanged(String::new())));
        assert!(!session_viewer.is_searching());
        assert_eq!(session_viewer.get_query(), "");
    }

    #[test]
    fn test_search_backspace_exit() {
        let mut session_viewer = SessionViewer::new();
        session_viewer.set_messages(create_sample_messages());
        
        // Enter search mode
        session_viewer.on(create_key_event(Key::Char('/')));
        
        // Type a single character
        session_viewer.on(create_key_event(Key::Char('a')));
        
        // Backspace should exit search when query becomes empty
        let msg = session_viewer.on(create_key_event(Key::Backspace));
        assert_eq!(msg, Some(AppMessage::SessionQueryChanged(String::new())));
        assert!(!session_viewer.is_searching());
    }

    #[test]
    fn test_action_keys() {
        let mut session_viewer = SessionViewer::new();
        let messages = create_sample_messages();
        session_viewer.set_messages(messages.clone());
        session_viewer.set_session_id(Some("test-session-id".to_string()));
        session_viewer.set_file_path(Some("/test/path.jsonl".to_string()));
        
        // Test 'o' key - Toggle order
        let msg = session_viewer.on(create_key_event(Key::Char('o')));
        assert_eq!(msg, Some(AppMessage::ToggleSessionOrder));
        
        // Test 'c' key - Copy selected JSON
        let msg = session_viewer.on(create_key_event(Key::Char('c')));
        assert!(matches!(msg, Some(AppMessage::CopyToClipboard(_))));
        
        // Test 'C' key - Copy all messages
        let msg = session_viewer.on(create_key_event(Key::Char('C')));
        if let Some(AppMessage::CopyToClipboard(text)) = msg {
            assert!(text.contains("Hello"));
            assert!(text.contains("Hi there!"));
        } else {
            panic!("Expected CopyToClipboard message");
        }
        
        // Test 'i' key - Copy session ID
        let msg = session_viewer.on(create_key_event(Key::Char('i')));
        assert_eq!(msg, Some(AppMessage::CopyToClipboard("test-session-id".to_string())));
        
        // Test 'f' key - Copy file path
        let msg = session_viewer.on(create_key_event(Key::Char('f')));
        assert_eq!(msg, Some(AppMessage::CopyToClipboard("/test/path.jsonl".to_string())));
        
        // Test 'm' key - Copy message content
        let msg = session_viewer.on(create_key_event(Key::Char('m')));
        assert!(matches!(msg, Some(AppMessage::CopyToClipboard(_))));
    }

    #[test]
    fn test_esc_key() {
        let mut session_viewer = SessionViewer::new();
        session_viewer.set_messages(create_sample_messages());
        
        let msg = session_viewer.on(create_key_event(Key::Esc));
        assert_eq!(msg, Some(AppMessage::ExitSessionViewer));
    }

    #[test]
    fn test_set_properties() {
        let mut session_viewer = SessionViewer::new();
        
        session_viewer.set_query("test query".to_string());
        assert_eq!(session_viewer.get_query(), "test query");
        
        session_viewer.set_order(Some(SessionOrder::Ascending));
        // Order is internal state, tested through display
        
        session_viewer.set_file_path(Some("/path/to/file.jsonl".to_string()));
        // File path is internal state
        
        session_viewer.set_session_id(Some("session-123".to_string()));
        // Session ID is internal state
        
        session_viewer.set_truncation_enabled(false);
        // Truncation is internal state
    }

    #[test]
    fn test_state() {
        let mut session_viewer = SessionViewer::new();
        session_viewer.set_messages(create_sample_messages());
        
        match session_viewer.state() {
            State::One(StateValue::Usize(idx)) => assert_eq!(idx, 0),
            _ => panic!("Unexpected state type"),
        }
        
        // Move down and check state
        session_viewer.on(create_key_event(Key::Down));
        match session_viewer.state() {
            State::One(StateValue::Usize(idx)) => assert_eq!(idx, 1),
            _ => panic!("Unexpected state type"),
        }
    }

    #[test]
    fn test_empty_messages() {
        let mut session_viewer = SessionViewer::new();
        
        // All navigation should fail with empty messages
        assert_eq!(session_viewer.on(create_key_event(Key::Down)), None);
        assert_eq!(session_viewer.on(create_key_event(Key::Up)), None);
    }

    #[test]
    fn test_navigation_boundaries() {
        let mut session_viewer = SessionViewer::new();
        session_viewer.set_messages(create_sample_messages());
        
        // Go to end
        for _ in 0..10 {
            session_viewer.on(create_key_event(Key::Down));
        }
        
        // Try to move down at the end
        let msg = session_viewer.on(create_key_event(Key::Down));
        assert_eq!(msg, None);
        
        // Go to beginning
        for _ in 0..10 {
            session_viewer.on(create_key_event(Key::Up));
        }
        
        // Try to move up at the beginning
        let msg = session_viewer.on(create_key_event(Key::Up));
        assert_eq!(msg, None);
    }

    #[test]
    fn test_start_stop_search() {
        let mut session_viewer = SessionViewer::new();
        
        session_viewer.start_search();
        assert!(session_viewer.is_searching());
        assert_eq!(session_viewer.get_query(), "");
        
        session_viewer.stop_search();
        assert!(!session_viewer.is_searching());
    }

    #[test]
    fn test_set_selected_index() {
        let mut session_viewer = SessionViewer::new();
        session_viewer.set_messages(create_sample_messages());
        
        session_viewer.set_selected_index(2);
        match session_viewer.state() {
            State::One(StateValue::Usize(idx)) => assert_eq!(idx, 2),
            _ => panic!("Unexpected state type"),
        }
    }
}