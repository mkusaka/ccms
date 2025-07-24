#[cfg(test)]
mod tests {
    use crate::query::condition::SearchOptions;
    use crate::interactive_ratatui::ui::tuirealm_components::app::Model;
    use crate::interactive_ratatui::ui::tuirealm_components::messages::AppMessage;
    use crate::interactive_ratatui::domain::models::Mode;
    use tuirealm::Update;

    #[test]
    fn test_tuirealm_integration_basic_flow() {
        // Create model without terminal for testing
        let options = SearchOptions::default();
        let mut model = Model::new_with_terminal(options, 100, false).expect("Failed to create model");
        
        // Initial state
        assert_eq!(model.mode, Mode::Search);
        assert_eq!(model.search_state.query, "");
        
        // Update query
        model.update(Some(AppMessage::QueryChanged("test query".to_string())));
        assert_eq!(model.search_state.query, "test query");
        
        // Trigger search (search executes synchronously in tests)
        model.update(Some(AppMessage::SearchRequested));
        // After synchronous search, is_searching should be false
        assert!(!model.search_state.is_searching);
        
        // Toggle role filter
        let msg = model.update(Some(AppMessage::ToggleRoleFilter));
        assert_eq!(msg, Some(AppMessage::SearchRequested));
        assert_eq!(model.search_state.role_filter, Some("user".to_string()));
        
        // Enter help
        model.update(Some(AppMessage::EnterHelp));
        assert_eq!(model.mode, Mode::Help);
        
        // Exit help
        model.update(Some(AppMessage::ExitHelp));
        assert_eq!(model.mode, Mode::Search);
        
        // Quit
        model.update(Some(AppMessage::Quit));
        assert!(model.quit);
    }

    #[test]
    fn test_tuirealm_navigation_flow() {
        use crate::query::condition::{SearchResult, QueryCondition};
        
        let options = SearchOptions::default();
        let mut model = Model::new_with_terminal(options, 100, false).expect("Failed to create model");
        
        // Add some search results
        let results = vec![
            SearchResult {
                file: "test1.jsonl".to_string(),
                uuid: "uuid1".to_string(),
                timestamp: "2024-01-01T12:00:00Z".to_string(),
                session_id: "session1".to_string(),
                role: "user".to_string(),
                text: "Test message 1".to_string(),
                has_tools: false,
                has_thinking: false,
                message_type: "message".to_string(),
                query: QueryCondition::Literal {
                    pattern: "test".to_string(),
                    case_sensitive: false,
                },
                project_path: "/test/path".to_string(),
                raw_json: None,
            },
            SearchResult {
                file: "test2.jsonl".to_string(),
                uuid: "uuid2".to_string(),
                timestamp: "2024-01-01T13:00:00Z".to_string(),
                session_id: "session2".to_string(),
                role: "assistant".to_string(),
                text: "Test message 2".to_string(),
                has_tools: false,
                has_thinking: false,
                message_type: "message".to_string(),
                query: QueryCondition::Literal {
                    pattern: "test".to_string(),
                    case_sensitive: false,
                },
                project_path: "/test/path".to_string(),
                raw_json: None,
            },
        ];
        
        model.update(Some(AppMessage::SearchCompleted(results)));
        assert_eq!(model.search_state.results.len(), 2);
        assert_eq!(model.search_state.selected_index, 0);
        
        // Navigate down
        model.update(Some(AppMessage::NavigateDown));
        assert_eq!(model.search_state.selected_index, 1);
        
        // Navigate up
        model.update(Some(AppMessage::NavigateUp));
        assert_eq!(model.search_state.selected_index, 0);
        
        // Enter result detail
        model.update(Some(AppMessage::EnterResultDetail));
        assert_eq!(model.mode, Mode::ResultDetail);
        assert!(model.ui_state.selected_result.is_some());
        
        // Exit result detail
        model.update(Some(AppMessage::ExitResultDetail));
        assert_eq!(model.mode, Mode::Search);
    }

    #[test]
    fn test_tuirealm_session_viewer_flow() {
        use crate::query::condition::{SearchResult, QueryCondition};
        
        let options = SearchOptions::default();
        let mut model = Model::new_with_terminal(options, 100, false).expect("Failed to create model");
        
        // Add a search result
        let result = SearchResult {
            file: "test.jsonl".to_string(),
            uuid: "uuid1".to_string(),
            timestamp: "2024-01-01T12:00:00Z".to_string(),
            session_id: "session1".to_string(),
            role: "user".to_string(),
            text: "Test message".to_string(),
            has_tools: false,
            has_thinking: false,
            message_type: "message".to_string(),
            query: QueryCondition::Literal {
                pattern: "test".to_string(),
                case_sensitive: false,
            },
            project_path: "/test/path".to_string(),
            raw_json: None,
        };
        
        model.update(Some(AppMessage::SearchCompleted(vec![result])));
        
        // Enter session viewer
        model.update(Some(AppMessage::EnterSessionViewer("session1".to_string())));
        assert_eq!(model.mode, Mode::SessionViewer);
        assert_eq!(model.session_state.session_id, Some("session1".to_string()));
        
        // Update session query
        model.update(Some(AppMessage::SessionQueryChanged("filter".to_string())));
        assert_eq!(model.session_state.query, "filter");
        
        // Toggle session order
        model.update(Some(AppMessage::ToggleSessionOrder));
        assert_eq!(model.session_state.order, Some(crate::interactive_ratatui::domain::models::SessionOrder::Ascending));
        
        // Exit session viewer
        model.update(Some(AppMessage::ExitSessionViewer));
        assert_eq!(model.mode, Mode::Search);
    }
}