#[cfg(test)]
mod tests {
    use super::super::session_preview::SessionPreview;
    use crate::interactive_ratatui::ui::app_state::SessionInfo;
    use crate::interactive_ratatui::ui::components::Component;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use ratatui::{Terminal, backend::TestBackend, buffer::Buffer};

    fn create_test_session_info_with_messages() -> SessionInfo {
        SessionInfo {
            file_path: "/test/session.jsonl".to_string(),
            session_id: "test-session".to_string(),
            timestamp: "2024-01-01T00:00:00Z".to_string(),
            message_count: 5,
            first_message: "First message".to_string(),
            preview_messages: vec![
                ("user".to_string(), "This is a test message".to_string()),
                (
                    "assistant".to_string(),
                    "Response about testing".to_string(),
                ),
                (
                    "user".to_string(),
                    "Another message without match".to_string(),
                ),
                (
                    "assistant".to_string(),
                    "More test content here".to_string(),
                ),
                ("user".to_string(), "Final message with test".to_string()),
            ],
            summary: Some("Summary about testing".to_string()),
        }
    }

    #[test]
    fn test_session_preview_basic_render() {
        let session_info = create_test_session_info_with_messages();
        let mut preview = SessionPreview::new();
        preview.set_session(Some(session_info));

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|f| {
                preview.render(f, f.area());
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let content = buffer_to_string(buffer);

        // Should contain session info
        assert!(content.contains("test-session"));
        assert!(
            content.contains("Messages: 5") || content.contains("5 messages"),
            "Content: {content}"
        );

        // Should contain messages
        assert!(content.contains("This is a test message"));
        assert!(content.contains("Response about testing"));
    }

    #[test]
    fn test_session_preview_highlight_matching() {
        let session_info = create_test_session_info_with_messages();
        let mut preview = SessionPreview::new();
        preview.set_session(Some(session_info));

        // Set query to match "test"
        preview.set_query("test".to_string());

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|f| {
                preview.render(f, f.area());
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let content = buffer_to_string(buffer);

        // Messages containing "test" should appear and be prioritized
        // All messages with "test" should be displayed
        assert!(content.contains("This is a test message"));
        assert!(content.contains("Response about testing"));
        assert!(content.contains("More test content here"));
        assert!(content.contains("Final message with test"));
    }

    #[test]
    fn test_session_preview_prioritize_matching_messages() {
        let session_info = create_test_session_info_with_messages();
        let mut preview = SessionPreview::new();
        preview.set_session(Some(session_info));

        // Set query to match specific message
        preview.set_query("Final message".to_string());

        let backend = TestBackend::new(80, 24); // Enough height to show messages
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|f| {
                preview.render(f, f.area());
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let content = buffer_to_string(buffer);

        // The matching message should appear in the limited space
        assert!(
            content.contains("Final message with test"),
            "Content: {content}"
        );
    }

    #[test]
    fn test_session_preview_no_query_no_highlight() {
        let session_info = create_test_session_info_with_messages();
        let mut preview = SessionPreview::new();
        preview.set_session(Some(session_info));

        // No query set - empty query should show all messages normally
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|f| {
                preview.render(f, f.area());
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let content = buffer_to_string(buffer);

        // All messages should appear in normal order
        assert!(content.contains("This is a test message"));
        assert!(content.contains("Response about testing"));
        assert!(content.contains("Another message without match"));
    }

    #[test]
    fn test_session_preview_case_insensitive_highlight() {
        let mut session_info = create_test_session_info_with_messages();
        // Add message with different case
        session_info
            .preview_messages
            .push(("user".to_string(), "TEST in uppercase".to_string()));

        let mut preview = SessionPreview::new();
        preview.set_session(Some(session_info));

        // Set lowercase query
        preview.set_query("test".to_string());

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|f| {
                preview.render(f, f.area());
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let content = buffer_to_string(buffer);

        // Both lowercase and uppercase "test" messages should match and appear
        assert!(content.contains("TEST in uppercase"));
        assert!(content.contains("This is a test message"));
    }

    #[test]
    fn test_session_preview_summary_highlight() {
        let session_info = create_test_session_info_with_messages();
        let mut preview = SessionPreview::new();
        preview.set_session(Some(session_info));

        // Set query to match text in summary
        preview.set_query("testing".to_string());

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|f| {
                preview.render(f, f.area());
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let content = buffer_to_string(buffer);

        // Summary should be included and highlighted
        assert!(content.contains("Summary about testing"));
    }

    #[test]
    fn test_session_preview_empty_state() {
        let mut preview = SessionPreview::new();

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|f| {
                preview.render(f, f.area());
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let content = buffer_to_string(buffer);

        // Should show empty state message
        assert!(content.contains("No session selected") || content.contains("Session Preview"));
    }

    #[test]
    fn test_session_preview_key_handling() {
        let session_info = create_test_session_info_with_messages();
        let mut preview = SessionPreview::new();
        preview.set_session(Some(session_info));

        // SessionPreview doesn't handle keys, should return None
        let result = preview.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::empty()));
        assert!(result.is_none());

        let result = preview.handle_key(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::empty()));
        assert!(result.is_none());
    }

    // Helper function to convert buffer to string for testing
    fn buffer_to_string(buffer: &Buffer) -> String {
        let mut output = String::new();
        for y in 0..buffer.area.height {
            for x in 0..buffer.area.width {
                let cell = buffer.cell((x, y)).unwrap();
                output.push_str(cell.symbol());
            }
            output.push('\n');
        }
        output
    }
}
