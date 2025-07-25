#[cfg(test)]
mod tests {
    use super::super::session_viewer::SessionViewer;
    use crate::interactive_ratatui::domain::models::SessionOrder;
    use crate::interactive_ratatui::ui::components::Component;
    use crate::interactive_ratatui::ui::events::Message;
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
    fn test_metadata_display() {
        let mut viewer = SessionViewer::new();
        viewer.set_file_path(Some("/path/to/session.jsonl".to_string()));
        viewer.set_session_id(Some("session-123".to_string()));

        let buffer = render_component(&mut viewer, 80, 24);

        // Check that metadata is displayed
        assert!(buffer_contains(&buffer, "Session: session-123"));
        assert!(buffer_contains(&buffer, "File: /path/to/session.jsonl"));
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

        // Test down navigation - should return SessionScrollDown when moving down
        let msg = viewer.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::empty()));
        assert!(matches!(msg, Some(Message::SessionScrollDown)));

        // Test up navigation - should return SessionScrollUp when moving up
        let msg = viewer.handle_key(KeyEvent::new(KeyCode::Up, KeyModifiers::empty()));
        assert!(matches!(msg, Some(Message::SessionScrollUp)));
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

        // Test copy single message
        let msg = viewer.handle_key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::empty()));
        assert!(matches!(msg, Some(Message::CopyToClipboard(_))));

        // Test copy all messages
        let msg = viewer.handle_key(KeyEvent::new(KeyCode::Char('C'), KeyModifiers::empty()));
        assert!(matches!(msg, Some(Message::CopyToClipboard(_))));

        // Test copy session ID
        let msg = viewer.handle_key(KeyEvent::new(KeyCode::Char('i'), KeyModifiers::empty()));
        assert!(matches!(msg, Some(Message::CopyToClipboard(id)) if id == "session-123"));

        // Test copy session ID with uppercase
        let msg = viewer.handle_key(KeyEvent::new(KeyCode::Char('I'), KeyModifiers::empty()));
        assert!(matches!(msg, Some(Message::CopyToClipboard(id)) if id == "session-123"));
    }

    #[test]
    fn test_copy_session_id_without_id() {
        let mut viewer = SessionViewer::new();
        // No session ID set

        let msg = viewer.handle_key(KeyEvent::new(KeyCode::Char('i'), KeyModifiers::empty()));
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

    #[test]
    fn test_vim_navigation() {
        let mut viewer = SessionViewer::new();
        viewer.set_messages(vec![
            r#"{"type":"user","message":{"content":"message 1"},"timestamp":"2024-01-01T00:00:00Z"}"#.to_string(),
            r#"{"type":"user","message":{"content":"message 2"},"timestamp":"2024-01-01T00:00:01Z"}"#.to_string(),
            r#"{"type":"user","message":{"content":"message 3"},"timestamp":"2024-01-01T00:00:02Z"}"#.to_string(),
        ]);

        // Test down navigation with 'j' - should return SessionScrollDown when moving down
        let msg = viewer.handle_key(KeyEvent::new(KeyCode::Char('j'), KeyModifiers::empty()));
        assert!(matches!(msg, Some(Message::SessionScrollDown)));

        // Test up navigation with 'k' - should return SessionScrollUp when moving up
        let msg = viewer.handle_key(KeyEvent::new(KeyCode::Char('k'), KeyModifiers::empty()));
        assert!(matches!(msg, Some(Message::SessionScrollUp)));
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
}
