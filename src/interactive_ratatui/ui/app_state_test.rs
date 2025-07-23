#[cfg(test)]
mod tests {
    use super::super::app_state::*;
    use super::super::commands::Command;
    use super::super::events::Message;
    use crate::SearchOptions;
    use crate::query::condition::{QueryCondition, SearchResult};

    fn create_test_state() -> AppState {
        AppState::new(SearchOptions::default(), 100)
    }

    fn create_test_result() -> SearchResult {
        SearchResult {
            file: "test.jsonl".to_string(),
            uuid: "test-uuid".to_string(),
            timestamp: "2024-01-01T00:00:00Z".to_string(),
            session_id: "test-session".to_string(),
            role: "user".to_string(),
            text: "Test message".to_string(),
            has_tools: false,
            has_thinking: false,
            message_type: "user".to_string(),
            query: QueryCondition::Literal {
                pattern: "test".to_string(),
                case_sensitive: false,
            },
            project_path: "/test".to_string(),
            raw_json: None,
        }
    }

    #[test]
    fn test_initial_state() {
        let state = create_test_state();

        assert_eq!(state.mode, Mode::Search);
        assert_eq!(state.search.query, "");
        assert_eq!(state.search.results.len(), 0);
        assert_eq!(state.search.selected_index, 0);
        assert_eq!(state.search.role_filter, None);
        assert!(!state.search.is_searching);
        assert!(state.ui.truncation_enabled);
    }

    #[test]
    fn test_query_changed_message() {
        let mut state = create_test_state();

        let command = state.update(Message::QueryChanged("hello world".to_string()));

        assert_eq!(state.search.query, "hello world");
        assert!(matches!(command, Command::ScheduleSearch(300)));
        assert_eq!(state.ui.message, Some("typing...".to_string()));
    }

    #[test]
    fn test_search_completed_message() {
        let mut state = create_test_state();
        let results = vec![create_test_result()];

        state.search.is_searching = true;
        let command = state.update(Message::SearchCompleted(results));

        assert!(!state.search.is_searching);
        assert_eq!(state.search.results.len(), 1);
        assert_eq!(state.search.selected_index, 0);
        assert_eq!(state.ui.message, None);
        assert!(matches!(command, Command::None));
    }

    #[test]
    fn test_scroll_navigation() {
        let mut state = create_test_state();
        state.search.results = vec![
            create_test_result(),
            create_test_result(),
            create_test_result(),
        ];

        // Test selecting results (new architecture uses SelectResult messages)
        let _command = state.update(Message::SelectResult(1));
        assert_eq!(state.search.selected_index, 1);

        let _command = state.update(Message::SelectResult(2));
        assert_eq!(state.search.selected_index, 2);

        // Test boundary check
        let _command = state.update(Message::SelectResult(3));
        assert_eq!(state.search.selected_index, 2); // Should not go beyond bounds

        // Test selecting back
        let _command = state.update(Message::SelectResult(1));
        assert_eq!(state.search.selected_index, 1);

        let _command = state.update(Message::SelectResult(0));
        assert_eq!(state.search.selected_index, 0);
    }

    #[test]
    fn test_mode_transitions() {
        let mut state = create_test_state();
        state.search.results = vec![create_test_result()];

        // Enter result detail
        let _command = state.update(Message::EnterResultDetail);
        assert_eq!(state.mode, Mode::ResultDetail);
        assert!(state.ui.selected_result.is_some());

        // Exit back to search
        let _command = state.update(Message::ExitToSearch);
        assert_eq!(state.mode, Mode::Search);

        // Show help
        let _command = state.update(Message::ShowHelp);
        assert_eq!(state.mode, Mode::Help);

        // Close help
        let _command = state.update(Message::CloseHelp);
        assert_eq!(state.mode, Mode::Search);
    }

    #[test]
    fn test_role_filter_cycling() {
        let mut state = create_test_state();

        assert_eq!(state.search.role_filter, None);

        let command = state.update(Message::ToggleRoleFilter);
        assert_eq!(state.search.role_filter, Some("user".to_string()));
        assert!(matches!(command, Command::ExecuteSearch));

        let _command = state.update(Message::ToggleRoleFilter);
        assert_eq!(state.search.role_filter, Some("assistant".to_string()));

        let _command = state.update(Message::ToggleRoleFilter);
        assert_eq!(state.search.role_filter, Some("system".to_string()));

        let _command = state.update(Message::ToggleRoleFilter);
        assert_eq!(state.search.role_filter, None);
    }

    #[test]
    fn test_session_viewer_entry() {
        let mut state = create_test_state();
        state.search.results = vec![create_test_result()];
        state.search.selected_index = 0;

        let command = state.update(Message::EnterSessionViewer);

        assert_eq!(state.mode, Mode::SessionViewer);
        assert!(state.session.file_path.is_some());
        assert!(matches!(command, Command::LoadSession(_)));
    }

    #[test]
    fn test_clipboard_command() {
        let mut state = create_test_state();
        let text = "Copy this text".to_string();

        let command = state.update(Message::CopyToClipboard(text.clone()));

        assert!(matches!(command, Command::CopyToClipboard(t) if t == text));
    }

    #[test]
    fn test_session_query_update() {
        let mut state = create_test_state();
        state.session.messages = vec![
            r#"{"type":"user","message":{"role":"user","content":"Hello world"},"uuid":"1","timestamp":"2024-12-25T14:30:00Z","sessionId":"session1","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}"#.to_string(),
            r#"{"type":"assistant","message":{"role":"assistant","content":"Goodbye world"},"uuid":"2","timestamp":"2024-12-25T14:31:00Z","sessionId":"session1","parentUuid":"1","isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}"#.to_string(),
            r#"{"type":"user","message":{"role":"user","content":"Hello again"},"uuid":"3","timestamp":"2024-12-25T14:32:00Z","sessionId":"session1","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}"#.to_string(),
        ];

        let command = state.update(Message::SessionQueryChanged("Hello".to_string()));

        assert_eq!(state.session.query, "Hello");
        assert_eq!(state.session.filtered_indices, vec![0, 2]);
        assert!(matches!(command, Command::None));
    }

    #[test]
    fn test_status_messages() {
        let mut state = create_test_state();

        let _command = state.update(Message::SetStatus("Loading...".to_string()));
        assert_eq!(state.ui.message, Some("Loading...".to_string()));

        let _command = state.update(Message::ClearStatus);
        assert_eq!(state.ui.message, None);
    }

    #[test]
    fn test_toggle_truncation() {
        let mut state = create_test_state();

        // Initial state should be truncated
        assert!(state.ui.truncation_enabled);

        // Toggle to full text
        let command = state.update(Message::ToggleTruncation);
        assert!(!state.ui.truncation_enabled);
        assert_eq!(
            state.ui.message,
            Some("Message display: Full Text".to_string())
        );
        assert!(matches!(command, Command::None));

        // Toggle back to truncated
        let command = state.update(Message::ToggleTruncation);
        assert!(state.ui.truncation_enabled);
        assert_eq!(
            state.ui.message,
            Some("Message display: Truncated".to_string())
        );
        assert!(matches!(command, Command::None));
    }
}
