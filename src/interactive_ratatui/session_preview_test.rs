#[cfg(test)]
mod tests {
    use crate::interactive_ratatui::ui::app_state::{AppState, Mode};
    use crate::interactive_ratatui::ui::commands::Command;
    use crate::interactive_ratatui::ui::events::Message;
    use crate::query::condition::{QueryCondition, SearchResult};

    #[test]
    fn test_session_viewer_preview_toggle() {
        let mut state = AppState::new();

        // Set up session viewer mode
        state.mode = Mode::SessionViewer;
        state.session.session_id = Some("test-session".to_string());

        // Initially preview should be disabled
        assert!(!state.session.preview_enabled);

        // Toggle preview
        let command = state.update(Message::ToggleSessionPreview);
        assert_eq!(command, Command::None);
        assert!(state.session.preview_enabled);

        // Toggle again
        let command = state.update(Message::ToggleSessionPreview);
        assert_eq!(command, Command::None);
        assert!(!state.session.preview_enabled);
    }

    #[test]
    fn test_session_viewer_preview_with_results() {
        let mut state = AppState::new();

        // Set up session viewer mode with results
        state.mode = Mode::SessionViewer;
        state.session.session_id = Some("test-session".to_string());

        // Add some test results
        let results = vec![
            SearchResult {
                file: "test.jsonl".to_string(),
                uuid: "uuid1".to_string(),
                timestamp: "2024-01-01T00:00:00Z".to_string(),
                session_id: "test-session".to_string(),
                role: "user".to_string(),
                text: "Test message 1".to_string(),
                message_type: "message".to_string(),
                query: QueryCondition::Literal {
                    pattern: "test".to_string(),
                    case_sensitive: false,
                },
                cwd: "/test".to_string(),
                raw_json: Some(
                    r#"{"type":"user","message":{"content":"Test message 1"}}"#.to_string(),
                ),
            },
            SearchResult {
                file: "test.jsonl".to_string(),
                uuid: "uuid2".to_string(),
                timestamp: "2024-01-01T00:01:00Z".to_string(),
                session_id: "test-session".to_string(),
                role: "assistant".to_string(),
                text: "Test response 1".to_string(),
                message_type: "message".to_string(),
                query: QueryCondition::Literal {
                    pattern: "test".to_string(),
                    case_sensitive: false,
                },
                cwd: "/test".to_string(),
                raw_json: Some(
                    r#"{"type":"assistant","message":{"content":"Test response 1"}}"#.to_string(),
                ),
            },
        ];

        state.session.search_results = results;

        // Enable preview
        let command = state.update(Message::ToggleSessionPreview);
        assert_eq!(command, Command::None);
        assert!(state.session.preview_enabled);

        // Verify preview state is maintained during navigation
        state.session.selected_index = 1;
        assert!(state.session.preview_enabled);
    }
}
