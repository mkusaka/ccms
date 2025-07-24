#[cfg(test)]
mod tests {
    use super::super::app::Model;
    use crate::interactive_ratatui::domain::models::{Mode, SessionOrder};
    use crate::interactive_ratatui::ui::tuirealm_components::messages::AppMessage;
    use crate::query::condition::{SearchOptions, SearchResult, QueryCondition};
    use tuirealm::Update;

    fn create_test_model() -> Model {
        let search_options = SearchOptions {
            max_results: Some(100),
            role: None,
            session_id: None,
            before: None,
            after: None,
            verbose: false,
            project_path: None,
        };
        
        Model::new_with_terminal(search_options, 100, false).expect("Failed to create model")
    }

    fn create_test_search_result() -> SearchResult {
        SearchResult {
            file: "test.jsonl".to_string(),
            uuid: "test-uuid-123".to_string(),
            timestamp: "2024-01-01T12:00:00Z".to_string(),
            session_id: "test_session".to_string(),
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
        }
    }

    #[test]
    fn test_model_creation() {
        let model = create_test_model();
        assert_eq!(model.mode, Mode::Search);
        assert!(model.mode_stack.is_empty());
        assert_eq!(model.search_state.query, "");
        assert!(model.search_state.results.is_empty());
        assert_eq!(model.search_state.selected_index, 0);
        assert!(!model.search_state.is_searching);
        assert_eq!(model.max_results, 100);
        assert!(!model.quit);
    }

    #[test]
    fn test_query_changed() {
        let mut model = create_test_model();
        let msg = model.update(Some(AppMessage::QueryChanged("test query".to_string())));
        assert_eq!(msg, None);
        assert_eq!(model.search_state.query, "test query");
        assert_eq!(model.ui_state.message, Some("typing...".to_string()));
    }

    #[test]
    fn test_search_completed() {
        let mut model = create_test_model();
        let results = vec![create_test_search_result()];
        let msg = model.update(Some(AppMessage::SearchCompleted(results.clone())));
        assert_eq!(msg, None);
        assert_eq!(model.search_state.results.len(), 1);
        assert!(!model.search_state.is_searching);
        assert_eq!(model.search_state.selected_index, 0);
        assert_eq!(model.ui_state.message, None);
    }

    #[test]
    fn test_navigate_up_down() {
        let mut model = create_test_model();
        // Add some results first
        let results = vec![create_test_search_result(), create_test_search_result()];
        model.update(Some(AppMessage::SearchCompleted(results)));
        
        // Navigate down
        let msg = model.update(Some(AppMessage::NavigateDown));
        assert_eq!(msg, None);
        assert_eq!(model.search_state.selected_index, 1);
        
        // Navigate up
        let msg = model.update(Some(AppMessage::NavigateUp));
        assert_eq!(msg, None);
        assert_eq!(model.search_state.selected_index, 0);
        
        // Try to navigate up when already at top
        let msg = model.update(Some(AppMessage::NavigateUp));
        assert_eq!(msg, None);
        assert_eq!(model.search_state.selected_index, 0);
    }

    #[test]
    fn test_enter_exit_result_detail() {
        let mut model = create_test_model();
        let results = vec![create_test_search_result()];
        model.update(Some(AppMessage::SearchCompleted(results)));
        
        // Enter result detail
        let msg = model.update(Some(AppMessage::EnterResultDetail));
        assert_eq!(msg, None);
        assert_eq!(model.mode, Mode::ResultDetail);
        assert_eq!(model.mode_stack.len(), 1);
        assert_eq!(model.mode_stack[0], Mode::Search);
        assert!(model.ui_state.selected_result.is_some());
        
        // Exit result detail
        let msg = model.update(Some(AppMessage::ExitResultDetail));
        assert_eq!(msg, None);
        assert_eq!(model.mode, Mode::Search);
        assert!(model.mode_stack.is_empty());
    }

    #[test]
    fn test_enter_exit_help() {
        let mut model = create_test_model();
        
        // Enter help
        let msg = model.update(Some(AppMessage::EnterHelp));
        assert_eq!(msg, None);
        assert_eq!(model.mode, Mode::Help);
        assert_eq!(model.mode_stack.len(), 1);
        assert_eq!(model.mode_stack[0], Mode::Search);
        
        // Exit help
        let msg = model.update(Some(AppMessage::ExitHelp));
        assert_eq!(msg, None);
        assert_eq!(model.mode, Mode::Search);
        assert!(model.mode_stack.is_empty());
    }

    #[test]
    fn test_toggle_role_filter() {
        let mut model = create_test_model();
        
        // First toggle - None to "user"
        let msg = model.update(Some(AppMessage::ToggleRoleFilter));
        assert_eq!(msg, Some(AppMessage::SearchRequested));
        assert_eq!(model.search_state.role_filter, Some("user".to_string()));
        
        // Second toggle - "user" to "assistant"
        let msg = model.update(Some(AppMessage::ToggleRoleFilter));
        assert_eq!(msg, Some(AppMessage::SearchRequested));
        assert_eq!(model.search_state.role_filter, Some("assistant".to_string()));
        
        // Third toggle - "assistant" to "system"
        let msg = model.update(Some(AppMessage::ToggleRoleFilter));
        assert_eq!(msg, Some(AppMessage::SearchRequested));
        assert_eq!(model.search_state.role_filter, Some("system".to_string()));
        
        // Fourth toggle - "system" back to None
        let msg = model.update(Some(AppMessage::ToggleRoleFilter));
        assert_eq!(msg, Some(AppMessage::SearchRequested));
        assert_eq!(model.search_state.role_filter, None);
    }

    #[test]
    fn test_toggle_truncation() {
        let mut model = create_test_model();
        assert!(model.ui_state.truncation_enabled);
        
        // Toggle off
        let msg = model.update(Some(AppMessage::ToggleTruncation));
        assert_eq!(msg, None);
        assert!(!model.ui_state.truncation_enabled);
        assert_eq!(model.ui_state.message, Some("Message display: Full Text".to_string()));
        
        // Toggle on
        let msg = model.update(Some(AppMessage::ToggleTruncation));
        assert_eq!(msg, None);
        assert!(model.ui_state.truncation_enabled);
        assert_eq!(model.ui_state.message, Some("Message display: Truncated".to_string()));
    }

    #[test]
    fn test_session_order_toggle() {
        let mut model = create_test_model();
        
        // First toggle - None to Ascending
        let msg = model.update(Some(AppMessage::ToggleSessionOrder));
        assert_eq!(msg, None);
        assert_eq!(model.session_state.order, Some(SessionOrder::Ascending));
        
        // Second toggle - Ascending to Descending
        let msg = model.update(Some(AppMessage::ToggleSessionOrder));
        assert_eq!(msg, None);
        assert_eq!(model.session_state.order, Some(SessionOrder::Descending));
        
        // Third toggle - Descending to Original
        let msg = model.update(Some(AppMessage::ToggleSessionOrder));
        assert_eq!(msg, None);
        assert_eq!(model.session_state.order, Some(SessionOrder::Original));
        
        // Fourth toggle - Original back to None
        let msg = model.update(Some(AppMessage::ToggleSessionOrder));
        assert_eq!(msg, None);
        assert_eq!(model.session_state.order, None);
    }

    #[test]
    fn test_status_messages() {
        let mut model = create_test_model();
        
        // Set status
        let msg = model.update(Some(AppMessage::SetStatus("Test status".to_string())));
        assert_eq!(msg, None);
        assert_eq!(model.ui_state.message, Some("Test status".to_string()));
        
        // Clear status
        let msg = model.update(Some(AppMessage::ClearStatus));
        assert_eq!(msg, None);
        assert_eq!(model.ui_state.message, None);
    }

    #[test]
    fn test_quit() {
        let mut model = create_test_model();
        assert!(!model.quit);
        
        let msg = model.update(Some(AppMessage::Quit));
        assert_eq!(msg, None);
        assert!(model.quit);
    }

    #[test]
    fn test_select_result() {
        let mut model = create_test_model();
        let results = vec![create_test_search_result(), create_test_search_result()];
        model.update(Some(AppMessage::SearchCompleted(results)));
        
        // Select valid index
        let msg = model.update(Some(AppMessage::SelectResult(1)));
        assert_eq!(msg, None);
        assert_eq!(model.search_state.selected_index, 1);
        
        // Try to select invalid index
        let msg = model.update(Some(AppMessage::SelectResult(10)));
        assert_eq!(msg, None);
        assert_eq!(model.search_state.selected_index, 1); // Should not change
    }
}