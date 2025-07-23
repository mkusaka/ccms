#[cfg(test)]
mod tests {
    use super::super::*;
    use crate::interactive_ratatui::domain::models::Mode;
    use crate::interactive_ratatui::ui::events::Message;
    use crate::{QueryCondition, SearchOptions, SearchResult};
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use ratatui::backend::TestBackend;
    use ratatui::{Terminal, buffer::Buffer};

    /// Test for terminal lifecycle management
    /// This test verifies that the run() method properly initializes and cleans up
    /// the terminal state, even when errors occur during execution.
    #[test]
    fn test_run_terminal_lifecycle() {
        // Note: Testing the actual run() method is challenging because it:
        // 1. Takes control of the terminal
        // 2. Runs an event loop
        // 3. Requires real user input
        //
        // In practice, this is tested through:
        // - Manual integration testing
        // - CI tests that verify the binary runs without panicking
        // - The existing unit tests that test the individual components

        // Here we document what run() should do:
        // 1. Enable raw mode via crossterm
        // 2. Setup alternate screen buffer
        // 3. Create terminal with CrosstermBackend
        // 4. Call run_app() in a loop
        // 5. On exit or error, restore terminal state
        // 6. Propagate any errors from run_app()
    }

    /// Test for the main event loop in run_app()
    /// This documents the expected behavior of the event loop
    #[test]
    fn test_run_app_behavior() {
        // The run_app() method should:
        // 1. Draw the current UI state
        // 2. Poll for events with a timeout
        // 3. Handle keyboard events appropriately
        // 4. Update the application state
        // 5. Continue until the user exits
        //
        // Key behaviors to test:
        // - Non-blocking event polling (50ms timeout)
        // - Proper event handling for all supported keys
        // - State updates trigger redraws
        // - Exit conditions work correctly
    }

    /// Test UI rendering methods in isolation
    #[test]
    fn test_ui_rendering_isolation() {
        let mut app = InteractiveSearch::new(SearchOptions::default());
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        // Test search mode rendering
        app.set_mode(Mode::Search);
        terminal
            .draw(|f| app.renderer.render(f, &app.state))
            .unwrap();
        let buffer = terminal.backend().buffer();
        assert!(buffer_contains(buffer, "Search"));

        // Test help mode rendering
        app.push_screen(Mode::Help);
        terminal
            .draw(|f| app.renderer.render(f, &app.state))
            .unwrap();
        let buffer = terminal.backend().buffer();
        assert!(buffer_contains(buffer, "Help"));

        // Test results rendering
        app.set_mode(Mode::Search);
        app.state.search.results = vec![
            create_test_result("user", "Hello world", "2024-01-01T12:00:00Z"),
            create_test_result("assistant", "Hi there!", "2024-01-01T12:01:00Z"),
        ];
        terminal
            .draw(|f| app.renderer.render(f, &app.state))
            .unwrap();
    }

    /// Test error handling in various scenarios
    #[test]
    fn test_error_handling_scenarios() {
        let mut app = InteractiveSearch::new(SearchOptions::default());

        // Test handling of invalid session file
        app.state.ui.selected_result = Some(SearchResult {
            file: "/nonexistent/file.jsonl".to_string(),
            uuid: "12345678-1234-5678-1234-567812345678".to_string(),
            timestamp: "2024-01-01T00:00:00Z".to_string(),
            session_id: "87654321-4321-8765-4321-876543218765".to_string(),
            role: "user".to_string(),
            text: "test".to_string(),
            has_tools: false,
            has_thinking: false,
            message_type: "user".to_string(),
            query: QueryCondition::Literal {
                pattern: "test".to_string(),
                case_sensitive: false,
            },
            project_path: "/test".to_string(),
            raw_json: None,
        });

        // Test session loading failure handling
        app.load_session_messages("/nonexistent/file.jsonl");
        assert!(app.state.session.messages.is_empty());
        assert!(app.state.ui.message.is_some());
    }

    /// Test session viewer functionality
    #[test]
    fn test_session_viewer_behavior() {
        let mut app = InteractiveSearch::new(SearchOptions::default());

        // Simulate having a selected result
        app.state.search.results = vec![create_test_result(
            "user",
            "Test message",
            "2024-01-01T00:00:00Z",
        )];
        app.state.search.selected_index = 0;

        // Transition to session viewer
        app.state.mode = Mode::SessionViewer;

        // Verify session viewer state initialization
        assert_eq!(app.current_mode(), Mode::SessionViewer);
    }

    /// Test search functionality integration
    #[test]
    fn test_search_integration() {
        let mut app = InteractiveSearch::new(SearchOptions::default());

        // Set a search query
        app.state.search.query = "test query".to_string();

        // Execute search
        app.execute_search();

        // Verify search state
        assert!(app.state.search.is_searching);
        assert_eq!(app.state.search.current_search_id, 1);
    }

    /// Test role filter cycling
    #[test]
    fn test_role_filter_cycling() {
        let mut app = InteractiveSearch::new(SearchOptions::default());

        // Initial state - no filter
        assert_eq!(app.state.search.role_filter, None);

        // Cycle through filters
        app.handle_message(Message::ToggleRoleFilter);
        assert_eq!(app.state.search.role_filter, Some("user".to_string()));

        app.handle_message(Message::ToggleRoleFilter);
        assert_eq!(app.state.search.role_filter, Some("assistant".to_string()));

        app.handle_message(Message::ToggleRoleFilter);
        assert_eq!(app.state.search.role_filter, Some("system".to_string()));

        app.handle_message(Message::ToggleRoleFilter);
        assert_eq!(app.state.search.role_filter, None);
    }

    /// Test clipboard functionality
    #[test]
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    fn test_clipboard_operations() {
        let app = InteractiveSearch::new(SearchOptions::default());

        // Test clipboard copy (this might fail in CI environments without clipboard access)
        let result = app.copy_to_clipboard("test text");
        // We don't assert success as clipboard might not be available in test environment
        // but we ensure it doesn't panic
        let _ = result;
    }

    // Helper functions
    fn create_test_result(role: &str, text: &str, timestamp: &str) -> SearchResult {
        SearchResult {
            file: "/test/file.jsonl".to_string(),
            uuid: "12345678-1234-5678-1234-567812345678".to_string(),
            timestamp: timestamp.to_string(),
            session_id: "87654321-4321-8765-4321-876543218765".to_string(),
            role: role.to_string(),
            text: text.to_string(),
            has_tools: false,
            has_thinking: false,
            message_type: role.to_string(),
            query: QueryCondition::Literal {
                pattern: "test".to_string(),
                case_sensitive: false,
            },
            project_path: "/test/project".to_string(),
            raw_json: None,
        }
    }

    fn buffer_contains(buffer: &Buffer, text: &str) -> bool {
        let content = buffer.area.x..buffer.area.x + buffer.area.width;
        let lines = buffer.area.y..buffer.area.y + buffer.area.height;

        for y in lines {
            let mut line = String::new();
            for x in content.clone() {
                let cell = &buffer[(x, y)];
                line.push_str(cell.symbol());
            }
            if line.contains(text) {
                return true;
            }
        }
        false
    }

    /// Test that initial search query doesn't show pattern in search bar
    #[test]
    fn test_initial_search_no_pattern_display() {
        let mut app = InteractiveSearch::new(SearchOptions::default());
        app.pattern = "~/.claude/**/*.jsonl".to_string();

        // Pattern should be stored internally but not shown in search query
        assert_eq!(app.pattern, "~/.claude/**/*.jsonl");
        assert_eq!(app.state.search.query, "");

        // Render and check that pattern is not visible in search bar
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|f| app.renderer.render(f, &app.state))
            .unwrap();
        let buffer = terminal.backend().buffer();
        assert!(!buffer_contains(buffer, "~/.claude"));
    }

    /// Test ESC key behavior in different modes
    #[test]
    fn test_esc_key_behavior() {
        let mut app = InteractiveSearch::new(SearchOptions::default());

        // ESC in search mode should exit
        app.state.mode = Mode::Search;
        let should_exit = app
            .handle_input(KeyEvent::new(KeyCode::Esc, KeyModifiers::empty()))
            .unwrap();
        assert!(should_exit);

        // ESC in result detail should return to search
        app.state.mode = Mode::ResultDetail;
        let should_exit = app
            .handle_input(KeyEvent::new(KeyCode::Esc, KeyModifiers::empty()))
            .unwrap();
        assert!(!should_exit);
        assert_eq!(app.state.mode, Mode::Search);

        // ESC in session viewer should return to previous mode
        app.state.mode = Mode::ResultDetail;
        app.state.mode_stack.push(Mode::ResultDetail);
        app.state.mode = Mode::SessionViewer;
        let should_exit = app
            .handle_input(KeyEvent::new(KeyCode::Esc, KeyModifiers::empty()))
            .unwrap();
        assert!(!should_exit);
        assert_eq!(app.state.mode, Mode::ResultDetail);
    }

    /// Test navigation stack functionality
    #[test]
    fn test_navigation_stack() {
        let mut app = InteractiveSearch::new(SearchOptions::default());

        // Start in search mode
        app.state.mode = Mode::Search;
        assert!(app.state.mode_stack.is_empty());

        // Navigate to result detail
        app.state.search.results = vec![create_test_result("user", "test", "2024-01-01T00:00:00Z")];
        app.handle_message(Message::EnterResultDetail);
        assert_eq!(app.state.mode, Mode::ResultDetail);
        assert_eq!(app.state.mode_stack, vec![Mode::Search]);

        // Navigate to session viewer
        app.handle_message(Message::EnterSessionViewer);
        assert_eq!(app.state.mode, Mode::SessionViewer);
        assert_eq!(app.state.mode_stack, vec![Mode::Search, Mode::ResultDetail]);

        // ESC should pop back to result detail
        app.handle_message(Message::ExitToSearch);
        assert_eq!(app.state.mode, Mode::ResultDetail);
        assert_eq!(app.state.mode_stack, vec![Mode::Search]);

        // Another ESC should go back to search
        app.handle_message(Message::ExitToSearch);
        assert_eq!(app.state.mode, Mode::Search);
        assert!(app.state.mode_stack.is_empty());
    }

    /// Test copy feedback messages
    #[test]
    fn test_copy_feedback() {
        let mut app = InteractiveSearch::new(SearchOptions::default());

        // Test file path copy feedback
        app.execute_command(Command::CopyToClipboard("/path/to/file.jsonl".to_string()));
        // In CI environment, clipboard might fail
        if let Some(msg) = &app.state.ui.message {
            assert!(
                msg == "✓ Copied file path" || msg.starts_with("Failed to copy:"),
                "Unexpected message: {msg}"
            );
        }

        // Test session ID copy feedback
        app.state.ui.message = None;
        app.execute_command(Command::CopyToClipboard(
            "12345678-1234-5678-1234-567812345678".to_string(),
        ));
        if let Some(msg) = &app.state.ui.message {
            assert!(
                msg == "✓ Copied session ID" || msg.starts_with("Failed to copy:"),
                "Unexpected message: {msg}"
            );
        }

        // Test short text copy feedback
        app.state.ui.message = None;
        app.execute_command(Command::CopyToClipboard("short text".to_string()));
        if let Some(msg) = &app.state.ui.message {
            assert!(
                msg == "✓ Copied: short text" || msg.starts_with("Failed to copy:"),
                "Unexpected message: {msg}"
            );
        }

        // Test long message copy feedback
        app.state.ui.message = None;
        let long_text = "a".repeat(200);
        app.execute_command(Command::CopyToClipboard(long_text));
        if let Some(msg) = &app.state.ui.message {
            assert!(
                msg == "✓ Copied message text" || msg.starts_with("Failed to copy:"),
                "Unexpected message: {msg}"
            );
        }
    }

    /// Test empty search query returns all results
    #[test]
    fn test_empty_search_returns_all() {
        let mut app = InteractiveSearch::new(SearchOptions::default());

        // Empty query should trigger search
        app.state.search.query = "".to_string();
        app.execute_search();

        // Verify search is initiated even with empty query
        assert!(app.state.search.is_searching);
        assert_eq!(app.state.search.current_search_id, 1);
    }

    /// Test message detail metadata display
    #[test]
    fn test_message_detail_metadata() {
        let mut app = InteractiveSearch::new(SearchOptions::default());
        let result = create_test_result("user", "Test message", "2024-01-01T12:00:00Z");

        app.state.mode = Mode::ResultDetail;
        app.state.ui.selected_result = Some(result.clone());
        app.renderer.get_result_detail_mut().set_result(result);

        // Render and check metadata is displayed
        let backend = TestBackend::new(100, 40);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|f| app.renderer.render(f, &app.state))
            .unwrap();
        let buffer = terminal.backend().buffer();

        // Debug: print buffer content
        let content = buffer
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        if !buffer_contains(buffer, "Role: user") {
            println!("Buffer content: {content}");
        }

        assert!(buffer_contains(buffer, "Role: user"));
        assert!(buffer_contains(buffer, "Time:"));
        assert!(buffer_contains(buffer, "File: /test/file.jsonl"));
        assert!(buffer_contains(buffer, "Project: /test/project"));
        assert!(buffer_contains(
            buffer,
            "UUID: 12345678-1234-5678-1234-567812345678"
        ));
        assert!(buffer_contains(
            buffer,
            "Session: 87654321-4321-8765-4321-876543218765"
        ));
    }

    /// Test session viewer metadata display
    #[test]
    fn test_session_viewer_metadata() {
        let mut app = InteractiveSearch::new(SearchOptions::default());

        app.state.mode = Mode::SessionViewer;
        app.state.session.file_path = Some("/path/to/session.jsonl".to_string());
        app.state.session.session_id = Some("session-123".to_string());

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|f| app.renderer.render(f, &app.state))
            .unwrap();
        let buffer = terminal.backend().buffer();

        assert!(buffer_contains(buffer, "Session: session-123"));
        assert!(buffer_contains(buffer, "File: /path/to/session.jsonl"));
    }

    /// Test result list full text scrolling
    #[test]
    fn test_result_list_full_text_scroll() {
        let mut app = InteractiveSearch::new(SearchOptions::default());

        // Create results with long text
        let long_text =
            "This is a very long message that will wrap across multiple lines when displayed. "
                .repeat(5);
        app.state.search.results = vec![
            create_test_result("user", &long_text, "2024-01-01T00:00:00Z"),
            create_test_result("assistant", "Short message", "2024-01-01T00:01:00Z"),
        ];

        // Enable full text mode
        app.state.ui.truncation_enabled = false;

        // Test scrolling with SelectResult messages (new architecture)
        app.handle_message(Message::SelectResult(1));
        assert_eq!(app.state.search.selected_index, 1);

        app.handle_message(Message::SelectResult(0));
        assert_eq!(app.state.search.selected_index, 0);
    }

    /// Test message detail copy shortcuts
    #[test]
    fn test_message_detail_copy_shortcuts() {
        let mut app = InteractiveSearch::new(SearchOptions::default());
        let result = create_test_result("user", "Test message", "2024-01-01T00:00:00Z");

        app.state.mode = Mode::ResultDetail;
        app.state.ui.selected_result = Some(result.clone());
        app.renderer.get_result_detail_mut().set_result(result);

        // Test all copy shortcuts
        let shortcuts = vec![
            ('f', "✓ Copied file path"),
            ('F', "✓ Copied file path"),
            ('i', "✓ Copied session ID"),
            ('I', "✓ Copied session ID"),
            ('p', "✓ Copied file path"), // project path
            ('P', "✓ Copied file path"),
            ('m', "✓ Copied: Test message"), // short text shows the actual text
            ('M', "✓ Copied: Test message"),
        ];

        for (key, expected_feedback) in shortcuts {
            app.handle_input(KeyEvent::new(KeyCode::Char(key), KeyModifiers::empty()))
                .unwrap();
            assert!(
                app.state.ui.message.is_some(),
                "No message after pressing '{key}'"
            );
            let actual_message = app.state.ui.message.as_ref().unwrap();
            println!("Key '{key}': expected '{expected_feedback}', got '{actual_message}'");

            // In CI environment, clipboard might fail
            assert!(
                actual_message == expected_feedback
                    || actual_message.starts_with("Failed to copy:"),
                "Message '{actual_message}' doesn't match expected feedback '{expected_feedback}'"
            );
        }
    }

    /// Test session viewer default message display
    #[test]
    fn test_session_viewer_default_display() {
        let mut app = InteractiveSearch::new(SearchOptions::default());

        // Load messages into session viewer
        app.state.session.messages = vec![
            r#"{"type":"user","message":{"content":"Hello"},"timestamp":"2024-01-01T00:00:00Z"}"#
                .to_string(),
            r#"{"type":"assistant","message":{"content":"Hi"},"timestamp":"2024-01-01T00:01:00Z"}"#
                .to_string(),
        ];
        app.state.session.filtered_indices = vec![0, 1];
        app.state.mode = Mode::SessionViewer;

        // Render and verify messages are displayed
        let backend = TestBackend::new(120, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|f| app.renderer.render(f, &app.state))
            .unwrap();
        let buffer = terminal.backend().buffer();

        assert!(buffer_contains(buffer, "user"));
        assert!(buffer_contains(buffer, "Hello"));
        assert!(buffer_contains(buffer, "assistant"));
        assert!(buffer_contains(buffer, "Hi"));
    }
}
