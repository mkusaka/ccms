#[cfg(test)]
mod tests {
    use super::super::result_list::ResultList;
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

    fn create_sample_results(count: usize) -> Vec<SearchResult> {
        (0..count)
            .map(|i| SearchResult {
                file: format!("file_{}.jsonl", i),
                uuid: format!("uuid_{}", i),
                timestamp: "2024-01-01T00:00:00Z".to_string(),
                session_id: format!("session_{}", i),
                role: if i % 2 == 0 { "user" } else { "assistant" }.to_string(),
                text: format!("This is sample text for result {}", i),
                has_tools: false,
                has_thinking: false,
                message_type: "message".to_string(),
                query: QueryCondition::Literal {
                    pattern: "test".to_string(),
                    case_sensitive: false,
                },
                project_path: "/test/path".to_string(),
                raw_json: None,
            })
            .collect()
    }

    #[test]
    fn test_result_list_creation() {
        let result_list = ResultList::new();
        assert!(result_list.is_empty());
        assert_eq!(result_list.len(), 0);
        assert_eq!(result_list.selected_index(), 0);
    }

    #[test]
    fn test_set_results() {
        let mut result_list = ResultList::new();
        let results = create_sample_results(5);
        
        result_list.set_results(results.clone());
        assert_eq!(result_list.len(), 5);
        assert!(!result_list.is_empty());
        assert_eq!(result_list.selected_index(), 0);
    }

    #[test]
    fn test_navigation_up_down() {
        let mut result_list = ResultList::new();
        result_list.set_results(create_sample_results(5));

        // Move down
        let msg = result_list.on(create_key_event(Key::Down));
        assert_eq!(msg, Some(AppMessage::SelectResult(1)));
        assert_eq!(result_list.selected_index(), 1);

        // Move down with 'j'
        let msg = result_list.on(create_key_event(Key::Char('j')));
        assert_eq!(msg, Some(AppMessage::SelectResult(2)));
        assert_eq!(result_list.selected_index(), 2);

        // Move up
        let msg = result_list.on(create_key_event(Key::Up));
        assert_eq!(msg, Some(AppMessage::SelectResult(1)));
        assert_eq!(result_list.selected_index(), 1);

        // Move up with 'k'
        let msg = result_list.on(create_key_event(Key::Char('k')));
        assert_eq!(msg, Some(AppMessage::SelectResult(0)));
        assert_eq!(result_list.selected_index(), 0);

        // Try to move up at the beginning
        let msg = result_list.on(create_key_event(Key::Up));
        assert_eq!(msg, None);
        assert_eq!(result_list.selected_index(), 0);
    }

    #[test]
    fn test_navigation_page_up_down() {
        let mut result_list = ResultList::new();
        result_list.set_results(create_sample_results(20));

        // Move to middle
        result_list.set_selected_index(10);

        // Page up
        let msg = result_list.on(create_key_event(Key::PageUp));
        assert_eq!(msg, Some(AppMessage::SelectResult(0)));
        assert_eq!(result_list.selected_index(), 0);

        // Page down
        let msg = result_list.on(create_key_event(Key::PageDown));
        assert_eq!(msg, Some(AppMessage::SelectResult(10)));
        assert_eq!(result_list.selected_index(), 10);

        // Page down again
        let msg = result_list.on(create_key_event(Key::PageDown));
        assert_eq!(msg, Some(AppMessage::SelectResult(19))); // Should cap at last item
        assert_eq!(result_list.selected_index(), 19);
    }

    #[test]
    fn test_navigation_home_end() {
        let mut result_list = ResultList::new();
        result_list.set_results(create_sample_results(10));
        result_list.set_selected_index(5);

        // Go to end
        let msg = result_list.on(create_key_event(Key::End));
        assert_eq!(msg, Some(AppMessage::SelectResult(9)));
        assert_eq!(result_list.selected_index(), 9);

        // Go to beginning
        let msg = result_list.on(create_key_event(Key::Home));
        assert_eq!(msg, Some(AppMessage::SelectResult(0)));
        assert_eq!(result_list.selected_index(), 0);

        // Try Home at beginning
        let msg = result_list.on(create_key_event(Key::Home));
        assert_eq!(msg, None);
        assert_eq!(result_list.selected_index(), 0);
    }

    #[test]
    fn test_enter_key() {
        let mut result_list = ResultList::new();
        result_list.set_results(create_sample_results(5));

        let msg = result_list.on(create_key_event(Key::Enter));
        assert_eq!(msg, Some(AppMessage::EnterResultDetail));
    }

    #[test]
    fn test_enter_key_empty_list() {
        let mut result_list = ResultList::new();

        let msg = result_list.on(create_key_event(Key::Enter));
        assert_eq!(msg, None);
    }

    #[test]
    fn test_set_selected_index() {
        let mut result_list = ResultList::new();
        result_list.set_results(create_sample_results(10));

        result_list.set_selected_index(5);
        assert_eq!(result_list.selected_index(), 5);

        // Test out of bounds
        result_list.set_selected_index(20);
        assert_eq!(result_list.selected_index(), 5); // Should not change
    }

    #[test]
    fn test_selected_result() {
        let mut result_list = ResultList::new();
        let results = create_sample_results(5);
        result_list.set_results(results.clone());

        result_list.set_selected_index(2);
        let selected = result_list.selected_result();
        assert!(selected.is_some());
        assert_eq!(selected.unwrap().text, "This is sample text for result 2");
    }

    #[test]
    fn test_update_results() {
        let mut result_list = ResultList::new();
        result_list.set_results(create_sample_results(5));
        
        let new_results = create_sample_results(3);
        result_list.update_results(new_results, 1);
        
        assert_eq!(result_list.len(), 3);
        assert_eq!(result_list.selected_index(), 1);
    }

    #[test]
    fn test_set_truncation_enabled() {
        let mut result_list = ResultList::new();
        result_list.set_results(create_sample_results(5));
        
        result_list.set_truncation_enabled(false);
        // Truncation state is internal, would be tested through rendering
        
        result_list.set_truncation_enabled(true);
        // Truncation state is internal, would be tested through rendering
    }

    #[test]
    fn test_state() {
        let mut result_list = ResultList::new();
        result_list.set_results(create_sample_results(5));
        result_list.set_selected_index(3);
        
        match result_list.state() {
            State::One(StateValue::Usize(idx)) => assert_eq!(idx, 3),
            _ => panic!("Unexpected state type"),
        }
    }

    #[test]
    fn test_boundary_navigation() {
        let mut result_list = ResultList::new();
        result_list.set_results(create_sample_results(3));

        // Go to end
        result_list.set_selected_index(2);

        // Try to move down at the end
        let msg = result_list.on(create_key_event(Key::Down));
        assert_eq!(msg, None);
        assert_eq!(result_list.selected_index(), 2);

        // Go to beginning
        result_list.set_selected_index(0);

        // Try to move up at the beginning
        let msg = result_list.on(create_key_event(Key::Up));
        assert_eq!(msg, None);
        assert_eq!(result_list.selected_index(), 0);
    }

    #[test]
    fn test_navigation_with_single_item() {
        let mut result_list = ResultList::new();
        result_list.set_results(create_sample_results(1));

        // All navigation should fail with single item
        assert_eq!(result_list.on(create_key_event(Key::Down)), None);
        assert_eq!(result_list.on(create_key_event(Key::Up)), None);
        assert_eq!(result_list.on(create_key_event(Key::PageDown)), None);
        assert_eq!(result_list.on(create_key_event(Key::PageUp)), None);
        assert_eq!(result_list.on(create_key_event(Key::Home)), None);
        assert_eq!(result_list.on(create_key_event(Key::End)), None);

        // But Enter should still work
        assert_eq!(result_list.on(create_key_event(Key::Enter)), Some(AppMessage::EnterResultDetail));
    }

    #[test]
    fn test_page_navigation_boundaries() {
        let mut result_list = ResultList::new();
        result_list.set_results(create_sample_results(15));

        // Start at position 5
        result_list.set_selected_index(5);

        // Page up (should go to 0 since 5-10 = -5)
        let msg = result_list.on(create_key_event(Key::PageUp));
        assert_eq!(msg, Some(AppMessage::SelectResult(0)));
        assert_eq!(result_list.selected_index(), 0);

        // Page down from 0 (should go to 10)
        let msg = result_list.on(create_key_event(Key::PageDown));
        assert_eq!(msg, Some(AppMessage::SelectResult(10)));
        assert_eq!(result_list.selected_index(), 10);

        // Page down from 10 (should go to 14, the last item)
        let msg = result_list.on(create_key_event(Key::PageDown));
        assert_eq!(msg, Some(AppMessage::SelectResult(14)));
        assert_eq!(result_list.selected_index(), 14);
    }
}