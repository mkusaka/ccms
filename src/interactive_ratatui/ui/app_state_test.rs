#[cfg(test)]
mod tests {
    use super::super::app_state::*;
    use super::super::commands::Command;
    use super::super::events::{CopyContent, Message};
    use crate::interactive_ratatui::domain::models::SearchTab;
    use crate::interactive_ratatui::domain::models::{Mode, SearchOrder, SessionOrder};
    use crate::interactive_ratatui::ui::app_state::SessionInfo;
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
        assert_eq!(state.ui.message, Some("[typing...]".to_string()));
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
        assert!(state.ui.show_help);
        assert_eq!(state.mode, Mode::Search); // Mode should not change

        // Close help
        let _command = state.update(Message::CloseHelp);
        assert!(!state.ui.show_help);
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
        state.mode = Mode::SessionViewer;
        state.session.session_id = Some("session1".to_string());
        state.session.file_path = Some("test.jsonl".to_string());

        let command = state.update(Message::SessionQueryChanged("Hello".to_string()));

        assert_eq!(state.session.query, "Hello");
        // In the new unified architecture, the search is performed by the search service
        // and results are stored in search_results, not filtered_indices
        assert!(matches!(command, Command::ExecuteSessionSearch));
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
        state.session.session_id = Some("session1".to_string());
        state.session.file_path = Some("test.jsonl".to_string());

        // Default is Ascending
        assert_eq!(state.session.order, SessionOrder::Ascending);

        // Toggle to Descending
        let command = state.update(Message::ToggleSessionOrder);
        assert_eq!(state.session.order, SessionOrder::Descending);
        // In the new architecture, changing order triggers a new search
        assert!(matches!(command, Command::ExecuteSessionSearch));

        // Toggle back to Ascending
        let command = state.update(Message::ToggleSessionOrder);
        assert_eq!(state.session.order, SessionOrder::Ascending);
        assert!(matches!(command, Command::ExecuteSessionSearch));
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
        // No longer need to call update_session_filter - filtering is done by search service

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

    fn create_test_session_info(id: &str, message: &str) -> SessionInfo {
        SessionInfo {
            file_path: format!("/test/{id}.jsonl"),
            session_id: id.to_string(),
            timestamp: "2024-01-01T00:00:00Z".to_string(),
            message_count: 5,
            first_message: message.to_string(),
            preview_messages: vec![
                (
                    "user".to_string(),
                    message.to_string(),
                    "2024-01-01T00:00:00Z".to_string(),
                ),
                (
                    "assistant".to_string(),
                    format!("Response to {message}"),
                    "2024-01-01T00:00:01Z".to_string(),
                ),
            ],
            summary: Some(format!("Summary about {message}")),
        }
    }

    #[test]
    fn test_session_list_filtering() {
        let mut state = create_test_state();

        // Load test sessions
        let sessions = vec![
            create_test_session_info("session1", "Hello world"),
            create_test_session_info("session2", "Goodbye world"),
            create_test_session_info("session3", "Testing search"),
        ];

        state.update(Message::SessionListLoaded(
            sessions
                .into_iter()
                .map(|s| {
                    (
                        s.file_path.clone(),
                        s.session_id.clone(),
                        s.timestamp.clone(),
                        s.message_count,
                        s.first_message.clone(),
                        s.preview_messages.clone(),
                        s.summary.clone(),
                    )
                })
                .collect(),
        ));

        // Initially all sessions should be visible
        assert_eq!(state.session_list.sessions.len(), 3);
        assert_eq!(state.session_list.filtered_sessions.len(), 3);

        // Filter by "world" - this triggers debounced search scheduling
        let command = state.update(Message::SessionListQueryChanged("world".to_string()));
        assert_eq!(state.session_list.query, "world");
        // Should schedule a search with debounce
        assert!(matches!(command, Command::ScheduleSessionListSearch(300)));
        assert!(state.session_list.is_typing);

        // Trigger the actual search
        let command = state.update(Message::SessionListSearchRequested);
        assert!(matches!(command, Command::ExecuteSessionListSearch));
        assert!(!state.session_list.is_typing);
        assert!(state.session_list.is_searching);
        // The filtered_sessions should remain unchanged until search completes
        assert_eq!(state.session_list.filtered_sessions.len(), 3);

        // Simulate search completion with filtered results
        let filtered_sessions = vec![
            create_test_session_info("session1", "Hello world"),
            create_test_session_info("session2", "Goodbye world"),
        ];
        state.update(Message::SessionListSearchCompleted(filtered_sessions));
        assert_eq!(state.session_list.filtered_sessions.len(), 2);

        // Verify correct sessions are filtered
        assert!(
            state
                .session_list
                .filtered_sessions
                .iter()
                .any(|s| s.session_id == "session1")
        );
        assert!(
            state
                .session_list
                .filtered_sessions
                .iter()
                .any(|s| s.session_id == "session2")
        );

        // Filter by "Hello" - triggers debounced search
        let command = state.update(Message::SessionListQueryChanged("Hello".to_string()));
        assert!(matches!(command, Command::ScheduleSessionListSearch(300)));

        // Trigger the actual search
        let command = state.update(Message::SessionListSearchRequested);
        assert!(matches!(command, Command::ExecuteSessionListSearch));

        // Simulate search completion
        let filtered_sessions = vec![create_test_session_info("session1", "Hello world")];
        state.update(Message::SessionListSearchCompleted(filtered_sessions));
        assert_eq!(state.session_list.filtered_sessions.len(), 1);
        assert_eq!(
            state.session_list.filtered_sessions[0].session_id,
            "session1"
        );

        // Clear filter - triggers debounced search that returns all
        let command = state.update(Message::SessionListQueryChanged("".to_string()));
        assert!(matches!(command, Command::ScheduleSessionListSearch(300)));

        // Trigger the actual search
        let command = state.update(Message::SessionListSearchRequested);
        assert!(matches!(command, Command::ExecuteSessionListSearch));

        // Simulate search completion with all sessions
        let all_sessions = vec![
            create_test_session_info("session1", "Hello world"),
            create_test_session_info("session2", "Goodbye world"),
            create_test_session_info("session3", "Testing search"),
        ];
        state.update(Message::SessionListSearchCompleted(all_sessions));
        assert_eq!(state.session_list.filtered_sessions.len(), 3);
    }

    #[test]
    fn test_session_list_query_inheritance() {
        let mut state = create_test_state();

        // Switch to SessionList tab
        state.update(Message::SwitchToSessionListTab);
        assert_eq!(state.search.current_tab, SearchTab::SessionList);

        // Load test sessions
        let sessions = vec![create_test_session_info("session1", "Test message")];

        state.update(Message::SessionListLoaded(
            sessions
                .into_iter()
                .map(|s| {
                    (
                        s.file_path.clone(),
                        s.session_id.clone(),
                        s.timestamp.clone(),
                        s.message_count,
                        s.first_message.clone(),
                        s.preview_messages.clone(),
                        s.summary.clone(),
                    )
                })
                .collect(),
        ));

        // Set a search query that will match the session
        state.update(Message::SessionListQueryChanged("test".to_string()));
        assert_eq!(state.session_list.query, "test");

        // Enter session viewer
        let command = state.update(Message::EnterSessionViewerFromList(
            "/test/session1.jsonl".to_string(),
        ));
        assert_eq!(state.mode, Mode::SessionViewer);

        // Query should be inherited
        assert_eq!(state.session.query, "test");
        assert!(matches!(command, Command::LoadSession(_)));
    }

    #[test]
    fn test_session_list_navigation() {
        let mut state = create_test_state();

        // Load test sessions
        let sessions = vec![
            create_test_session_info("session1", "First"),
            create_test_session_info("session2", "Second"),
            create_test_session_info("session3", "Third"),
        ];

        state.update(Message::SessionListLoaded(
            sessions
                .into_iter()
                .map(|s| {
                    (
                        s.file_path.clone(),
                        s.session_id.clone(),
                        s.timestamp.clone(),
                        s.message_count,
                        s.first_message.clone(),
                        s.preview_messages.clone(),
                        s.summary.clone(),
                    )
                })
                .collect(),
        ));

        assert_eq!(state.session_list.selected_index, 0);

        // Navigate down
        state.update(Message::SessionListScrollDown);
        assert_eq!(state.session_list.selected_index, 1);

        state.update(Message::SessionListScrollDown);
        assert_eq!(state.session_list.selected_index, 2);

        // Should not go beyond last item
        state.update(Message::SessionListScrollDown);
        assert_eq!(state.session_list.selected_index, 2);

        // Navigate up
        state.update(Message::SessionListScrollUp);
        assert_eq!(state.session_list.selected_index, 1);

        // Page navigation
        state.update(Message::SessionListPageDown);
        assert_eq!(state.session_list.selected_index, 2); // At the end

        state.update(Message::SessionListPageUp);
        assert_eq!(state.session_list.selected_index, 0); // Back to start
    }

    #[test]
    fn test_session_list_filter_resets_selection() {
        let mut state = create_test_state();

        // Load test sessions
        let sessions = vec![
            create_test_session_info("session1", "First"),
            create_test_session_info("session2", "Second"),
            create_test_session_info("session3", "Third"),
        ];

        state.update(Message::SessionListLoaded(
            sessions
                .into_iter()
                .map(|s| {
                    (
                        s.file_path.clone(),
                        s.session_id.clone(),
                        s.timestamp.clone(),
                        s.message_count,
                        s.first_message.clone(),
                        s.preview_messages.clone(),
                        s.summary.clone(),
                    )
                })
                .collect(),
        ));

        // Select the third item
        state.update(Message::SessionListScrollDown);
        state.update(Message::SessionListScrollDown);
        assert_eq!(state.session_list.selected_index, 2);

        // Filter to show only first item - triggers debounced search
        let command = state.update(Message::SessionListQueryChanged("First".to_string()));
        assert!(matches!(command, Command::ScheduleSessionListSearch(300)));

        // Trigger the actual search
        let command = state.update(Message::SessionListSearchRequested);
        assert!(matches!(command, Command::ExecuteSessionListSearch));

        // Simulate search completion with filtered results
        let filtered_sessions = vec![create_test_session_info("session1", "First")];
        state.update(Message::SessionListSearchCompleted(filtered_sessions));

        // Selection should reset to 0
        assert_eq!(state.session_list.selected_index, 0);
        assert_eq!(state.session_list.scroll_offset, 0);
    }

    #[test]
    fn test_session_list_case_insensitive_search() {
        let mut state = create_test_state();

        // Load test sessions
        let sessions = vec![
            create_test_session_info("session1", "HELLO WORLD"),
            create_test_session_info("session2", "hello world"),
            create_test_session_info("session3", "HeLLo WoRLD"),
        ];

        state.update(Message::SessionListLoaded(
            sessions
                .into_iter()
                .map(|s| {
                    (
                        s.file_path.clone(),
                        s.session_id.clone(),
                        s.timestamp.clone(),
                        s.message_count,
                        s.first_message.clone(),
                        s.preview_messages.clone(),
                        s.summary.clone(),
                    )
                })
                .collect(),
        ));

        // Search with lowercase - triggers debounced search
        let command = state.update(Message::SessionListQueryChanged("hello".to_string()));
        assert!(matches!(command, Command::ScheduleSessionListSearch(300)));

        // Trigger the actual search
        let command = state.update(Message::SessionListSearchRequested);
        assert!(matches!(command, Command::ExecuteSessionListSearch));

        // Simulate search completion - all sessions match
        let all_sessions = vec![
            create_test_session_info("session1", "HELLO WORLD"),
            create_test_session_info("session2", "hello world"),
            create_test_session_info("session3", "HeLLo WoRLD"),
        ];
        state.update(Message::SessionListSearchCompleted(all_sessions));
        assert_eq!(state.session_list.filtered_sessions.len(), 3);

        // Search with uppercase - triggers debounced search
        let command = state.update(Message::SessionListQueryChanged("WORLD".to_string()));
        assert!(matches!(command, Command::ScheduleSessionListSearch(300)));

        // Trigger the actual search
        let command = state.update(Message::SessionListSearchRequested);
        assert!(matches!(command, Command::ExecuteSessionListSearch));

        // Simulate search completion - all sessions match
        let all_sessions = vec![
            create_test_session_info("session1", "HELLO WORLD"),
            create_test_session_info("session2", "hello world"),
            create_test_session_info("session3", "HeLLo WoRLD"),
        ];
        state.update(Message::SessionListSearchCompleted(all_sessions));
        assert_eq!(state.session_list.filtered_sessions.len(), 3);
    }

    #[test]
    fn test_session_list_search_in_summary() {
        let mut state = create_test_state();

        // Create session with unique text in summary
        let mut session = create_test_session_info("session1", "Regular message");
        session.summary = Some("Unique summary content".to_string());

        state.update(Message::SessionListLoaded(vec![(
            session.file_path.clone(),
            session.session_id.clone(),
            session.timestamp.clone(),
            session.message_count,
            session.first_message.clone(),
            session.preview_messages.clone(),
            session.summary.clone(),
        )]));

        // Search for text in summary - triggers debounced search
        let command = state.update(Message::SessionListQueryChanged("unique".to_string()));
        assert!(matches!(command, Command::ScheduleSessionListSearch(300)));

        // Trigger the actual search
        let command = state.update(Message::SessionListSearchRequested);
        assert!(matches!(command, Command::ExecuteSessionListSearch));

        // Simulate search completion - note that current implementation searches messages, not summaries
        // So this would return empty unless messages contain "unique"
        let filtered_sessions = vec![];
        state.update(Message::SessionListSearchCompleted(filtered_sessions));
        assert_eq!(state.session_list.filtered_sessions.len(), 0);

        // Search for text not in summary - triggers debounced search
        let command = state.update(Message::SessionListQueryChanged("notfound".to_string()));
        assert!(matches!(command, Command::ScheduleSessionListSearch(300)));

        // Trigger the actual search
        let command = state.update(Message::SessionListSearchRequested);
        assert!(matches!(command, Command::ExecuteSessionListSearch));

        // Simulate search completion - no results
        let filtered_sessions = vec![];
        state.update(Message::SessionListSearchCompleted(filtered_sessions));
        assert_eq!(state.session_list.filtered_sessions.len(), 0);
    }

    #[test]
    fn test_session_list_preview_toggle() {
        let mut state = create_test_state();

        // Preview should be enabled by default
        assert!(state.session_list.preview_enabled);

        // Toggle preview
        state.update(Message::ToggleSessionListPreview);
        assert!(!state.session_list.preview_enabled);

        // Toggle back
        state.update(Message::ToggleSessionListPreview);
        assert!(state.session_list.preview_enabled);
    }

    // Pagination tests
    #[test]
    fn test_search_completed_sets_pagination_state() {
        let mut state = create_test_state();

        // Create exactly 100 results to simulate a full page
        let mut results = Vec::new();
        for i in 0..100 {
            let mut result = create_test_result();
            result.text = format!("Message {i}");
            results.push(result);
        }

        state.update(Message::SearchCompleted(results));

        // Should indicate more results are available
        assert_eq!(state.search.total_loaded, 100);
        assert!(state.search.has_more_results);
        assert!(!state.search.loading_more);
    }

    #[test]
    fn test_search_completed_with_partial_page() {
        let mut state = create_test_state();

        // Create less than 100 results
        let mut results = Vec::new();
        for i in 0..50 {
            let mut result = create_test_result();
            result.text = format!("Message {i}");
            results.push(result);
        }

        state.update(Message::SearchCompleted(results));

        // Should indicate no more results
        assert_eq!(state.search.total_loaded, 50);
        assert!(!state.search.has_more_results);
        assert!(!state.search.loading_more);
    }

    #[test]
    fn test_search_requested_resets_pagination() {
        let mut state = create_test_state();

        // Set some pagination state
        state.search.has_more_results = true;
        state.search.loading_more = true;
        state.search.total_loaded = 200;

        // New search should reset pagination
        state.update(Message::SearchRequested);

        assert!(!state.search.has_more_results);
        assert!(!state.search.loading_more);
        assert_eq!(state.search.total_loaded, 0);
    }

    #[test]
    fn test_load_more_results_message() {
        let mut state = create_test_state();

        // Set up initial state with results available
        state.search.has_more_results = true;
        state.search.loading_more = false;
        state.search.total_loaded = 100;

        let command = state.update(Message::LoadMoreResults);

        // Should start loading more
        assert!(state.search.loading_more);
        assert_eq!(state.ui.message, Some("[loading more...]".to_string()));
        assert!(matches!(command, Command::LoadMore(100)));
    }

    #[test]
    fn test_load_more_when_no_more_results() {
        let mut state = create_test_state();

        // No more results available
        state.search.has_more_results = false;
        state.search.loading_more = false;

        let command = state.update(Message::LoadMoreResults);

        // Should not trigger loading
        assert!(!state.search.loading_more);
        assert!(matches!(command, Command::None));
    }

    #[test]
    fn test_load_more_when_already_loading() {
        let mut state = create_test_state();

        // Already loading
        state.search.has_more_results = true;
        state.search.loading_more = true;

        let command = state.update(Message::LoadMoreResults);

        // Should not trigger another load
        assert!(matches!(command, Command::None));
    }

    #[test]
    fn test_more_results_loaded_full_batch() {
        let mut state = create_test_state();

        // Set initial state
        state.search.loading_more = true;
        state.search.total_loaded = 100;
        let mut existing_results = Vec::new();
        for i in 0..100 {
            let mut result = create_test_result();
            result.text = format!("Message {i}");
            existing_results.push(result);
        }
        state.search.results = existing_results;

        // Load another full batch of 100
        let mut new_results = Vec::new();
        for i in 100..200 {
            let mut result = create_test_result();
            result.text = format!("Message {i}");
            new_results.push(result);
        }

        state.update(Message::MoreResultsLoaded(new_results));

        // Should append results and update state
        assert_eq!(state.search.results.len(), 200);
        assert_eq!(state.search.total_loaded, 200);
        assert!(state.search.has_more_results);
        assert!(!state.search.loading_more);
        assert_eq!(state.ui.message, None);
    }

    #[test]
    fn test_more_results_loaded_partial_batch() {
        let mut state = create_test_state();

        // Set initial state
        state.search.loading_more = true;
        state.search.total_loaded = 100;
        let mut existing_results = Vec::new();
        for i in 0..100 {
            let mut result = create_test_result();
            result.text = format!("Message {i}");
            existing_results.push(result);
        }
        state.search.results = existing_results;

        // Load partial batch (less than 100)
        let mut new_results = Vec::new();
        for i in 100..130 {
            let mut result = create_test_result();
            result.text = format!("Message {i}");
            new_results.push(result);
        }

        state.update(Message::MoreResultsLoaded(new_results));

        // Should indicate no more results
        assert_eq!(state.search.results.len(), 130);
        assert_eq!(state.search.total_loaded, 130);
        assert!(!state.search.has_more_results);
        assert!(!state.search.loading_more);
        assert_eq!(state.ui.message, Some("[all results loaded]".to_string()));
    }
}
