#[cfg(test)]
mod tests {
    use super::super::session_viewer::SessionViewer;
    use crate::interactive_ratatui::domain::models::SessionOrder;
    use crate::interactive_ratatui::ui::components::Component;
    use crate::interactive_ratatui::ui::events::{CopyContent, Message};
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use ratatui::{Terminal, backend::TestBackend, buffer::Buffer};

    fn render_component(component: &mut SessionViewer, width: u16, height: u16) -> Buffer {
        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|f| {
                component.render(f, f.area());
            })
            .unwrap();

        terminal.backend().buffer().clone()
    }

    fn buffer_contains(buffer: &Buffer, text: &str) -> bool {
        let content = buffer
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        content.contains(text)
    }

    #[test]
    fn test_session_viewer_new() {
        let mut viewer = SessionViewer::new();
        // Just test that it can be created
        let _buffer = render_component(&mut viewer, 80, 24);
    }

    #[test]
    fn test_session_viewer_with_long_file_path() {
        let mut viewer = SessionViewer::new();

        // Set a very long file path that should wrap
        let long_path = "/Users/masatomokusaka/.claude/projects/very-long-project-name/session-files/0ff88f7e-99a2-4c72-b7c1-fb95713d1832.jsonl";
        viewer.set_file_path(Some(long_path.to_string()));
        viewer.set_session_id(Some("test-session-123".to_string()));

        // Use a narrow terminal to force wrapping
        let buffer = render_component(&mut viewer, 40, 20);

        // Check that the title and session info are rendered
        assert!(buffer_contains(&buffer, "Session Viewer"));
        assert!(buffer_contains(&buffer, "Session: test-session-123"));
        assert!(buffer_contains(&buffer, "File:"));

        // The long path should be present (wrapped across multiple lines)
        assert!(buffer_contains(&buffer, "/Users/masatomokusaka"));
        assert!(buffer_contains(&buffer, ".jsonl"));
    }

    #[test]
    fn test_set_messages() {
        let mut viewer = SessionViewer::new();
        let messages = vec![
            r#"{"type":"user","message":{"content":"Hello"},"timestamp":"2024-01-01T00:00:00Z"}"#.to_string(),
            r#"{"type":"assistant","message":{"content":"Hi there"},"timestamp":"2024-01-01T00:01:00Z"}"#.to_string(),
        ];

        viewer.set_messages(messages.clone());
        // Test that messages are set and displayed
        let buffer = render_component(&mut viewer, 100, 30);
        assert!(buffer_contains(&buffer, "Session Messages"));
    }

    #[test]
    fn test_set_filtered_indices() {
        let mut viewer = SessionViewer::new();
        viewer.set_messages(vec![
            "msg1".to_string(),
            "msg2".to_string(),
            "msg3".to_string(),
        ]);

        viewer.set_filtered_indices(vec![0, 2]);
        // Just test that it doesn't crash
        let _buffer = render_component(&mut viewer, 80, 24);
    }

    #[test]
    fn test_metadata_display_vertical() {
        let mut viewer = SessionViewer::new();
        viewer.set_file_path(Some("/path/to/session.jsonl".to_string()));
        viewer.set_session_id(Some("session-123".to_string()));

        let buffer = render_component(&mut viewer, 80, 24);

        // Check that metadata is displayed vertically
        assert!(buffer_contains(&buffer, "Session: session-123"));
        assert!(buffer_contains(&buffer, "File: /path/to/session.jsonl"));

        // Check that they are on separate lines by examining the buffer content
        let content = buffer
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();

        // Find positions of Session and File labels
        let session_pos = content
            .find("Session: session-123")
            .expect("Session not found");
        let file_pos = content
            .find("File: /path/to/session.jsonl")
            .expect("File not found");

        // They should be on different lines (80 chars per line)
        let session_line = session_pos / 80;
        let file_line = file_pos / 80;
        assert!(
            file_line > session_line,
            "File should be on a line below Session"
        );
    }

    #[test]
    fn test_metadata_display_empty_file_path() {
        let mut viewer = SessionViewer::new();
        viewer.set_file_path(Some("".to_string()));
        viewer.set_session_id(Some("session-123".to_string()));

        let buffer = render_component(&mut viewer, 80, 24);

        // Check that metadata is displayed
        assert!(buffer_contains(&buffer, "Session: session-123"));
        assert!(buffer_contains(&buffer, "File:"));
    }

    #[test]
    fn test_metadata_display_none_file_path() {
        let mut viewer = SessionViewer::new();
        viewer.set_file_path(None);
        viewer.set_session_id(Some("session-123".to_string()));

        let buffer = render_component(&mut viewer, 80, 24);

        // Should only show Session, not File
        assert!(buffer_contains(&buffer, "Session: session-123"));
        assert!(!buffer_contains(&buffer, "File:"));
    }

    #[test]
    fn test_metadata_display_with_long_paths() {
        let mut viewer = SessionViewer::new();
        let long_path = "/very/long/path/that/should/wrap/around/the/screen/width/when/displayed/in/the/title/bar/session.jsonl";
        viewer.set_file_path(Some(long_path.to_string()));
        viewer.set_session_id(Some(
            "very-long-session-id-that-should-also-wrap-when-necessary-1234567890".to_string(),
        ));

        let buffer = render_component(&mut viewer, 80, 24);

        // Check that both are displayed (wrapping is handled by ratatui)
        assert!(buffer_contains(&buffer, "Session:"));
        assert!(buffer_contains(&buffer, "File:"));
    }

    #[test]
    fn test_metadata_display_real_path() {
        let mut viewer = SessionViewer::new();
        let real_path = "/Users/masatomokusaka/.claude/projects/-Users-masatomokusaka-src-github-com-clerk-clerk-playwright-nextjs/fb101a01-0e24-4a45-9e42-74117ebc20e6.jsonl";
        viewer.set_file_path(Some(real_path.to_string()));
        viewer.set_session_id(Some("fb101a01-0e24-4a45-9e42-74117ebc20e6".to_string()));

        let buffer = render_component(&mut viewer, 180, 24);

        // Check that both are displayed
        assert!(buffer_contains(
            &buffer,
            "Session: fb101a01-0e24-4a45-9e42-74117ebc20e6"
        ));
        assert!(buffer_contains(
            &buffer,
            "File: /Users/masatomokusaka/.claude/projects"
        ));
    }

    #[test]
    fn test_default_message_display() {
        let mut viewer = SessionViewer::new();
        let messages = vec![
            r#"{"type":"user","message":{"content":"Hello world"},"timestamp":"2024-01-01T00:00:00Z"}"#.to_string(),
            r#"{"type":"assistant","message":{"content":"Hi there!"},"timestamp":"2024-01-01T00:01:00Z"}"#.to_string(),
        ];

        viewer.set_messages(messages);
        let buffer = render_component(&mut viewer, 100, 30);

        // Messages should be displayed by default
        assert!(buffer_contains(&buffer, "user"));
        assert!(buffer_contains(&buffer, "Hello world"));
        assert!(buffer_contains(&buffer, "assistant"));
        assert!(buffer_contains(&buffer, "Hi there!"));
    }

    #[test]
    fn test_empty_messages_display() {
        let mut viewer = SessionViewer::new();
        let buffer = render_component(&mut viewer, 80, 24);

        assert!(buffer_contains(&buffer, "No messages in session"));
    }

    #[test]
    fn test_navigation() {
        let mut viewer = SessionViewer::new();
        viewer.set_messages(vec![
            r#"{"type":"user","message":{"content":"message 1"},"timestamp":"2024-01-01T00:00:00Z"}"#.to_string(),
            r#"{"type":"user","message":{"content":"message 2"},"timestamp":"2024-01-01T00:00:01Z"}"#.to_string(),
            r#"{"type":"user","message":{"content":"message 3"},"timestamp":"2024-01-01T00:00:02Z"}"#.to_string(),
        ]);

        // Test down navigation - should return SessionNavigated message
        let msg = viewer.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::empty()));
        assert!(matches!(msg, Some(Message::SessionNavigated(_, _))));

        // Test up navigation - should return SessionNavigated message
        let msg = viewer.handle_key(KeyEvent::new(KeyCode::Up, KeyModifiers::empty()));
        assert!(matches!(msg, Some(Message::SessionNavigated(_, _))));
    }

    #[test]
    fn test_search_mode() {
        let mut viewer = SessionViewer::new();

        // Enter search mode
        let msg = viewer.handle_key(KeyEvent::new(KeyCode::Char('/'), KeyModifiers::empty()));
        assert!(msg.is_none());

        // Type in search
        let msg = viewer.handle_key(KeyEvent::new(KeyCode::Char('t'), KeyModifiers::empty()));
        assert!(matches!(msg, Some(Message::SessionQueryChanged(q)) if q == "t"));

        // Cancel search
        let msg = viewer.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::empty()));
        assert!(matches!(msg, Some(Message::SessionQueryChanged(q)) if q.is_empty()));
    }

    #[test]
    fn test_copy_operations() {
        let mut viewer = SessionViewer::new();
        viewer.set_messages(vec![
            r#"{"type":"user","message":{"content":"test"}}"#.to_string(),
        ]);
        viewer.set_session_id(Some("session-123".to_string()));
        viewer.set_file_path(Some("/path/to/session.jsonl".to_string()));

        // Test copy single message
        let msg = viewer.handle_key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::empty()));
        assert!(matches!(
            msg,
            Some(Message::CopyToClipboard(CopyContent::JsonData(_)))
        ));

        // Test copy all messages
        let msg = viewer.handle_key(KeyEvent::new(KeyCode::Char('C'), KeyModifiers::empty()));
        assert!(matches!(
            msg,
            Some(Message::CopyToClipboard(CopyContent::JsonData(_)))
        ));

        // Test copy session ID
        let msg = viewer.handle_key(KeyEvent::new(KeyCode::Char('i'), KeyModifiers::empty()));
        assert!(
            matches!(msg, Some(Message::CopyToClipboard(CopyContent::SessionId(id))) if id == "session-123")
        );

        // Test copy session ID with uppercase
        let msg = viewer.handle_key(KeyEvent::new(KeyCode::Char('I'), KeyModifiers::empty()));
        assert!(
            matches!(msg, Some(Message::CopyToClipboard(CopyContent::SessionId(id))) if id == "session-123")
        );

        // Test copy file path
        let msg = viewer.handle_key(KeyEvent::new(KeyCode::Char('f'), KeyModifiers::empty()));
        assert!(
            matches!(msg, Some(Message::CopyToClipboard(CopyContent::FilePath(path))) if path == "/path/to/session.jsonl")
        );

        // Test copy file path with uppercase
        let msg = viewer.handle_key(KeyEvent::new(KeyCode::Char('F'), KeyModifiers::empty()));
        assert!(
            matches!(msg, Some(Message::CopyToClipboard(CopyContent::FilePath(path))) if path == "/path/to/session.jsonl")
        );
    }

    #[test]
    fn test_copy_project_path() {
        let mut viewer = SessionViewer::new();
        viewer.set_messages(vec![
            r#"{"type":"user","message":{"content":"test"}}"#.to_string(),
        ]);
        viewer.set_file_path(Some(
            "/Users/masatomokusaka/.claude/projects/-Users-project-name/session.jsonl".to_string(),
        ));

        // Test copy project path
        let msg = viewer.handle_key(KeyEvent::new(KeyCode::Char('p'), KeyModifiers::empty()));
        assert!(
            matches!(msg, Some(Message::CopyToClipboard(CopyContent::ProjectPath(path))) if path == "/Users/project/name")
        );

        // Test copy project path with uppercase
        let msg = viewer.handle_key(KeyEvent::new(KeyCode::Char('P'), KeyModifiers::empty()));
        assert!(
            matches!(msg, Some(Message::CopyToClipboard(CopyContent::ProjectPath(path))) if path == "/Users/project/name")
        );
    }

    #[test]
    fn test_copy_project_path_without_path() {
        let mut viewer = SessionViewer::new();

        // No project path set (no file path)
        let msg = viewer.handle_key(KeyEvent::new(KeyCode::Char('p'), KeyModifiers::empty()));
        assert!(msg.is_none());
    }

    #[test]
    fn test_extract_project_path() {
        let mut viewer = SessionViewer::new();

        // Test with typical Claude project path
        viewer.set_file_path(Some("/Users/masatomokusaka/.claude/projects/-Users-masatomokusaka-src-github-com-clerk-clerk-playwright-nextjs/fb101a01-0e24-4a45-9e42-74117ebc20e6.jsonl".to_string()));

        let buffer = render_component(&mut viewer, 180, 24);
        assert!(buffer_contains(
            &buffer,
            "Project: /Users/masatomokusaka/src/github/com/clerk/clerk/playwright/nextjs"
        ));

        // Test with shorter path
        viewer.set_file_path(Some(
            "/home/user/.claude/projects/-tmp-test/session.jsonl".to_string(),
        ));
        let buffer = render_component(&mut viewer, 100, 24);
        assert!(buffer_contains(&buffer, "Project: /tmp/test"));

        // Test with no project path (invalid format)
        viewer.set_file_path(Some("/invalid/path/file.jsonl".to_string()));
        let buffer = render_component(&mut viewer, 100, 24);
        // Should not show Project: line when extraction fails
        assert!(!buffer_contains(&buffer, "Project:"));
    }

    #[test]
    fn test_copy_session_id_without_id() {
        let mut viewer = SessionViewer::new();
        // No session ID set

        let msg = viewer.handle_key(KeyEvent::new(KeyCode::Char('i'), KeyModifiers::empty()));
        assert!(msg.is_none());
    }

    #[test]
    fn test_copy_file_path_without_path() {
        let mut viewer = SessionViewer::new();
        // No file path set

        let msg = viewer.handle_key(KeyEvent::new(KeyCode::Char('f'), KeyModifiers::empty()));
        assert!(msg.is_none());
    }

    #[test]
    fn test_toggle_order() {
        let mut viewer = SessionViewer::new();

        let msg = viewer.handle_key(KeyEvent::new(KeyCode::Char('o'), KeyModifiers::empty()));
        assert!(matches!(msg, Some(Message::ToggleSessionOrder)));
    }

    #[test]
    fn test_exit_to_search() {
        let mut viewer = SessionViewer::new();

        // Test ESC key
        let msg = viewer.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::empty()));
        assert!(matches!(msg, Some(Message::ExitToSearch)));
    }

    #[test]
    fn test_json_message_parsing() {
        let mut viewer = SessionViewer::new();
        let messages = vec![
            r#"{"type":"user","message":{"content":"Hello world"},"timestamp":"2024-01-01T12:00:00Z"}"#.to_string(),
            r#"{"type":"assistant","message":{"content":"Hi there!"},"timestamp":"2024-01-01T12:01:00Z"}"#.to_string(),
            "Invalid JSON message".to_string(),
        ];

        viewer.set_messages(messages);
        let buffer = render_component(&mut viewer, 120, 30);

        // Should display parsed messages with role and time
        // Note: The new ListViewer displays role without brackets and padded to 10 chars
        assert!(buffer_contains(&buffer, "user"));
        assert!(buffer_contains(&buffer, "01/01 12:00"));
        assert!(buffer_contains(&buffer, "Hello world"));
        assert!(buffer_contains(&buffer, "assistant"));
        assert!(buffer_contains(&buffer, "Hi there!"));
        // Invalid JSON messages are filtered out in the new implementation
    }

    #[test]
    fn test_order_display() {
        let mut viewer = SessionViewer::new();
        viewer.set_order(Some(SessionOrder::Ascending));

        let buffer = render_component(&mut viewer, 80, 24);
        assert!(buffer_contains(&buffer, "Order: Ascending"));

        viewer.set_order(Some(SessionOrder::Descending));
        let buffer = render_component(&mut viewer, 80, 24);
        assert!(buffer_contains(&buffer, "Order: Descending"));

        viewer.set_order(Some(SessionOrder::Original));
        let buffer = render_component(&mut viewer, 80, 24);
        assert!(buffer_contains(&buffer, "Order: Original"));

        viewer.set_order(None);
        let buffer = render_component(&mut viewer, 80, 24);
        assert!(buffer_contains(&buffer, "Order: Default"));
    }

    #[test]
    fn test_message_display() {
        let mut viewer = SessionViewer::new();
        viewer.set_message(Some("✓ Copied session ID".to_string()));

        let buffer = render_component(&mut viewer, 80, 24);
        assert!(buffer_contains(&buffer, "✓ Copied session ID"));
    }

    #[test]
    fn test_copy_with_message_feedback() {
        let mut viewer = SessionViewer::new();
        viewer.set_session_id(Some("session-123".to_string()));
        viewer.set_file_path(Some("/path/to/file.jsonl".to_string()));

        // Test session ID copy
        let msg = viewer.handle_key(KeyEvent::new(KeyCode::Char('i'), KeyModifiers::empty()));
        assert!(
            matches!(msg, Some(Message::CopyToClipboard(CopyContent::SessionId(id))) if id == "session-123")
        );

        // Simulate the message being set after copy
        viewer.set_message(Some("✓ Copied session ID".to_string()));
        let buffer = render_component(&mut viewer, 80, 24);
        assert!(buffer_contains(&buffer, "✓ Copied session ID"));

        // Test file path copy
        let msg = viewer.handle_key(KeyEvent::new(KeyCode::Char('f'), KeyModifiers::empty()));
        assert!(
            matches!(msg, Some(Message::CopyToClipboard(CopyContent::FilePath(path))) if path == "/path/to/file.jsonl")
        );

        // Simulate the message being set after copy
        viewer.set_message(Some("✓ Copied file path".to_string()));
        let buffer = render_component(&mut viewer, 80, 24);
        assert!(buffer_contains(&buffer, "✓ Copied file path"));
    }

    #[test]
    fn test_truncation_toggle() {
        let mut viewer = SessionViewer::new();
        let messages = vec![
            r#"{"type":"user","message":{"content":"This is a very long message that should be truncated when truncation is enabled but shown in full when truncation is disabled"},"timestamp":"2024-01-01T00:00:00Z"}"#.to_string(),
        ];

        viewer.set_messages(messages);

        // Test with truncation enabled (default)
        viewer.set_truncation_enabled(true);
        let buffer = render_component(&mut viewer, 80, 24);
        // The message should be truncated (ListViewer shows truncated line)
        assert!(buffer_contains(&buffer, "user"));

        // Test with truncation disabled
        viewer.set_truncation_enabled(false);
        let buffer = render_component(&mut viewer, 80, 24);
        // The message should show in full
        assert!(buffer_contains(&buffer, "user"));
        // Since we can't easily check for the full message content due to wrapping,
        // at least verify the method doesn't crash
    }

    #[test]
    fn test_search_bar_rendering() {
        let mut viewer = SessionViewer::new();
        // Enter search mode first
        viewer.handle_key(KeyEvent::new(KeyCode::Char('/'), KeyModifiers::empty()));
        // Type some text
        viewer.handle_key(KeyEvent::new(KeyCode::Char('t'), KeyModifiers::empty()));
        viewer.handle_key(KeyEvent::new(KeyCode::Char('e'), KeyModifiers::empty()));
        viewer.handle_key(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::empty()));
        viewer.handle_key(KeyEvent::new(KeyCode::Char('t'), KeyModifiers::empty()));

        let buffer = render_component(&mut viewer, 80, 24);
        assert!(buffer_contains(&buffer, "test"));
        assert!(buffer_contains(&buffer, "Search in session"));
    }

    #[test]
    fn test_empty_filtered_results() {
        let mut viewer = SessionViewer::new();
        viewer.set_messages(vec!["message 1".to_string(), "message 2".to_string()]);
        viewer.set_filtered_indices(vec![]); // No matches

        let buffer = render_component(&mut viewer, 80, 24);
        // Should handle empty filtered results gracefully
        assert!(buffer_contains(&buffer, "Session Messages"));
    }

    fn create_key_event_with_modifiers(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
        KeyEvent {
            code,
            modifiers,
            kind: crossterm::event::KeyEventKind::Press,
            state: crossterm::event::KeyEventState::empty(),
        }
    }

    #[test]
    fn test_search_shortcuts_ctrl_a() {
        let mut viewer = SessionViewer::new();
        viewer.start_search();
        viewer.set_query("hello world".to_string());
        viewer.set_cursor_position(11); // At end

        // Ctrl+A - Move to beginning
        let msg = viewer.handle_key(create_key_event_with_modifiers(
            KeyCode::Char('a'),
            KeyModifiers::CONTROL,
        ));
        assert!(msg.is_none());
        assert_eq!(viewer.cursor_position(), 0);
    }

    #[test]
    fn test_search_shortcuts_ctrl_e() {
        let mut viewer = SessionViewer::new();
        viewer.start_search();
        viewer.set_query("hello world".to_string());
        viewer.set_cursor_position(0); // At beginning

        // Ctrl+E - Move to end
        let msg = viewer.handle_key(create_key_event_with_modifiers(
            KeyCode::Char('e'),
            KeyModifiers::CONTROL,
        ));
        assert!(msg.is_none());
        assert_eq!(viewer.cursor_position(), 11); // "hello world" has 11 characters
    }

    #[test]
    fn test_search_shortcuts_ctrl_b() {
        let mut viewer = SessionViewer::new();
        viewer.start_search();
        viewer.set_query("hello".to_string());
        viewer.set_cursor_position(3);

        // Ctrl+B - Move backward
        let msg = viewer.handle_key(create_key_event_with_modifiers(
            KeyCode::Char('b'),
            KeyModifiers::CONTROL,
        ));
        assert!(msg.is_none());
        assert_eq!(viewer.cursor_position(), 2);

        // At beginning, should not move
        viewer.set_cursor_position(0);
        let msg = viewer.handle_key(create_key_event_with_modifiers(
            KeyCode::Char('b'),
            KeyModifiers::CONTROL,
        ));
        assert!(msg.is_none());
        assert_eq!(viewer.cursor_position(), 0);
    }

    #[test]
    fn test_search_shortcuts_ctrl_f() {
        let mut viewer = SessionViewer::new();
        viewer.start_search();
        viewer.set_query("hello".to_string());
        viewer.set_cursor_position(2);

        // Ctrl+F - Move forward
        let msg = viewer.handle_key(create_key_event_with_modifiers(
            KeyCode::Char('f'),
            KeyModifiers::CONTROL,
        ));
        assert!(msg.is_none());
        assert_eq!(viewer.cursor_position(), 3);

        // At end, should not move
        viewer.set_cursor_position(5);
        let msg = viewer.handle_key(create_key_event_with_modifiers(
            KeyCode::Char('f'),
            KeyModifiers::CONTROL,
        ));
        assert!(msg.is_none());
        assert_eq!(viewer.cursor_position(), 5);
    }

    #[test]
    fn test_search_shortcuts_ctrl_h() {
        let mut viewer = SessionViewer::new();
        viewer.start_search();
        viewer.set_query("hello".to_string());
        viewer.set_cursor_position(5);

        // Ctrl+H - Delete before cursor
        let msg = viewer.handle_key(create_key_event_with_modifiers(
            KeyCode::Char('h'),
            KeyModifiers::CONTROL,
        ));
        assert!(matches!(msg, Some(Message::SessionQueryChanged(q)) if q == "hell"));
        assert_eq!(viewer.query(), "hell");
        assert_eq!(viewer.cursor_position(), 4);

        // At beginning, should do nothing
        viewer.set_cursor_position(0);
        let msg = viewer.handle_key(create_key_event_with_modifiers(
            KeyCode::Char('h'),
            KeyModifiers::CONTROL,
        ));
        assert!(msg.is_none());
        assert_eq!(viewer.query(), "hell");
    }

    #[test]
    fn test_search_shortcuts_ctrl_d() {
        let mut viewer = SessionViewer::new();
        viewer.start_search();
        viewer.set_query("hello".to_string());
        viewer.set_cursor_position(0);

        // Ctrl+D - Delete under cursor
        let msg = viewer.handle_key(create_key_event_with_modifiers(
            KeyCode::Char('d'),
            KeyModifiers::CONTROL,
        ));
        assert!(matches!(msg, Some(Message::SessionQueryChanged(q)) if q == "ello"));
        assert_eq!(viewer.query(), "ello");
        assert_eq!(viewer.cursor_position(), 0);

        // At end, should do nothing
        viewer.set_cursor_position(4);
        let msg = viewer.handle_key(create_key_event_with_modifiers(
            KeyCode::Char('d'),
            KeyModifiers::CONTROL,
        ));
        assert!(msg.is_none());
        assert_eq!(viewer.query(), "ello");
    }

    #[test]
    fn test_search_shortcuts_ctrl_w() {
        let mut viewer = SessionViewer::new();
        viewer.start_search();
        viewer.set_query("hello world test".to_string());
        viewer.set_cursor_position(16);

        // Ctrl+W - Delete word before cursor
        let msg = viewer.handle_key(create_key_event_with_modifiers(
            KeyCode::Char('w'),
            KeyModifiers::CONTROL,
        ));
        assert!(matches!(msg, Some(Message::SessionQueryChanged(q)) if q == "hello world "));
        assert_eq!(viewer.cursor_position(), 12);

        // Delete another word
        let msg = viewer.handle_key(create_key_event_with_modifiers(
            KeyCode::Char('w'),
            KeyModifiers::CONTROL,
        ));
        assert!(matches!(msg, Some(Message::SessionQueryChanged(q)) if q == "hello "));
        assert_eq!(viewer.cursor_position(), 6);

        // At beginning, should do nothing
        viewer.set_cursor_position(0);
        let msg = viewer.handle_key(create_key_event_with_modifiers(
            KeyCode::Char('w'),
            KeyModifiers::CONTROL,
        ));
        assert!(msg.is_none());
    }

    #[test]
    fn test_search_shortcuts_ctrl_u() {
        let mut viewer = SessionViewer::new();
        viewer.start_search();
        viewer.set_query("hello world".to_string());
        viewer.set_cursor_position(6);

        // Ctrl+U - Delete to beginning
        let msg = viewer.handle_key(create_key_event_with_modifiers(
            KeyCode::Char('u'),
            KeyModifiers::CONTROL,
        ));
        assert!(matches!(msg, Some(Message::SessionQueryChanged(q)) if q == "world"));
        assert_eq!(viewer.cursor_position(), 0);

        // At beginning, should do nothing
        let msg = viewer.handle_key(create_key_event_with_modifiers(
            KeyCode::Char('u'),
            KeyModifiers::CONTROL,
        ));
        assert!(msg.is_none());
    }

    #[test]
    fn test_search_shortcuts_ctrl_k() {
        let mut viewer = SessionViewer::new();
        viewer.start_search();
        viewer.set_query("hello world".to_string());
        viewer.set_cursor_position(6);

        // Ctrl+K - Delete to end
        let msg = viewer.handle_key(create_key_event_with_modifiers(
            KeyCode::Char('k'),
            KeyModifiers::CONTROL,
        ));
        assert!(matches!(msg, Some(Message::SessionQueryChanged(q)) if q == "hello "));
        assert_eq!(viewer.cursor_position(), 6);

        // At end, should do nothing
        let msg = viewer.handle_key(create_key_event_with_modifiers(
            KeyCode::Char('k'),
            KeyModifiers::CONTROL,
        ));
        assert!(msg.is_none());
    }

    #[test]
    fn test_search_shortcuts_alt_b() {
        let mut viewer = SessionViewer::new();
        viewer.start_search();
        viewer.set_query("hello world test".to_string());
        viewer.set_cursor_position(16);

        // Alt+B - Move backward by word
        let msg = viewer.handle_key(create_key_event_with_modifiers(
            KeyCode::Char('b'),
            KeyModifiers::ALT,
        ));
        assert!(msg.is_none());
        assert_eq!(viewer.cursor_position(), 12); // Beginning of "test"

        // Move backward again
        let msg = viewer.handle_key(create_key_event_with_modifiers(
            KeyCode::Char('b'),
            KeyModifiers::ALT,
        ));
        assert!(msg.is_none());
        assert_eq!(viewer.cursor_position(), 6); // Beginning of "world"
    }

    #[test]
    fn test_search_shortcuts_alt_f() {
        let mut viewer = SessionViewer::new();
        viewer.start_search();
        viewer.set_query("hello world test".to_string());
        viewer.set_cursor_position(0);

        // Alt+F - Move forward by word
        let msg = viewer.handle_key(create_key_event_with_modifiers(
            KeyCode::Char('f'),
            KeyModifiers::ALT,
        ));
        assert!(msg.is_none());
        assert_eq!(viewer.cursor_position(), 6); // After "hello "

        // Move forward again
        let msg = viewer.handle_key(create_key_event_with_modifiers(
            KeyCode::Char('f'),
            KeyModifiers::ALT,
        ));
        assert!(msg.is_none());
        assert_eq!(viewer.cursor_position(), 12); // After "world "
    }

    #[test]
    fn test_search_shortcuts_with_unicode() {
        let mut viewer = SessionViewer::new();
        viewer.start_search();
        viewer.set_query("こんにちは 世界 🌍".to_string());
        viewer.set_cursor_position(10); // At end (10 characters total)

        // Test Ctrl+W with unicode
        let msg = viewer.handle_key(create_key_event_with_modifiers(
            KeyCode::Char('w'),
            KeyModifiers::CONTROL,
        ));
        assert!(matches!(msg, Some(Message::SessionQueryChanged(q)) if q == "こんにちは 世界 "));

        // Test Alt+B with unicode
        let msg = viewer.handle_key(create_key_event_with_modifiers(
            KeyCode::Char('b'),
            KeyModifiers::ALT,
        ));
        assert!(msg.is_none());
        assert_eq!(viewer.cursor_position(), 6); // Beginning of "世界"

        // Test Ctrl+U with unicode
        let msg = viewer.handle_key(create_key_event_with_modifiers(
            KeyCode::Char('u'),
            KeyModifiers::CONTROL,
        ));
        assert!(matches!(msg, Some(Message::SessionQueryChanged(q)) if q == "世界 "));
        assert_eq!(viewer.cursor_position(), 0);
    }

    #[test]
    fn test_search_mode_character_input() {
        let mut viewer = SessionViewer::new();
        viewer.start_search();
        viewer.set_query("hello".to_string());
        viewer.set_cursor_position(0);

        // Type at beginning
        let msg = viewer.handle_key(KeyEvent::new(KeyCode::Char('X'), KeyModifiers::empty()));
        assert!(matches!(msg, Some(Message::SessionQueryChanged(q)) if q == "Xhello"));
        assert_eq!(viewer.cursor_position(), 1);
    }

    #[test]
    fn test_control_chars_dont_insert() {
        let mut viewer = SessionViewer::new();
        viewer.start_search();
        viewer.set_query("hello".to_string());
        viewer.set_cursor_position(5);

        // Control+character combinations should not insert the character
        let msg = viewer.handle_key(create_key_event_with_modifiers(
            KeyCode::Char('x'),
            KeyModifiers::CONTROL,
        ));
        assert!(msg.is_none());
        assert_eq!(viewer.query(), "hello");

        // Alt+character combinations should not insert the character
        let msg = viewer.handle_key(create_key_event_with_modifiers(
            KeyCode::Char('x'),
            KeyModifiers::ALT,
        ));
        assert!(msg.is_none());
        assert_eq!(viewer.query(), "hello");
    }

    #[test]
    fn test_search_mode_arrow_keys() {
        let mut viewer = SessionViewer::new();
        viewer.start_search();
        viewer.set_query("hello world".to_string());
        viewer.set_cursor_position(11);

        // Move cursor left
        let msg = viewer.handle_key(KeyEvent::new(KeyCode::Left, KeyModifiers::empty()));
        assert!(msg.is_none());
        assert_eq!(viewer.cursor_position(), 10);

        // Move right
        let msg = viewer.handle_key(KeyEvent::new(KeyCode::Right, KeyModifiers::empty()));
        assert!(msg.is_none());
        assert_eq!(viewer.cursor_position(), 11);

        // Move to beginning with Home
        let msg = viewer.handle_key(KeyEvent::new(KeyCode::Home, KeyModifiers::empty()));
        assert!(msg.is_none());
        assert_eq!(viewer.cursor_position(), 0);

        // Move to end with End
        let msg = viewer.handle_key(KeyEvent::new(KeyCode::End, KeyModifiers::empty()));
        assert!(msg.is_none());
        assert_eq!(viewer.cursor_position(), 11);
    }

    #[test]
    fn test_search_mode_backspace_and_delete() {
        let mut viewer = SessionViewer::new();
        viewer.start_search();
        viewer.set_query("hello".to_string());
        viewer.set_cursor_position(5);

        // Test backspace
        let msg = viewer.handle_key(KeyEvent::new(KeyCode::Backspace, KeyModifiers::empty()));
        assert!(matches!(msg, Some(Message::SessionQueryChanged(q)) if q == "hell"));
        assert_eq!(viewer.cursor_position(), 4);

        // Test delete at beginning
        viewer.set_cursor_position(0);
        let msg = viewer.handle_key(KeyEvent::new(KeyCode::Delete, KeyModifiers::empty()));
        assert!(matches!(msg, Some(Message::SessionQueryChanged(q)) if q == "ell"));
        assert_eq!(viewer.cursor_position(), 0);
    }

    #[test]
    fn test_search_mode_stays_active_on_empty_backspace() {
        let mut viewer = SessionViewer::new();
        viewer.start_search();

        // Type a single character
        viewer.handle_key(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::empty()));
        assert_eq!(viewer.query(), "a");

        // Backspace to empty - should stay in search mode
        let msg = viewer.handle_key(KeyEvent::new(KeyCode::Backspace, KeyModifiers::empty()));
        assert!(matches!(msg, Some(Message::SessionQueryChanged(q)) if q.is_empty()));

        // Should still be in search mode - try typing again
        let msg = viewer.handle_key(KeyEvent::new(KeyCode::Char('b'), KeyModifiers::empty()));
        assert!(matches!(msg, Some(Message::SessionQueryChanged(q)) if q == "b"));

        // Verify we're still in search mode - ESC should exit
        let msg = viewer.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::empty()));
        assert!(matches!(msg, Some(Message::SessionQueryChanged(q)) if q.is_empty()));
    }

    #[test]
    fn test_search_mode_backspace_on_empty_query() {
        let mut viewer = SessionViewer::new();
        viewer.start_search();

        // Backspace on empty query should do nothing but stay in search mode
        let msg = viewer.handle_key(KeyEvent::new(KeyCode::Backspace, KeyModifiers::empty()));
        assert!(msg.is_none());

        // Should still be in search mode - can type
        let msg = viewer.handle_key(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::empty()));
        assert!(matches!(msg, Some(Message::SessionQueryChanged(q)) if q == "x"));
    }

    #[test]
    fn test_navigation_moves_selection() {
        let mut viewer = SessionViewer::new();
        viewer.set_messages(vec![
            r#"{"type":"user","message":{"content":"message 1"},"timestamp":"2024-01-01T00:00:00Z"}"#.to_string(),
            r#"{"type":"user","message":{"content":"message 2"},"timestamp":"2024-01-01T00:00:01Z"}"#.to_string(),
            r#"{"type":"user","message":{"content":"message 3"},"timestamp":"2024-01-01T00:00:02Z"}"#.to_string(),
            r#"{"type":"user","message":{"content":"message 4"},"timestamp":"2024-01-01T00:00:03Z"}"#.to_string(),
            r#"{"type":"user","message":{"content":"message 5"},"timestamp":"2024-01-01T00:00:04Z"}"#.to_string(),
        ]);

        // Initially at index 0
        assert_eq!(viewer.list_viewer.selected_index, 0);

        // Move down
        viewer.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::empty()));
        assert_eq!(viewer.list_viewer.selected_index, 1);

        // Move down again
        viewer.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::empty()));
        assert_eq!(viewer.list_viewer.selected_index, 2);

        // Move up
        viewer.handle_key(KeyEvent::new(KeyCode::Up, KeyModifiers::empty()));
        assert_eq!(viewer.list_viewer.selected_index, 1);

        // Move to end
        viewer.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::empty()));
        viewer.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::empty()));
        viewer.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::empty()));
        assert_eq!(viewer.list_viewer.selected_index, 4);

        // Try to move past end
        viewer.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::empty()));
        assert_eq!(viewer.list_viewer.selected_index, 4); // Should stay at 4
    }

    #[test]
    fn test_search_mode_navigation() {
        let mut viewer = SessionViewer::new();
        viewer.set_messages(vec![
            r#"{"type":"user","message":{"content":"message 1"},"timestamp":"2024-01-01T00:00:00Z"}"#.to_string(),
            r#"{"type":"user","message":{"content":"message 2"},"timestamp":"2024-01-01T00:00:01Z"}"#.to_string(),
            r#"{"type":"user","message":{"content":"message 3"},"timestamp":"2024-01-01T00:00:02Z"}"#.to_string(),
        ]);

        // Enter search mode
        viewer.handle_key(KeyEvent::new(KeyCode::Char('/'), KeyModifiers::empty()));

        // Navigate while in search mode
        assert_eq!(viewer.list_viewer.selected_index, 0);

        viewer.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::empty()));
        assert_eq!(viewer.list_viewer.selected_index, 1);

        viewer.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::empty()));
        assert_eq!(viewer.list_viewer.selected_index, 2);

        // Test Ctrl+P/N
        viewer.handle_key(create_key_event_with_modifiers(
            KeyCode::Char('p'),
            KeyModifiers::CONTROL,
        ));
        assert_eq!(viewer.list_viewer.selected_index, 1);

        viewer.handle_key(create_key_event_with_modifiers(
            KeyCode::Char('n'),
            KeyModifiers::CONTROL,
        ));
        assert_eq!(viewer.list_viewer.selected_index, 2);
    }

    #[test]
    fn test_set_messages_preserves_selection_when_unchanged() {
        let mut viewer = SessionViewer::new();
        let messages = vec![
            r#"{"type":"user","message":{"content":"message 1"},"timestamp":"2024-01-01T00:00:00Z"}"#.to_string(),
            r#"{"type":"user","message":{"content":"message 2"},"timestamp":"2024-01-01T00:00:01Z"}"#.to_string(),
            r#"{"type":"user","message":{"content":"message 3"},"timestamp":"2024-01-01T00:00:02Z"}"#.to_string(),
        ];

        // Set messages initially
        viewer.set_messages(messages.clone());
        assert_eq!(viewer.list_viewer.selected_index, 0);

        // Move selection down
        viewer.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::empty()));
        viewer.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::empty()));
        assert_eq!(viewer.list_viewer.selected_index, 2);

        // Set the same messages again - selection should be preserved
        viewer.set_messages(messages);
        assert_eq!(viewer.list_viewer.selected_index, 2);
    }

    #[test]
    fn test_set_messages_resets_selection_when_changed() {
        let mut viewer = SessionViewer::new();
        let messages1 = vec![
            r#"{"type":"user","message":{"content":"message 1"},"timestamp":"2024-01-01T00:00:00Z"}"#.to_string(),
            r#"{"type":"user","message":{"content":"message 2"},"timestamp":"2024-01-01T00:00:01Z"}"#.to_string(),
        ];

        // Set messages initially
        viewer.set_messages(messages1);
        assert_eq!(viewer.list_viewer.selected_index, 0);

        // Move selection down
        viewer.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::empty()));
        assert_eq!(viewer.list_viewer.selected_index, 1);

        // Set different messages - selection should reset
        let messages2 = vec![
            r#"{"type":"user","message":{"content":"new message 1"},"timestamp":"2024-01-01T00:00:00Z"}"#.to_string(),
            r#"{"type":"user","message":{"content":"new message 2"},"timestamp":"2024-01-01T00:00:01Z"}"#.to_string(),
        ];
        viewer.set_messages(messages2);
        assert_eq!(viewer.list_viewer.selected_index, 0);
    }

    #[test]
    fn test_ctrl_p_n_in_normal_mode() {
        let mut viewer = SessionViewer::new();
        viewer.set_messages(vec![
            r#"{"type":"user","message":{"content":"message 1"},"timestamp":"2024-01-01T00:00:00Z"}"#.to_string(),
            r#"{"type":"user","message":{"content":"message 2"},"timestamp":"2024-01-01T00:00:01Z"}"#.to_string(),
            r#"{"type":"user","message":{"content":"message 3"},"timestamp":"2024-01-01T00:00:02Z"}"#.to_string(),
        ]);

        // Initially at index 0
        assert_eq!(viewer.list_viewer.selected_index, 0);

        // Ctrl+N to move down
        let msg = viewer.handle_key(create_key_event_with_modifiers(
            KeyCode::Char('n'),
            KeyModifiers::CONTROL,
        ));
        assert!(matches!(msg, Some(Message::SessionNavigated(_, _))));
        assert_eq!(viewer.list_viewer.selected_index, 1);

        // Ctrl+N again
        let msg = viewer.handle_key(create_key_event_with_modifiers(
            KeyCode::Char('n'),
            KeyModifiers::CONTROL,
        ));
        assert!(matches!(msg, Some(Message::SessionNavigated(_, _))));
        assert_eq!(viewer.list_viewer.selected_index, 2);

        // Ctrl+P to move up
        let msg = viewer.handle_key(create_key_event_with_modifiers(
            KeyCode::Char('p'),
            KeyModifiers::CONTROL,
        ));
        assert!(matches!(msg, Some(Message::SessionNavigated(_, _))));
        assert_eq!(viewer.list_viewer.selected_index, 1);
    }

    #[test]
    fn test_ctrl_u_d_navigation_normal_mode() {
        let mut viewer = SessionViewer::new();
        let mut messages = vec![];
        for i in 0..30 {
            messages.push(format!(
                r#"{{"type":"user","message":{{"content":"message {}"}},"timestamp":"2024-01-01T00:00:{:02}Z"}}"#,
                i + 1, i
            ));
        }
        viewer.set_messages(messages);

        // Initially at index 0
        assert_eq!(viewer.list_viewer.selected_index, 0);

        // Ctrl+D to move down half page
        let msg = viewer.handle_key(create_key_event_with_modifiers(
            KeyCode::Char('d'),
            KeyModifiers::CONTROL,
        ));
        assert!(matches!(msg, Some(Message::SessionNavigated)));
        // The exact position depends on viewport height, but should have moved down
        let first_pos = viewer.list_viewer.selected_index;
        assert!(first_pos > 0);

        // Ctrl+D again
        let msg = viewer.handle_key(create_key_event_with_modifiers(
            KeyCode::Char('d'),
            KeyModifiers::CONTROL,
        ));
        assert!(matches!(msg, Some(Message::SessionNavigated)));
        let second_pos = viewer.list_viewer.selected_index;
        assert!(second_pos > first_pos);

        // Navigate near the end
        viewer.list_viewer.selected_index = 25;

        // Ctrl+D should go to last item
        let msg = viewer.handle_key(create_key_event_with_modifiers(
            KeyCode::Char('d'),
            KeyModifiers::CONTROL,
        ));
        assert!(matches!(msg, Some(Message::SessionNavigated)));
        assert_eq!(viewer.list_viewer.selected_index, 29);

        // Can't move down from last item
        let msg = viewer.handle_key(create_key_event_with_modifiers(
            KeyCode::Char('d'),
            KeyModifiers::CONTROL,
        ));
        assert!(matches!(msg, Some(Message::SessionNavigated)));
        assert_eq!(viewer.list_viewer.selected_index, 29);

        // Ctrl+U to move up half page
        let msg = viewer.handle_key(create_key_event_with_modifiers(
            KeyCode::Char('u'),
            KeyModifiers::CONTROL,
        ));
        assert!(matches!(msg, Some(Message::SessionNavigated)));
        assert!(viewer.list_viewer.selected_index < 29);

        // Move to position 5
        viewer.list_viewer.selected_index = 5;

        // Ctrl+U should go to first item
        let msg = viewer.handle_key(create_key_event_with_modifiers(
            KeyCode::Char('u'),
            KeyModifiers::CONTROL,
        ));
        assert!(matches!(msg, Some(Message::SessionNavigated)));
        assert_eq!(viewer.list_viewer.selected_index, 0);

        // Can't move up from first item
        let msg = viewer.handle_key(create_key_event_with_modifiers(
            KeyCode::Char('u'),
            KeyModifiers::CONTROL,
        ));
        assert!(matches!(msg, Some(Message::SessionNavigated)));
        assert_eq!(viewer.list_viewer.selected_index, 0);
    }

    #[test]
    fn test_role_filter_toggle() {
        let mut viewer = SessionViewer::new();

        // Tab key toggles role filter
        let msg = viewer.handle_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::empty()));
        assert!(matches!(msg, Some(Message::ToggleSessionRoleFilter)));

        // Tab key should return the message for state management to handle
    }

    #[test]
    fn test_role_filter_display() {
        let mut viewer = SessionViewer::new();
        viewer.set_messages(vec![
            r#"{"type":"user","message":{"content":"Hello"},"timestamp":"2024-01-01T00:00:00Z"}"#.to_string(),
            r#"{"type":"assistant","message":{"content":"Hi there"},"timestamp":"2024-01-01T00:01:00Z"}"#.to_string(),
        ]);

        // Test without role filter
        viewer.set_role_filter(None);
        let buffer = render_component(&mut viewer, 100, 30);
        assert!(!buffer_contains(&buffer, "Role:"));

        // Test with user filter
        viewer.set_role_filter(Some("user".to_string()));
        let buffer = render_component(&mut viewer, 100, 30);
        assert!(buffer_contains(&buffer, "Role: user"));

        // Test with assistant filter
        viewer.set_role_filter(Some("assistant".to_string()));
        let buffer = render_component(&mut viewer, 100, 30);
        assert!(buffer_contains(&buffer, "Role: assistant"));

        // Test with system filter
        viewer.set_role_filter(Some("system".to_string()));
        let buffer = render_component(&mut viewer, 100, 30);
        assert!(buffer_contains(&buffer, "Role: system"));
    }

    #[test]
    fn test_tab_key_not_processed_with_ctrl_modifier() {
        let mut viewer = SessionViewer::new();

        // Tab with CTRL modifier should not trigger role filter toggle
        let msg = viewer.handle_key(create_key_event_with_modifiers(
            KeyCode::Tab,
            KeyModifiers::CONTROL,
        ));
        assert!(msg.is_none());
    }

    #[test]
    fn test_role_filter_toggle_in_search_mode() {
        let mut viewer = SessionViewer::new();

        // Enter search mode
        viewer.handle_key(KeyEvent::new(KeyCode::Char('/'), KeyModifiers::empty()));

        // Tab key should toggle role filter even in search mode
        let msg = viewer.handle_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::empty()));
        assert!(matches!(msg, Some(Message::ToggleSessionRoleFilter)));
    }

    #[test]
    fn test_search_mode_role_filter_help_text() {
        let mut viewer = SessionViewer::new();

        // Enter search mode
        viewer.handle_key(KeyEvent::new(KeyCode::Char('/'), KeyModifiers::empty()));

        let buffer = render_component(&mut viewer, 100, 30);
        assert!(buffer_contains(&buffer, "Tab: Role Filter"));
    }

    #[test]
    fn test_status_bar_wrapping_narrow_terminal() {
        let mut viewer = SessionViewer::new();

        // Create a very narrow terminal (40 characters wide) to force wrapping
        // Increase height to 20 to ensure all wrapped text is visible
        let buffer = render_component(&mut viewer, 40, 20);

        // The long status text should be wrapped across multiple lines
        // Check that key parts of the shortcuts are present
        assert!(buffer_contains(&buffer, "Navigate"));
        assert!(buffer_contains(&buffer, "Copy"));
        assert!(buffer_contains(&buffer, "Search"));
        assert!(buffer_contains(&buffer, "Back"));
    }

    #[test]
    fn test_status_bar_wrapping_very_narrow_terminal() {
        let mut viewer = SessionViewer::new();

        // Create an extremely narrow terminal (20 characters wide)
        // Increase height to accommodate wrapped text
        let buffer = render_component(&mut viewer, 20, 25);

        // Even with extreme narrow width, shortcuts should be wrapped and visible
        assert!(buffer_contains(&buffer, "Navigate"));
        assert!(buffer_contains(&buffer, "Filter"));
        assert!(buffer_contains(&buffer, "Copy"));
    }

    #[test]
    fn test_status_bar_no_wrapping_wide_terminal() {
        let mut viewer = SessionViewer::new();

        // Create a wide terminal (200 characters wide) where no wrapping should occur
        let buffer = render_component(&mut viewer, 200, 10);

        // All shortcuts should be on the same line
        let content = buffer
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();

        // Find the status bar content
        let navigate_pos = content.find("Navigate").expect("Navigate not found");
        let back_pos = content.find("Back").expect("Back not found");

        // They should be on the same line (200 chars per line)
        let navigate_line = navigate_pos / 200;
        let back_line = back_pos / 200;
        assert_eq!(
            navigate_line, back_line,
            "Status bar should be on a single line in wide terminal"
        );
    }

    #[test]
    fn test_status_bar_height_with_message() {
        let mut viewer = SessionViewer::new();
        viewer.set_message(Some("✓ Test message".to_string()));

        // Create a narrow terminal to test wrapping with message present
        let buffer = render_component(&mut viewer, 40, 15);

        // Both message and status bar should be visible
        assert!(buffer_contains(&buffer, "✓ Test message"));
        assert!(buffer_contains(&buffer, "Navigate"));
        assert!(buffer_contains(&buffer, "Back"));
    }
}
