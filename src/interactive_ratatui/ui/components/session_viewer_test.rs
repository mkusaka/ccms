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
        let viewer = SessionViewer::new();
        assert!(viewer.messages.is_empty());
        assert!(viewer.filtered_indices.is_empty());
        assert_eq!(viewer.selected_index, 0);
        assert_eq!(viewer.scroll_offset, 0);
        assert!(viewer.query.is_empty());
        assert!(viewer.order.is_none());
        assert!(!viewer.is_searching);
        assert!(viewer.file_path.is_none());
        assert!(viewer.session_id.is_none());
    }

    #[test]
    fn test_set_messages() {
        let mut viewer = SessionViewer::new();
        let messages = vec![
            r#"{"type":"user","message":{"content":"Hello"},"timestamp":"2024-01-01T00:00:00Z"}"#.to_string(),
            r#"{"type":"assistant","message":{"content":"Hi there"},"timestamp":"2024-01-01T00:01:00Z"}"#.to_string(),
        ];

        viewer.set_messages(messages.clone());
        assert_eq!(viewer.messages.len(), 2);
        assert_eq!(viewer.filtered_indices, vec![0, 1]);
        assert_eq!(viewer.selected_index, 0);
        assert_eq!(viewer.scroll_offset, 0);
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
        assert_eq!(viewer.filtered_indices, vec![0, 2]);

        // Test reset when selected index is out of bounds
        viewer.selected_index = 2;
        viewer.set_filtered_indices(vec![0]);
        assert_eq!(viewer.selected_index, 0);
        assert_eq!(viewer.scroll_offset, 0);
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
            "message 1".to_string(),
            "message 2".to_string(),
            "message 3".to_string(),
        ]);

        // Test down navigation
        let msg = viewer.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::empty()));
        assert!(matches!(msg, Some(Message::SessionScrollDown)));

        // Test up navigation
        let msg = viewer.handle_key(KeyEvent::new(KeyCode::Up, KeyModifiers::empty()));
        assert!(matches!(msg, Some(Message::SessionScrollUp)));
    }

    #[test]
    fn test_search_mode() {
        let mut viewer = SessionViewer::new();

        // Enter search mode
        let msg = viewer.handle_key(KeyEvent::new(KeyCode::Char('/'), KeyModifiers::empty()));
        assert!(msg.is_none());
        assert!(viewer.is_searching);

        // Type in search
        let msg = viewer.handle_key(KeyEvent::new(KeyCode::Char('t'), KeyModifiers::empty()));
        assert!(matches!(msg, Some(Message::SessionQueryChanged(q)) if q == "t"));
        assert_eq!(viewer.query, "t");

        // Cancel search
        let msg = viewer.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::empty()));
        assert!(matches!(msg, Some(Message::SessionQueryChanged(q)) if q.is_empty()));
        assert!(!viewer.is_searching);
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

        // Test Backspace key
        let msg = viewer.handle_key(KeyEvent::new(KeyCode::Backspace, KeyModifiers::empty()));
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
        assert!(buffer_contains(&buffer, "[user"));
        assert!(buffer_contains(&buffer, "01/01 12:00"));
        assert!(buffer_contains(&buffer, "Hello world"));
        assert!(buffer_contains(&buffer, "[assistant"));
        assert!(buffer_contains(&buffer, "Hi there!"));
        // Invalid JSON should display raw
        assert!(buffer_contains(&buffer, "Invalid JSON message"));
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
    fn test_search_bar_rendering() {
        let mut viewer = SessionViewer::new();
        viewer.is_searching = true;
        viewer.query = "test query".to_string();

        let buffer = render_component(&mut viewer, 80, 24);
        assert!(buffer_contains(&buffer, "test query"));
        assert!(buffer_contains(&buffer, "Search in session"));
    }

    #[test]
    fn test_empty_filtered_results() {
        let mut viewer = SessionViewer::new();
        viewer.set_messages(vec!["message 1".to_string(), "message 2".to_string()]);
        viewer.set_filtered_indices(vec![]); // No matches

        let _buffer = render_component(&mut viewer, 80, 24);
        // Should show all messages when filtered_indices is empty
        assert_eq!(viewer.filtered_indices, vec![0, 1]);
    }
}
