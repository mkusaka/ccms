#[cfg(test)]
mod tests {
    use super::super::Component;
    use super::super::result_list::*;
    use crate::interactive_ratatui::ui::events::Message;
    use crate::query::condition::{QueryCondition, SearchResult};
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    fn create_key_event(code: KeyCode) -> KeyEvent {
        KeyEvent {
            code,
            modifiers: KeyModifiers::empty(),
            kind: crossterm::event::KeyEventKind::Press,
            state: crossterm::event::KeyEventState::empty(),
        }
    }

    fn create_test_result(role: &str, text: &str) -> SearchResult {
        SearchResult {
            file: "test.jsonl".to_string(),
            uuid: "test-uuid".to_string(),
            timestamp: "2024-01-01T12:00:00Z".to_string(),
            session_id: "test-session".to_string(),
            role: role.to_string(),
            text: text.to_string(),
            has_tools: false,
            has_thinking: false,
            message_type: role.to_string(),
            query: QueryCondition::Literal {
                pattern: "test".to_string(),
                case_sensitive: false,
            },
            project_path: "/test".to_string(),
            raw_json: None,
        }
    }

    #[test]
    fn test_result_list_creation() {
        let list = ResultList::new();
        assert!(list.selected_result().is_none());
    }

    #[test]
    fn test_update_results() {
        let mut list = ResultList::new();
        let results = vec![
            create_test_result("user", "Hello"),
            create_test_result("assistant", "Hi there"),
        ];

        list.update_results(results.clone(), 0);

        assert!(list.selected_result().is_some());
        assert_eq!(list.selected_result().unwrap().text, "Hello");
    }

    #[test]
    fn test_navigation_up_down() {
        let mut list = ResultList::new();
        let results = vec![
            create_test_result("user", "First"),
            create_test_result("assistant", "Second"),
            create_test_result("user", "Third"),
        ];

        list.update_results(results, 0);

        // Initially at index 0
        assert_eq!(list.selected_result().unwrap().text, "First");

        // Move down
        let msg = list.handle_key(create_key_event(KeyCode::Down));
        assert!(matches!(msg, Some(Message::SelectResult(1))));
        list.update_selection(1);
        assert_eq!(list.selected_result().unwrap().text, "Second");

        // Move down again
        let msg = list.handle_key(create_key_event(KeyCode::Down));
        assert!(matches!(msg, Some(Message::SelectResult(2))));
        list.update_selection(2);
        assert_eq!(list.selected_result().unwrap().text, "Third");

        // Can't move down from last item
        let msg = list.handle_key(create_key_event(KeyCode::Down));
        assert!(msg.is_none());

        // Move up
        let msg = list.handle_key(create_key_event(KeyCode::Up));
        assert!(matches!(msg, Some(Message::SelectResult(1))));
        list.update_selection(1);
        assert_eq!(list.selected_result().unwrap().text, "Second");
    }

    #[test]
    fn test_page_navigation() {
        let mut list = ResultList::new();
        let mut results = vec![];
        for i in 0..20 {
            results.push(create_test_result("user", &format!("Message {i}")));
        }

        list.update_results(results, 0);

        // Page down
        let msg = list.handle_key(create_key_event(KeyCode::PageDown));
        assert!(matches!(msg, Some(Message::SelectResult(10))));
        list.update_selection(10);

        // Page up
        let msg = list.handle_key(create_key_event(KeyCode::PageUp));
        assert!(matches!(msg, Some(Message::SelectResult(0))));
        list.update_selection(0);
    }

    #[test]
    fn test_home_end_navigation() {
        let mut list = ResultList::new();
        let results = vec![
            create_test_result("user", "First"),
            create_test_result("assistant", "Middle"),
            create_test_result("user", "Last"),
        ];

        list.update_results(results, 1); // Start in middle

        // Go to end
        let msg = list.handle_key(create_key_event(KeyCode::End));
        assert!(matches!(msg, Some(Message::SelectResult(2))));
        list.update_selection(2);
        assert_eq!(list.selected_result().unwrap().text, "Last");

        // Go to home
        let msg = list.handle_key(create_key_event(KeyCode::Home));
        assert!(matches!(msg, Some(Message::SelectResult(0))));
        list.update_selection(0);
        assert_eq!(list.selected_result().unwrap().text, "First");
    }

    #[test]
    fn test_enter_key() {
        let mut list = ResultList::new();
        let results = vec![create_test_result("user", "Test")];
        list.update_results(results, 0);

        // Enter should open detail view
        let msg = list.handle_key(create_key_event(KeyCode::Enter));
        assert!(matches!(msg, Some(Message::EnterResultDetail)));
    }

    #[test]
    fn test_truncate_message() {
        // Test message truncation
        let short = "Hello";
        let truncated = ResultList::truncate_message(short, 10);
        assert_eq!(truncated, "Hello");

        let long = "This is a very long message that should be truncated";
        let truncated = ResultList::truncate_message(long, 20);
        assert_eq!(truncated, "This is a very lo...");

        // Test with newlines
        let multiline = "Line 1\nLine 2";
        let truncated = ResultList::truncate_message(multiline, 20);
        assert_eq!(truncated, "Line 1 Line 2");
    }

    #[test]
    fn test_format_timestamp() {
        let ts = "2024-01-15T14:30:45Z";
        let formatted = ResultList::format_timestamp(ts);
        assert_eq!(formatted, "01/15 14:30");

        // Invalid timestamp
        let invalid = "not a timestamp";
        let formatted = ResultList::format_timestamp(invalid);
        assert_eq!(formatted, "N/A");
    }

    #[test]
    fn test_empty_results() {
        let mut list = ResultList::new();
        list.update_results(vec![], 0);

        assert!(list.selected_result().is_none());

        // Navigation should do nothing
        let msg = list.handle_key(create_key_event(KeyCode::Down));
        assert!(msg.is_none());

        let msg = list.handle_key(create_key_event(KeyCode::Up));
        assert!(msg.is_none());
    }

    #[test]
    fn test_unicode_truncation() {
        // Test that unicode is handled correctly
        let japanese = "こんにちは世界、これは長いメッセージです";
        let truncated = ResultList::truncate_message(japanese, 10);
        assert_eq!(truncated, "こんにちは世界..."); // 7 chars + "..."

        let emoji = "🔍🎯💻🎨🔧 Search tool";
        let truncated = ResultList::truncate_message(emoji, 10);
        assert_eq!(truncated, "🔍🎯💻🎨🔧 S...");
    }

    #[test]
    fn test_wrap_text() {
        // Test basic wrapping
        let wrapped = ResultList::wrap_text("Hello world this is a test", 10);
        assert_eq!(wrapped, vec!["Hello", "world this", "is a test"]);

        // Test text that fits on one line
        let wrapped = ResultList::wrap_text("Short", 10);
        assert_eq!(wrapped, vec!["Short"]);

        // Test empty text
        let wrapped = ResultList::wrap_text("", 10);
        assert_eq!(wrapped, vec![""]);

        // Test very long word
        let wrapped = ResultList::wrap_text("superlongwordthatdoesntfit", 10);
        assert_eq!(wrapped, vec!["superlongwordthatdoesntfit"]);

        // Test multiple spaces
        let wrapped = ResultList::wrap_text("Hello    world", 20);
        assert_eq!(wrapped, vec!["Hello world"]);

        // Test zero width
        let wrapped = ResultList::wrap_text("Hello", 0);
        assert_eq!(wrapped, Vec::<String>::new());

        // Test unicode text
        let wrapped = ResultList::wrap_text("こんにちは 世界 です", 10);
        assert_eq!(wrapped, vec!["こんにちは 世界", "です"]);
    }
}
