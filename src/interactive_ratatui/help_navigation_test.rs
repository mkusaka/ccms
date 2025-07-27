#[cfg(test)]
mod tests {
    use crate::SearchOptions;
    use crate::interactive_ratatui::ui::app_state::{AppState, Mode};
    use crate::interactive_ratatui::ui::events::Message;

    #[test]
    fn test_help_dialog_navigation_from_search_mode() {
        let mut state = AppState::new(SearchOptions::default(), 100);
        assert_eq!(state.mode, Mode::Search);
        assert!(state.navigation_history.is_empty());

        // Show help from search mode
        state.update(Message::ShowHelp);
        assert_eq!(state.mode, Mode::Help);
        assert_eq!(state.navigation_history.len(), 2); // Initial Search + Help
        assert!(state.navigation_history.can_go_back()); // Can go back to Search

        // Close help should return to search mode
        state.update(Message::CloseHelp);
        assert_eq!(state.mode, Mode::Search);
        assert!(!state.navigation_history.can_go_back()); // At position 0
        assert!(state.navigation_history.can_go_forward()); // Can go forward to Help
    }

    #[test]
    fn test_help_dialog_navigation_from_result_detail_mode() {
        let mut state = AppState::new(SearchOptions::default(), 100);

        // First navigate to result detail mode
        // (We need to set up a result first)
        state.search.results = vec![create_test_result()];
        state.update(Message::EnterResultDetail);
        assert_eq!(state.mode, Mode::ResultDetail);
        assert_eq!(state.navigation_history.len(), 2); // Search + ResultDetail

        // Show help from result detail mode
        state.update(Message::ShowHelp);
        assert_eq!(state.mode, Mode::Help);
        assert_eq!(state.navigation_history.len(), 3); // Search + ResultDetail + Help

        // Close help should return to result detail mode
        state.update(Message::CloseHelp);
        assert_eq!(state.mode, Mode::ResultDetail);
        assert!(state.navigation_history.can_go_back()); // Can go back to Search
        assert!(state.navigation_history.can_go_forward()); // Can go forward to Help
    }

    #[test]
    fn test_help_dialog_navigation_from_session_viewer_mode() {
        let mut state = AppState::new(SearchOptions::default(), 100);

        // First navigate to session viewer mode
        state.search.results = vec![create_test_result()];
        state.update(Message::EnterSessionViewer);
        assert_eq!(state.mode, Mode::SessionViewer);
        assert_eq!(state.navigation_history.len(), 2); // Search + SessionViewer

        // Show help from session viewer mode
        state.update(Message::ShowHelp);
        assert_eq!(state.mode, Mode::Help);
        assert_eq!(state.navigation_history.len(), 3); // Search + SessionViewer + Help

        // Close help should return to session viewer mode
        state.update(Message::CloseHelp);
        assert_eq!(state.mode, Mode::SessionViewer);
        assert!(state.navigation_history.can_go_back()); // Can go back to Search
        assert!(state.navigation_history.can_go_forward()); // Can go forward to Help
    }

    #[test]
    fn test_help_dialog_navigation_from_help_mode() {
        let mut state = AppState::new(SearchOptions::default(), 100);

        // Show help
        state.update(Message::ShowHelp);
        assert_eq!(state.mode, Mode::Help);
        assert_eq!(state.navigation_history.len(), 2); // Search + Help

        // Trying to show help again from help mode will push another Help state
        state.update(Message::ShowHelp);
        assert_eq!(state.mode, Mode::Help);
        assert_eq!(state.navigation_history.len(), 3); // Search + Help + Help

        // Close help - should go back to previous state (which is also Help)
        state.update(Message::CloseHelp);
        assert_eq!(state.mode, Mode::Help);
        assert_eq!(state.navigation_history.len(), 3); // History stays same, just position changed
    }

    #[test]
    fn test_help_dialog_navigation_complex_flow() {
        let mut state = AppState::new(SearchOptions::default(), 100);

        // Navigate: Search -> ResultDetail -> SessionViewer -> Help
        state.search.results = vec![create_test_result()];
        state.update(Message::EnterResultDetail);
        assert_eq!(state.navigation_history.len(), 2); // Search + ResultDetail

        state.update(Message::EnterSessionViewer);
        assert_eq!(state.navigation_history.len(), 3); // Search + ResultDetail + SessionViewer

        state.update(Message::ShowHelp);
        assert_eq!(state.mode, Mode::Help);
        assert_eq!(state.navigation_history.len(), 4); // Search + ResultDetail + SessionViewer + Help

        // Close help should return to session viewer
        state.update(Message::CloseHelp);
        assert_eq!(state.mode, Mode::SessionViewer);

        // Navigate back to result detail
        state.update(Message::ExitToSearch);
        assert_eq!(state.mode, Mode::ResultDetail);

        // Navigate back to search
        state.update(Message::ExitToSearch);
        assert_eq!(state.mode, Mode::Search);

        // Should be able to navigate forward
        assert!(state.navigation_history.can_go_forward());
    }

    #[test]
    fn test_navigation_back_forward() {
        let mut state = AppState::new(SearchOptions::default(), 100);

        // Navigate: Search -> ResultDetail -> SessionViewer
        state.search.results = vec![create_test_result()];
        state.update(Message::EnterResultDetail);
        assert_eq!(state.navigation_history.len(), 2); // Search + ResultDetail

        state.update(Message::EnterSessionViewer);
        assert_eq!(state.mode, Mode::SessionViewer);
        assert_eq!(state.navigation_history.len(), 3); // Search + ResultDetail + SessionViewer

        // Navigate back from SessionViewer to ResultDetail
        state.update(Message::NavigateBack);
        assert_eq!(state.mode, Mode::ResultDetail);
        assert!(state.navigation_history.can_go_back());
        assert!(state.navigation_history.can_go_forward());

        // Navigate back again to Search
        state.update(Message::NavigateBack);
        assert_eq!(state.mode, Mode::Search);
        assert!(!state.navigation_history.can_go_back()); // Can't go back from position 0
        assert!(state.navigation_history.can_go_forward());

        // Navigate forward - goes to ResultDetail
        state.update(Message::NavigateForward);
        assert_eq!(state.mode, Mode::ResultDetail);
        assert!(state.navigation_history.can_go_back());
        assert!(state.navigation_history.can_go_forward());

        // Navigate forward again to SessionViewer
        state.update(Message::NavigateForward);
        assert_eq!(state.mode, Mode::SessionViewer);
        assert!(state.navigation_history.can_go_back());
        assert!(!state.navigation_history.can_go_forward()); // At the end of history
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
