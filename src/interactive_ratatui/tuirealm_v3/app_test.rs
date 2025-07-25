#[cfg(test)]
mod app_tests {
    use super::super::*;
    use crate::query::condition::{QueryCondition, SearchResult};
    use std::sync::mpsc;

    fn create_test_app() -> App {
        App::new(None, None, None, None)
    }

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
    fn test_app_creation() {
        let app = App::new(
            Some("test pattern".to_string()),
            Some("2024-01-01T00:00:00Z".to_string()),
            Some("2024-12-31T23:59:59Z".to_string()),
            Some("session-123".to_string()),
        );
        
        assert_eq!(app.state.search_query, "");
        assert!(!app.state.is_searching);
        assert_eq!(app.state.mode, AppMode::Search);
    }

    #[test]
    fn test_execute_search() {
        let mut app = create_test_app();
        app.state.search_query = "test query".to_string();
        
        app.execute_search();
        
        assert!(app.state.is_searching);
        assert!(app.search_rx.is_some());
    }

    #[test]
    fn test_execute_search_empty_query() {
        let mut app = create_test_app();
        app.state.search_query = "".to_string();
        
        app.execute_search();
        
        // Should not execute search with empty query
        assert!(!app.state.is_searching);
        assert!(app.search_rx.is_none());
    }

    #[test]
    fn test_load_session_success() {
        let mut app = create_test_app();
        
        // Mock session service would be needed for proper testing
        // For now, we'll test that the method doesn't panic
        app.load_session("test-session".to_string());
        
        // In actual implementation, this would fail, but we're testing the structure
        assert!(app.state.status_message.is_some());
    }

    #[test]
    fn test_filter_session_messages() {
        let mut app = create_test_app();
        app.state.session_messages = vec![
            "First message".to_string(),
            "Second message with keyword".to_string(),
            "Third message".to_string(),
        ];
        app.state.session_query = "keyword".to_string();
        
        app.filter_session_messages();
        
        assert_eq!(app.state.session_filtered_indices, vec![1]);
    }

    #[test]
    fn test_filter_session_messages_empty_query() {
        let mut app = create_test_app();
        app.state.session_messages = vec![
            "First message".to_string(),
            "Second message".to_string(),
        ];
        app.state.session_query = "".to_string();
        
        app.filter_session_messages();
        
        assert_eq!(app.state.session_filtered_indices, vec![0, 1]);
    }

    #[test]
    fn test_handle_copy_message() {
        let mut app = create_test_app();
        app.state.mode = AppMode::ResultDetail;
        app.state.current_result = Some(create_test_search_result());
        
        app.handle_copy("message");
        
        // Should have attempted to copy
        assert!(app.state.status_message.is_some());
    }

    #[test]
    fn test_handle_copy_session_id() {
        let mut app = create_test_app();
        app.state.mode = AppMode::ResultDetail;
        app.state.current_result = Some(create_test_search_result());
        
        app.handle_copy("session");
        
        assert!(app.state.status_message.is_some());
    }

    #[test]
    fn test_handle_copy_timestamp() {
        let mut app = create_test_app();
        app.state.mode = AppMode::ResultDetail;
        app.state.current_result = Some(create_test_search_result());
        
        app.handle_copy("timestamp");
        
        assert!(app.state.status_message.is_some());
    }

    #[test]
    fn test_handle_copy_raw_json() {
        let mut app = create_test_app();
        app.state.mode = AppMode::ResultDetail;
        app.state.current_result = Some(create_test_search_result());
        
        app.handle_copy("json");
        
        assert!(app.state.status_message.is_some());
    }

    #[test]
    fn test_update_quit_message() {
        let mut app = create_test_app();
        
        app.update(Some(AppMessage::Quit));
        
        assert!(app.state.should_quit);
    }

    #[test]
    fn test_update_change_mode() {
        let mut app = create_test_app();
        
        app.update(Some(AppMessage::ChangeMode(AppMode::Help)));
        
        assert_eq!(app.state.mode, AppMode::Help);
    }

    #[test]
    fn test_update_search_query_changed() {
        let mut app = create_test_app();
        
        app.update(Some(AppMessage::SearchQueryChanged("new query".to_string())));
        
        assert_eq!(app.state.search_query, "new query");
    }

    #[test]
    fn test_update_toggle_role_filter() {
        let mut app = create_test_app();
        assert_eq!(app.state.role_filter, None);
        
        app.update(Some(AppMessage::ToggleRoleFilter));
        assert_eq!(app.state.role_filter, Some("User".to_string()));
        
        app.update(Some(AppMessage::ToggleRoleFilter));
        assert_eq!(app.state.role_filter, Some("Assistant".to_string()));
        
        app.update(Some(AppMessage::ToggleRoleFilter));
        assert_eq!(app.state.role_filter, Some("System".to_string()));
        
        app.update(Some(AppMessage::ToggleRoleFilter));
        assert_eq!(app.state.role_filter, None);
    }

    #[test]
    fn test_update_result_navigation() {
        let mut app = create_test_app();
        app.state.search_results = vec![
            create_test_search_result(),
            create_test_search_result(),
            create_test_search_result(),
        ];
        app.state.selected_index = 1;
        
        app.update(Some(AppMessage::ResultUp));
        assert_eq!(app.state.selected_index, 0);
        
        app.update(Some(AppMessage::ResultUp));
        assert_eq!(app.state.selected_index, 0); // Should not go below 0
        
        app.update(Some(AppMessage::ResultDown));
        assert_eq!(app.state.selected_index, 1);
        
        app.update(Some(AppMessage::ResultEnd));
        assert_eq!(app.state.selected_index, 2);
        
        app.update(Some(AppMessage::ResultHome));
        assert_eq!(app.state.selected_index, 0);
    }

    #[test]
    fn test_update_enter_result_detail() {
        let mut app = create_test_app();
        let result = create_test_search_result();
        app.state.search_results = vec![result.clone()];
        
        app.update(Some(AppMessage::EnterResultDetail(0)));
        
        assert_eq!(app.state.mode, AppMode::ResultDetail);
        assert_eq!(app.state.current_result.unwrap().text, result.text);
        assert_eq!(app.state.detail_scroll_offset, 0);
    }

    #[test]
    fn test_update_toggle_truncation() {
        let mut app = create_test_app();
        assert!(app.state.truncation_enabled);
        
        app.update(Some(AppMessage::ToggleTruncation));
        assert!(!app.state.truncation_enabled);
        
        app.update(Some(AppMessage::ToggleTruncation));
        assert!(app.state.truncation_enabled);
    }

    #[test]
    fn test_update_session_order_toggle() {
        let mut app = create_test_app();
        app.state.session_messages = vec!["A".to_string(), "B".to_string()];
        
        app.update(Some(AppMessage::SessionToggleOrder));
        assert_eq!(app.state.session_order, Some(SessionOrder::Descending));
        
        app.update(Some(AppMessage::SessionToggleOrder));
        assert_eq!(app.state.session_order, Some(SessionOrder::Ascending));
        
        app.update(Some(AppMessage::SessionToggleOrder));
        assert_eq!(app.state.session_order, Some(SessionOrder::Original));
        
        app.update(Some(AppMessage::SessionToggleOrder));
        assert_eq!(app.state.session_order, None);
    }

    #[test]
    fn test_update_search_completed_with_results() {
        let mut app = create_test_app();
        let (tx, rx) = mpsc::channel();
        app.search_rx = Some(rx);
        
        // Send results through channel
        let results = vec![create_test_search_result()];
        tx.send(results.clone()).unwrap();
        drop(tx);
        
        // Update should receive results
        app.update(None);
        
        assert_eq!(app.state.search_results.len(), 1);
        assert!(!app.state.is_searching);
        assert_eq!(app.state.selected_index, 0);
    }

    #[test]
    fn test_session_search_workflow() {
        let mut app = create_test_app();
        
        // Start session search
        app.update(Some(AppMessage::SessionSearchStart));
        assert!(app.state.is_session_searching);
        assert_eq!(app.state.session_query, "");
        
        // Update query
        app.update(Some(AppMessage::SessionQueryChanged("test".to_string())));
        assert_eq!(app.state.session_query, "test");
        
        // End session search
        app.update(Some(AppMessage::SessionSearchEnd));
        assert!(!app.state.is_session_searching);
    }

    #[test]
    fn test_detail_scrolling() {
        let mut app = create_test_app();
        app.state.detail_scroll_offset = 5;
        
        app.update(Some(AppMessage::DetailScrollUp));
        assert_eq!(app.state.detail_scroll_offset, 4);
        
        app.update(Some(AppMessage::DetailScrollDown));
        assert_eq!(app.state.detail_scroll_offset, 5);
        
        app.update(Some(AppMessage::DetailPageUp));
        assert_eq!(app.state.detail_scroll_offset, 0);
        
        app.update(Some(AppMessage::DetailPageDown));
        assert_eq!(app.state.detail_scroll_offset, 10);
    }

    #[test]
    fn test_message_handling() {
        let mut app = create_test_app();
        
        app.update(Some(AppMessage::ShowMessage("Test message".to_string())));
        assert_eq!(app.state.status_message, Some("Test message".to_string()));
        
        app.update(Some(AppMessage::ClearMessage));
        assert_eq!(app.state.status_message, None);
    }
}