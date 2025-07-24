#[cfg(test)]
mod tests {
    use super::super::result_detail::ResultDetail;
    use crate::interactive_ratatui::ui::tuirealm_components::messages::AppMessage;
    use crate::query::condition::{QueryCondition, SearchResult};
    use tuirealm::event::{Key, KeyEvent, KeyModifiers};
    use tuirealm::{Component, Event, NoUserEvent, MockComponent, State, StateValue};

    fn create_key_event(code: Key) -> Event<NoUserEvent> {
        Event::Keyboard(KeyEvent {
            code,
            modifiers: KeyModifiers::NONE,
        })
    }

    fn create_sample_result() -> SearchResult {
        SearchResult {
            file: "test_file.jsonl".to_string(),
            uuid: "test-uuid-1234".to_string(),
            timestamp: "2024-01-01T12:00:00Z".to_string(),
            session_id: "test-session-id".to_string(),
            role: "user".to_string(),
            text: "This is a test message with\nmultiple lines\nto test scrolling functionality".to_string(),
            has_tools: false,
            has_thinking: false,
            message_type: "message".to_string(),
            query: QueryCondition::Literal {
                pattern: "test".to_string(),
                case_sensitive: false,
            },
            project_path: "/test/project/path".to_string(),
            raw_json: Some("{\"test\": \"json\"}".to_string()),
        }
    }

    fn create_long_result() -> SearchResult {
        let mut result = create_sample_result();
        result.text = (0..100)
            .map(|i| format!("Line {}: This is a long line of text to test scrolling behavior", i))
            .collect::<Vec<_>>()
            .join("\n");
        result
    }

    #[test]
    fn test_result_detail_creation() {
        let result_detail = ResultDetail::new();
        assert!(result_detail.get_result().is_none());
    }

    #[test]
    fn test_set_result() {
        let mut result_detail = ResultDetail::new();
        let result = create_sample_result();
        
        result_detail.set_result(result.clone());
        assert!(result_detail.get_result().is_some());
        assert_eq!(result_detail.get_result().unwrap().uuid, "test-uuid-1234");
    }

    #[test]
    fn test_clear() {
        let mut result_detail = ResultDetail::new();
        result_detail.set_result(create_sample_result());
        result_detail.set_message(Some("Test message".to_string()));
        
        result_detail.clear();
        assert!(result_detail.get_result().is_none());
        // Message should also be cleared
    }

    #[test]
    fn test_set_message() {
        let mut result_detail = ResultDetail::new();
        
        result_detail.set_message(Some("Test message".to_string()));
        // Message is internal state, tested through rendering
        
        result_detail.set_message(None);
        // Message cleared
    }

    #[test]
    fn test_scroll_navigation() {
        let mut result_detail = ResultDetail::new();
        result_detail.set_result(create_long_result());
        
        // Scroll down with Down arrow
        let msg = result_detail.on(create_key_event(Key::Down));
        assert_eq!(msg, None); // Scrolling doesn't generate messages
        
        // Scroll down with 'j'
        let msg = result_detail.on(create_key_event(Key::Char('j')));
        assert_eq!(msg, None);
        
        // Scroll up with Up arrow
        let msg = result_detail.on(create_key_event(Key::Up));
        assert_eq!(msg, None);
        
        // Scroll up with 'k'
        let msg = result_detail.on(create_key_event(Key::Char('k')));
        assert_eq!(msg, None);
    }

    #[test]
    fn test_page_navigation() {
        let mut result_detail = ResultDetail::new();
        result_detail.set_result(create_long_result());
        
        // Page down
        let msg = result_detail.on(create_key_event(Key::PageDown));
        assert_eq!(msg, None);
        
        // Page up
        let msg = result_detail.on(create_key_event(Key::PageUp));
        assert_eq!(msg, None);
    }

    #[test]
    fn test_action_keys() {
        let mut result_detail = ResultDetail::new();
        let result = create_sample_result();
        result_detail.set_result(result.clone());
        
        // Test 's' key - Enter session viewer
        let msg = result_detail.on(create_key_event(Key::Char('s')));
        assert_eq!(msg, Some(AppMessage::EnterSessionViewer("test-session-id".to_string())));
        
        // Test 'S' key - Enter session viewer (uppercase)
        let msg = result_detail.on(create_key_event(Key::Char('S')));
        assert_eq!(msg, Some(AppMessage::EnterSessionViewer("test-session-id".to_string())));
        
        // Test 'f' key - Copy file path
        let msg = result_detail.on(create_key_event(Key::Char('f')));
        assert_eq!(msg, Some(AppMessage::CopyToClipboard("test_file.jsonl".to_string())));
        
        // Test 'i' key - Copy session ID
        let msg = result_detail.on(create_key_event(Key::Char('i')));
        assert_eq!(msg, Some(AppMessage::CopyToClipboard("test-session-id".to_string())));
        
        // Test 'p' key - Copy project path
        let msg = result_detail.on(create_key_event(Key::Char('p')));
        assert_eq!(msg, Some(AppMessage::CopyToClipboard("/test/project/path".to_string())));
        
        // Test 'm' key - Copy message text
        let msg = result_detail.on(create_key_event(Key::Char('m')));
        assert_eq!(msg, Some(AppMessage::CopyToClipboard(result.text.clone())));
        
        // Test 'r' key - Copy raw JSON
        let msg = result_detail.on(create_key_event(Key::Char('r')));
        assert_eq!(msg, Some(AppMessage::CopyToClipboard("{\"test\": \"json\"}".to_string())));
        
        // Test 'c' key - Copy message text (alias)
        let msg = result_detail.on(create_key_event(Key::Char('c')));
        assert_eq!(msg, Some(AppMessage::CopyToClipboard(result.text.clone())));
    }

    #[test]
    fn test_copy_raw_json_fallback() {
        let mut result_detail = ResultDetail::new();
        let mut result = create_sample_result();
        result.raw_json = None; // No raw JSON
        result_detail.set_result(result.clone());
        
        // Test 'r' key - Should generate formatted text
        let msg = result_detail.on(create_key_event(Key::Char('r')));
        if let Some(AppMessage::CopyToClipboard(text)) = msg {
            assert!(text.contains("File: test_file.jsonl"));
            assert!(text.contains("UUID: test-uuid-1234"));
            assert!(text.contains("Session ID: test-session-id"));
        } else {
            panic!("Expected CopyToClipboard message");
        }
    }

    #[test]
    fn test_esc_key() {
        let mut result_detail = ResultDetail::new();
        result_detail.set_result(create_sample_result());
        
        let msg = result_detail.on(create_key_event(Key::Esc));
        assert_eq!(msg, Some(AppMessage::ExitResultDetail));
    }

    #[test]
    fn test_action_keys_without_result() {
        let mut result_detail = ResultDetail::new();
        // No result set
        
        // All action keys should return None
        assert_eq!(result_detail.on(create_key_event(Key::Char('s'))), None);
        assert_eq!(result_detail.on(create_key_event(Key::Char('f'))), None);
        assert_eq!(result_detail.on(create_key_event(Key::Char('i'))), None);
        assert_eq!(result_detail.on(create_key_event(Key::Char('p'))), None);
        assert_eq!(result_detail.on(create_key_event(Key::Char('m'))), None);
        assert_eq!(result_detail.on(create_key_event(Key::Char('r'))), None);
        assert_eq!(result_detail.on(create_key_event(Key::Char('c'))), None);
    }

    #[test]
    fn test_state() {
        let mut result_detail = ResultDetail::new();
        result_detail.set_result(create_long_result());
        
        // Initial state should be scroll offset 0
        match result_detail.state() {
            State::One(StateValue::Usize(offset)) => assert_eq!(offset, 0),
            _ => panic!("Unexpected state type"),
        }
        
        // Scroll down and check state
        result_detail.on(create_key_event(Key::Down));
        match result_detail.state() {
            State::One(StateValue::Usize(offset)) => assert!(offset > 0),
            _ => panic!("Unexpected state type"),
        }
    }

    #[test]
    fn test_uppercase_action_keys() {
        let mut result_detail = ResultDetail::new();
        result_detail.set_result(create_sample_result());
        
        // Test uppercase variants of action keys
        assert!(matches!(
            result_detail.on(create_key_event(Key::Char('F'))),
            Some(AppMessage::CopyToClipboard(_))
        ));
        assert!(matches!(
            result_detail.on(create_key_event(Key::Char('I'))),
            Some(AppMessage::CopyToClipboard(_))
        ));
        assert!(matches!(
            result_detail.on(create_key_event(Key::Char('P'))),
            Some(AppMessage::CopyToClipboard(_))
        ));
        assert!(matches!(
            result_detail.on(create_key_event(Key::Char('M'))),
            Some(AppMessage::CopyToClipboard(_))
        ));
        assert!(matches!(
            result_detail.on(create_key_event(Key::Char('R'))),
            Some(AppMessage::CopyToClipboard(_))
        ));
        assert!(matches!(
            result_detail.on(create_key_event(Key::Char('C'))),
            Some(AppMessage::CopyToClipboard(_))
        ));
    }
}