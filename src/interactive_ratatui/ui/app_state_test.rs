#[cfg(test)]
mod tests {
    use super::super::app_state::*;
    use super::super::commands::Command;
    use super::super::events::{CopyContent, Message};
    use crate::interactive_ratatui::domain::models::{Mode, SearchOrder, SessionOrder};
    use crate::query::condition::{QueryCondition, SearchResult};

    fn create_test_state() -> AppState {
        AppState::new()
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
            cwd: "/test".to_string(),
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

        // Enter message detail
        let _command = state.update(Message::EnterMessageDetail);
        assert_eq!(state.mode, Mode::MessageDetail);
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
        assert_eq!(state.search.role_filter, Some("summary".to_string()));

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

        let command = state.update(Message::CopyToClipboard(CopyContent::MessageContent(
            text.clone(),
        )));

        assert!(
            matches!(command, Command::CopyToClipboard(CopyContent::MessageContent(t)) if t == text)
        );
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
    fn test_toggle_preview() {
        let mut state = create_test_state();

        // Initial state should have preview disabled
        assert!(!state.search.preview_enabled);

        // Toggle to preview on
        let command = state.update(Message::TogglePreview);
        assert!(state.search.preview_enabled);
        // No status message should be set
        assert_eq!(state.ui.message, None);
        assert!(matches!(command, Command::None));

        // Toggle back to preview off
        let command = state.update(Message::TogglePreview);
        assert!(!state.search.preview_enabled);
        // Still no status message
        assert_eq!(state.ui.message, None);
        assert!(matches!(command, Command::None));
    }

    #[test]
    fn test_toggle_session_order() {
        let mut state = create_test_state();
        state.mode = Mode::SessionViewer;

        // Initial state should be Ascending (default)
        assert_eq!(state.session.order, SessionOrder::Ascending);

        // Toggle to Descending
        let command = state.update(Message::ToggleSessionOrder);
        assert_eq!(state.session.order, SessionOrder::Descending);
        assert!(matches!(command, Command::None));

        // Toggle back to Ascending
        let command = state.update(Message::ToggleSessionOrder);
        assert_eq!(state.session.order, SessionOrder::Ascending);
        assert!(matches!(command, Command::None));
    }

    #[test]
    fn test_session_order_sorting() {
        let mut state = create_test_state();
        state.mode = Mode::SessionViewer;

        // Set up messages with different timestamps
        state.session.messages = vec![
            r#"{"type":"user","message":{"content":"Third message"},"timestamp":"2024-01-03T12:00:00Z"}"#.to_string(),
            r#"{"type":"assistant","message":{"content":"First message"},"timestamp":"2024-01-01T12:00:00Z"}"#.to_string(),
            r#"{"type":"user","message":{"content":"Second message"},"timestamp":"2024-01-02T12:00:00Z"}"#.to_string(),
        ];

        // Default is Ascending, so should already be sorted
        state.update(Message::SessionQueryChanged("".to_string()));
        assert_eq!(state.session.filtered_indices, vec![1, 2, 0]); // Sorted by timestamp ascending

        // Set to Descending order
        state.session.order = SessionOrder::Descending;
        state.update(Message::SessionQueryChanged("".to_string()));
        assert_eq!(state.session.filtered_indices, vec![0, 2, 1]); // Sorted by timestamp descending

        // Set back to Ascending order
        state.session.order = SessionOrder::Ascending;
        state.update(Message::SessionQueryChanged("".to_string()));
        assert_eq!(state.session.filtered_indices, vec![1, 2, 0]); // Back to ascending
    }

    #[test]
    fn test_toggle_search_order() {
        let mut state = create_test_state();

        // Default should be Descending (newest first)
        assert_eq!(state.search.order, SearchOrder::Descending);

        // Toggle to Ascending (oldest first)
        let command = state.update(Message::ToggleSearchOrder);
        assert_eq!(command, Command::ExecuteSearch); // Should trigger new search
        assert_eq!(state.search.order, SearchOrder::Ascending);

        // Toggle back to Descending
        let command = state.update(Message::ToggleSearchOrder);
        assert_eq!(command, Command::ExecuteSearch); // Should trigger new search
        assert_eq!(state.search.order, SearchOrder::Descending);
    }

    #[test]
    fn test_search_completed_respects_engine_order() {
        let mut state = create_test_state();

        // Create test results already sorted by the engine
        let mut result1 = create_test_result();
        result1.timestamp = "2024-01-01T12:00:00Z".to_string();

        let mut result2 = create_test_result();
        result2.timestamp = "2024-01-02T12:00:00Z".to_string();

        let mut result3 = create_test_result();
        result3.timestamp = "2024-01-03T12:00:00Z".to_string();

        // Results come pre-sorted from the engine
        let results = vec![result3.clone(), result2.clone(), result1.clone()]; // Descending order
        state.update(Message::SearchCompleted(results));

        // Should maintain the order from the engine
        assert_eq!(state.search.results[0].timestamp, "2024-01-03T12:00:00Z");
        assert_eq!(state.search.results[1].timestamp, "2024-01-02T12:00:00Z");
        assert_eq!(state.search.results[2].timestamp, "2024-01-01T12:00:00Z");
    }

    #[test]
    fn test_filter_persistence_across_mode_transitions() {
        let mut state = create_test_state();

        // Set initial search filter and order
        state.search.role_filter = Some("user".to_string());
        state.search.order = SearchOrder::Ascending;

        // Add test results
        let results = vec![create_test_result()];
        state.search.results = results.clone();

        // Navigate to ResultDetail
        state.update(Message::EnterMessageDetail);
        assert_eq!(state.mode, Mode::MessageDetail);

        // Navigate back to Search
        state.update(Message::ExitToSearch);
        assert_eq!(state.mode, Mode::Search);

        // Verify filters are preserved
        assert_eq!(state.search.role_filter, Some("user".to_string()));
        assert_eq!(state.search.order, SearchOrder::Ascending);
    }

    #[test]
    fn test_session_filter_persistence() {
        let mut state = create_test_state();

        // Set up session with initial state
        state.mode = Mode::SessionViewer;
        state.session.messages = vec![
            r#"{"type":"user","message":{"content":"test1"},"timestamp":"2024-01-01T10:00:00Z"}"#.to_string(),
            r#"{"type":"assistant","message":{"content":"test2"},"timestamp":"2024-01-01T11:00:00Z"}"#.to_string(),
        ];
        state.session.role_filter = Some("assistant".to_string());
        state.session.order = SessionOrder::Descending;
        state.update_session_filter();

        // Navigate to ResultDetail from session
        let raw_json = state.session.messages[0].clone();
        state.update(Message::EnterMessageDetailFromSession(
            raw_json,
            "test.jsonl".to_string(),
            Some("session-id".to_string()),
        ));
        assert_eq!(state.mode, Mode::MessageDetail);

        // Navigate back to SessionViewer
        state.update(Message::ExitToSearch);
        assert_eq!(state.mode, Mode::SessionViewer);

        // Verify session filters are preserved
        assert_eq!(state.session.role_filter, Some("assistant".to_string()));
        assert_eq!(state.session.order, SessionOrder::Descending);
    }

    #[test]
    fn test_filter_changes_update_navigation_history() {
        let mut state = create_test_state();

        // Add test results to enable navigation
        state.search.results = vec![create_test_result()];

        // Navigate to ResultDetail to establish navigation history
        state.update(Message::EnterMessageDetail);

        // Go back to Search
        state.update(Message::ExitToSearch);
        assert_eq!(state.mode, Mode::Search);

        // Change filter
        state.update(Message::ToggleRoleFilter);
        assert_eq!(state.search.role_filter, Some("user".to_string()));

        // Change order too
        state.update(Message::ToggleSearchOrder);
        assert_eq!(state.search.order, SearchOrder::Ascending);

        // Navigate to ResultDetail again
        state.update(Message::EnterMessageDetail);

        // Go back - should see both updated filter and order
        state.update(Message::ExitToSearch);
        assert_eq!(state.search.role_filter, Some("user".to_string()));
        assert_eq!(state.search.order, SearchOrder::Ascending);
    }
}
