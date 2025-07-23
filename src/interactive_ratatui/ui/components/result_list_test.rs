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
        assert!(matches!(msg, Some(Message::SelectResult(_))));
        assert_eq!(list.selected_result().unwrap().text, "Second");

        // Move down again
        let msg = list.handle_key(create_key_event(KeyCode::Down));
        assert!(matches!(msg, Some(Message::SelectResult(_))));
        assert_eq!(list.selected_result().unwrap().text, "Third");

        // Can't move down from last item
        let msg = list.handle_key(create_key_event(KeyCode::Down));
        assert!(msg.is_none());

        // Move up
        let msg = list.handle_key(create_key_event(KeyCode::Up));
        assert!(matches!(msg, Some(Message::SelectResult(_))));
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
        assert!(matches!(msg, Some(Message::SelectResult(_))));

        // Page up
        let msg = list.handle_key(create_key_event(KeyCode::PageUp));
        assert!(matches!(msg, Some(Message::SelectResult(_))));
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
        assert!(matches!(msg, Some(Message::SelectResult(_))));
        assert_eq!(list.selected_result().unwrap().text, "Last");

        // Go to home
        let msg = list.handle_key(create_key_event(KeyCode::Home));
        assert!(matches!(msg, Some(Message::SelectResult(_))));
        assert_eq!(list.selected_result().unwrap().text, "First");
    }

    #[test]
    fn test_enter_and_s_keys() {
        let mut list = ResultList::new();
        let results = vec![create_test_result("user", "Test")];
        list.update_results(results, 0);

        // Enter should open detail view
        let msg = list.handle_key(create_key_event(KeyCode::Enter));
        assert!(matches!(msg, Some(Message::EnterResultDetail)));

        // 's' should open session viewer
        let msg = list.handle_key(create_key_event(KeyCode::Char('s')));
        assert!(matches!(msg, Some(Message::EnterSessionViewer)));
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
}
