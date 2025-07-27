#[cfg(test)]
mod tests {
    use crate::SearchOptions;
    use crate::interactive_ratatui::ui::app_state::{AppState, Mode};
    use crate::interactive_ratatui::ui::events::Message;

    #[test]
    fn test_help_dialog_navigation_from_search_mode() {
        let mut state = AppState::new(SearchOptions::default(), 100);
        assert_eq!(state.mode, Mode::Search);
        assert!(state.mode_stack.is_empty());

        // Show help from search mode
        state.update(Message::ShowHelp);
        assert_eq!(state.mode, Mode::Help);
        assert_eq!(state.mode_stack.len(), 1);
        assert_eq!(state.mode_stack[0], Mode::Search);

        // Close help should return to search mode
        state.update(Message::CloseHelp);
        assert_eq!(state.mode, Mode::Search);
        assert!(state.mode_stack.is_empty());
    }

    #[test]
    fn test_help_dialog_navigation_from_result_detail_mode() {
        let mut state = AppState::new(SearchOptions::default(), 100);

        // First navigate to result detail mode
        // (We need to set up a result first)
        state.search.results = vec![create_test_result()];
        state.update(Message::EnterResultDetail);
        assert_eq!(state.mode, Mode::ResultDetail);
        assert_eq!(state.mode_stack.len(), 1);
        assert_eq!(state.mode_stack[0], Mode::Search);

        // Show help from result detail mode
        state.update(Message::ShowHelp);
        assert_eq!(state.mode, Mode::Help);
        assert_eq!(state.mode_stack.len(), 2);
        assert_eq!(state.mode_stack[0], Mode::Search);
        assert_eq!(state.mode_stack[1], Mode::ResultDetail);

        // Close help should return to result detail mode
        state.update(Message::CloseHelp);
        assert_eq!(state.mode, Mode::ResultDetail);
        assert_eq!(state.mode_stack.len(), 1);
        assert_eq!(state.mode_stack[0], Mode::Search);
    }

    #[test]
    fn test_help_dialog_navigation_from_session_viewer_mode() {
        let mut state = AppState::new(SearchOptions::default(), 100);

        // First navigate to session viewer mode
        state.search.results = vec![create_test_result()];
        state.update(Message::EnterSessionViewer);
        assert_eq!(state.mode, Mode::SessionViewer);
        assert_eq!(state.mode_stack.len(), 1);
        assert_eq!(state.mode_stack[0], Mode::Search);

        // Show help from session viewer mode
        state.update(Message::ShowHelp);
        assert_eq!(state.mode, Mode::Help);
        assert_eq!(state.mode_stack.len(), 2);
        assert_eq!(state.mode_stack[0], Mode::Search);
        assert_eq!(state.mode_stack[1], Mode::SessionViewer);

        // Close help should return to session viewer mode
        state.update(Message::CloseHelp);
        assert_eq!(state.mode, Mode::SessionViewer);
        assert_eq!(state.mode_stack.len(), 1);
        assert_eq!(state.mode_stack[0], Mode::Search);
    }

    #[test]
    fn test_help_dialog_navigation_from_help_mode() {
        let mut state = AppState::new(SearchOptions::default(), 100);

        // Show help
        state.update(Message::ShowHelp);
        assert_eq!(state.mode, Mode::Help);

        // Trying to show help again from help mode should not change anything
        // (This is prevented in the key handler, but test the state handling)
        state.update(Message::ShowHelp);
        assert_eq!(state.mode, Mode::Help);
        assert_eq!(state.mode_stack.len(), 2); // Would push again if not prevented

        // Clean up the duplicate
        state.mode_stack.pop();

        // Close help
        state.update(Message::CloseHelp);
        assert_eq!(state.mode, Mode::Search);
    }

    #[test]
    fn test_help_dialog_navigation_complex_flow() {
        let mut state = AppState::new(SearchOptions::default(), 100);

        // Navigate: Search -> ResultDetail -> SessionViewer -> Help
        state.search.results = vec![create_test_result()];
        state.update(Message::EnterResultDetail);
        state.update(Message::EnterSessionViewer);
        state.update(Message::ShowHelp);

        assert_eq!(state.mode, Mode::Help);
        assert_eq!(state.mode_stack.len(), 3);
        assert_eq!(state.mode_stack[0], Mode::Search);
        assert_eq!(state.mode_stack[1], Mode::ResultDetail);
        assert_eq!(state.mode_stack[2], Mode::SessionViewer);

        // Close help should return to session viewer
        state.update(Message::CloseHelp);
        assert_eq!(state.mode, Mode::SessionViewer);
        assert_eq!(state.mode_stack.len(), 2);

        // Navigate back to search
        state.update(Message::ExitToSearch);
        assert_eq!(state.mode, Mode::ResultDetail);
        state.update(Message::ExitToSearch);
        assert_eq!(state.mode, Mode::Search);
        assert!(state.mode_stack.is_empty());
    }

    // Helper function to create a test result
    fn create_test_result() -> crate::query::condition::SearchResult {
        use crate::query::condition::{QueryCondition, SearchResult};

        SearchResult {
            file: "/test/file.jsonl".to_string(),
            uuid: "test-uuid".to_string(),
            timestamp: "2024-01-01T00:00:00Z".to_string(),
            session_id: "test-session".to_string(),
            role: "user".to_string(),
            text: "Test content".to_string(),
            has_tools: false,
            has_thinking: false,
            message_type: "user".to_string(),
            query: QueryCondition::Literal {
                pattern: "test".to_string(),
                case_sensitive: false,
            },
            project_path: "/test/project".to_string(),
            raw_json: Some(
                r#"{"type":"user","content":[{"type":"text","text":"Test content"}]}"#.to_string(),
            ),
        }
    }
}
