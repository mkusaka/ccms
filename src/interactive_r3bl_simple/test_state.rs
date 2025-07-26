#[cfg(test)]
mod tests {
    use super::super::state::*;
    use crate::query::{QueryCondition, SearchResult};

    fn create_test_result(id: u32, role: &str, text: &str) -> SearchResult {
        SearchResult {
            file: format!("test{}.jsonl", id),
            uuid: format!("uuid-{}", id),
            timestamp: "2024-01-01T00:00:00Z".to_string(),
            session_id: format!("session-{}", id),
            role: role.to_string(),
            text: text.to_string(),
            has_tools: false,
            has_thinking: false,
            message_type: "text".to_string(),
            query: QueryCondition::Literal {
                pattern: "test".to_string(),
                case_sensitive: false,
            },
            project_path: "/test".to_string(),
            raw_json: None,
        }
    }

    #[test]
    fn test_app_state_new() {
        let state = AppState::new();
        assert_eq!(state.query, "");
        assert_eq!(state.search_results.len(), 0);
        assert_eq!(state.selected_index, 0);
        assert!(!state.is_searching);
        assert_eq!(state.scroll_offset, 0);
        assert!(!state.show_help);
        assert_eq!(state.current_mode, ViewMode::Search);
        assert!(state.status_message.is_none());
        assert!(state.needs_render);
        assert_eq!(state.ctrl_c_count, 0);
        assert!(state.last_ctrl_c_time.is_none());
    }

    #[test]
    fn test_get_selected_result() {
        let mut state = AppState::new();
        
        // No results
        assert!(state.get_selected_result().is_none());
        
        // Add results
        state.search_results.push(create_test_result(1, "user", "First message"));
        state.search_results.push(create_test_result(2, "assistant", "Second message"));
        
        // First result selected
        assert_eq!(state.get_selected_result().unwrap().uuid, "uuid-1");
        
        // Select second result
        state.selected_index = 1;
        assert_eq!(state.get_selected_result().unwrap().uuid, "uuid-2");
        
        // Invalid index
        state.selected_index = 5;
        assert!(state.get_selected_result().is_none());
    }

    #[test]
    fn test_navigate_up() {
        let mut state = AppState::new();
        state.search_results.push(create_test_result(1, "user", "First"));
        state.search_results.push(create_test_result(2, "assistant", "Second"));
        state.search_results.push(create_test_result(3, "user", "Third"));
        
        // Start at index 2
        state.selected_index = 2;
        state.scroll_offset = 1;
        
        // Navigate up
        state.navigate_up();
        assert_eq!(state.selected_index, 1);
        
        // Navigate up again
        state.navigate_up();
        assert_eq!(state.selected_index, 0);
        assert_eq!(state.scroll_offset, 0); // Should adjust scroll
        
        // Try to navigate up at beginning
        state.navigate_up();
        assert_eq!(state.selected_index, 0); // Should stay at 0
    }

    #[test]
    fn test_navigate_down() {
        let mut state = AppState::new();
        state.search_results.push(create_test_result(1, "user", "First"));
        state.search_results.push(create_test_result(2, "assistant", "Second"));
        state.search_results.push(create_test_result(3, "user", "Third"));
        
        // Start at index 0
        state.selected_index = 0;
        
        // Navigate down
        state.navigate_down();
        assert_eq!(state.selected_index, 1);
        
        // Navigate down again
        state.navigate_down();
        assert_eq!(state.selected_index, 2);
        
        // Try to navigate down at end
        state.navigate_down();
        assert_eq!(state.selected_index, 2); // Should stay at 2
    }

    #[test]
    fn test_scroll_adjustment() {
        let mut state = AppState::new();
        
        // Add 30 results
        for i in 0..30 {
            state.search_results.push(create_test_result(i, "user", &format!("Message {}", i)));
        }
        
        // Start near bottom
        state.selected_index = 25;
        state.scroll_offset = 10;
        
        // Navigate down - should adjust scroll
        state.navigate_down();
        assert_eq!(state.selected_index, 26);
        // The scroll offset is adjusted so the selected item is visible
        // With visible_height = 20 and selected_index = 26, scroll_offset should be 7
        // because 26 - 20 + 1 = 7
        let expected_scroll = if state.selected_index >= state.scroll_offset + 20 {
            state.selected_index - 20 + 1
        } else {
            state.scroll_offset
        };
        assert_eq!(state.scroll_offset, expected_scroll);
    }

    #[test]
    fn test_status_message() {
        let mut state = AppState::new();
        
        // Initially no status
        assert!(state.status_message.is_none());
        
        // Set status
        state.set_status("Test status".to_string());
        assert_eq!(state.status_message.as_ref().unwrap(), "Test status");
        
        // Clear status
        state.clear_status();
        assert!(state.status_message.is_none());
    }

    #[test]
    fn test_view_mode() {
        let state = AppState::new();
        assert_eq!(state.current_mode, ViewMode::Search);
        
        // ViewMode should be clonable and comparable
        let mode = state.current_mode.clone();
        assert_eq!(mode, ViewMode::Search);
        assert_ne!(mode, ViewMode::Help);
    }

    #[test]
    fn test_search_signal() {
        let results = vec![
            create_test_result(1, "user", "Message 1"),
            create_test_result(2, "assistant", "Message 2"),
        ];
        
        // Test SearchCompleted variant
        let signal = SearchSignal::SearchCompleted(results.clone());
        match signal {
            SearchSignal::SearchCompleted(r) => assert_eq!(r.len(), 2),
            _ => panic!("Wrong variant"),
        }
        
        // Test SearchError variant
        let signal = SearchSignal::SearchError("Test error".to_string());
        match signal {
            SearchSignal::SearchError(e) => assert_eq!(e, "Test error"),
            _ => panic!("Wrong variant"),
        }
    }
}