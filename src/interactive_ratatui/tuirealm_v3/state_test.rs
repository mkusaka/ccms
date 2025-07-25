#[cfg(test)]
mod state_tests {
    use super::super::*;
    use crate::query::condition::{QueryCondition, SearchResult};

    fn create_test_search_result() -> SearchResult {
        SearchResult {
            file: "test.jsonl".to_string(),
            uuid: "test-uuid".to_string(),
            timestamp: "2024-01-01T00:00:00Z".to_string(),
            session_id: "test-session".to_string(),
            role: "User".to_string(),
            text: "Test message".to_string(),
            has_tools: false,
            has_thinking: false,
            message_type: "message".to_string(),
            query: QueryCondition::Literal {
                pattern: "test".to_string(),
                case_sensitive: false,
            },
            project_path: "/test/project".to_string(),
            raw_json: Some(r#"{"content": "Test message"}"#.to_string()),
        }
    }

    #[test]
    fn test_app_state_new() {
        let state = AppState::new();
        
        assert_eq!(state.mode, AppMode::Search);
        assert_eq!(state.previous_mode, None);
        assert_eq!(state.search_query, "");
        assert!(!state.is_searching);
        assert!(state.search_results.is_empty());
        assert_eq!(state.selected_index, 0);
        assert_eq!(state.role_filter, None);
        assert!(state.session_messages.is_empty());
        assert!(state.session_filtered_indices.is_empty());
        assert_eq!(state.session_query, "");
        assert!(!state.is_session_searching);
        assert_eq!(state.session_order, None);
        assert_eq!(state.session_id, None);
        assert_eq!(state.session_scroll_offset, 0);
        assert_eq!(state.current_result, None);
        assert_eq!(state.detail_scroll_offset, 0);
        assert!(state.truncation_enabled);
        assert_eq!(state.status_message, None);
        assert!(!state.should_quit);
    }

    #[test]
    fn test_change_mode() {
        let mut state = AppState::new();
        
        state.change_mode(AppMode::Help);
        assert_eq!(state.mode, AppMode::Help);
        assert_eq!(state.previous_mode, Some(AppMode::Search));
        
        state.change_mode(AppMode::ResultDetail);
        assert_eq!(state.mode, AppMode::ResultDetail);
        assert_eq!(state.previous_mode, Some(AppMode::Help));
    }

    #[test]
    fn test_return_to_previous_mode() {
        let mut state = AppState::new();
        
        // Change to Help
        state.change_mode(AppMode::Help);
        assert_eq!(state.mode, AppMode::Help);
        assert_eq!(state.previous_mode, Some(AppMode::Search));
        
        // Return to previous
        state.return_to_previous_mode();
        assert_eq!(state.mode, AppMode::Search);
        assert_eq!(state.previous_mode, None);
        
        // Should do nothing when no previous mode
        state.return_to_previous_mode();
        assert_eq!(state.mode, AppMode::Search);
        assert_eq!(state.previous_mode, None);
    }


    #[test]
    fn test_set_message() {
        let mut state = AppState::new();
        
        state.set_message("Test message".to_string());
        assert_eq!(state.status_message, Some("Test message".to_string()));
    }

    #[test]
    fn test_clear_message() {
        let mut state = AppState::new();
        state.status_message = Some("Test".to_string());
        
        state.clear_message();
        assert_eq!(state.status_message, None);
    }

    #[test]
    fn test_cycle_role_filter() {
        let mut state = AppState::new();
        
        // None -> User
        state.cycle_role_filter();
        assert_eq!(state.role_filter, Some("User".to_string()));
        
        // User -> Assistant
        state.cycle_role_filter();
        assert_eq!(state.role_filter, Some("Assistant".to_string()));
        
        // Assistant -> System
        state.cycle_role_filter();
        assert_eq!(state.role_filter, Some("System".to_string()));
        
        // System -> None
        state.cycle_role_filter();
        assert_eq!(state.role_filter, None);
    }

    #[test]
    fn test_state_transitions() {
        let mut state = AppState::new();
        
        // Simulate search workflow
        state.search_query = "test".to_string();
        state.is_searching = true;
        
        // Search completes
        state.search_results = vec![create_test_search_result()];
        state.is_searching = false;
        state.selected_index = 0;
        
        // Enter result detail
        state.current_result = Some(state.search_results[0].clone());
        state.change_mode(AppMode::ResultDetail);
        
        assert_eq!(state.mode, AppMode::ResultDetail);
        assert!(state.current_result.is_some());
        
        // Return to search
        state.return_to_previous_mode();
        assert_eq!(state.mode, AppMode::Search);
    }

    #[test]
    fn test_session_state_management() {
        let mut state = AppState::new();
        
        // Set up session data
        state.session_id = Some("session-123".to_string());
        state.session_messages = vec![
            "Message 1".to_string(),
            "Message 2".to_string(),
            "Message 3".to_string(),
        ];
        state.session_filtered_indices = vec![0, 1, 2];
        
        // Test session search
        state.is_session_searching = true;
        state.session_query = "Message 2".to_string();
        
        // Simulate filtering
        state.session_filtered_indices = vec![1];
        state.selected_index = 0;
        
        assert_eq!(state.session_filtered_indices.len(), 1);
        assert_eq!(state.session_filtered_indices[0], 1);
    }

    #[test]
    fn test_scroll_offset_management() {
        let mut state = AppState::new();
        
        // Detail scroll
        state.detail_scroll_offset = 0;
        for _ in 0..5 {
            state.detail_scroll_offset += 1;
        }
        assert_eq!(state.detail_scroll_offset, 5);
        
        // Session scroll
        state.session_scroll_offset = 0;
        for _ in 0..10 {
            state.session_scroll_offset += 1;
        }
        assert_eq!(state.session_scroll_offset, 10);
    }
}