#[cfg(test)]
mod tests {
    use super::super::session_list::SessionList;
    use crate::interactive_ratatui::ui::app_state::SessionInfo;
    use crate::interactive_ratatui::ui::components::Component;
    use crate::interactive_ratatui::ui::events::Message;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use ratatui::{Terminal, backend::TestBackend, buffer::Buffer};

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
    fn test_session_list_initial_state() {
        let mut session_list = SessionList::new();

        // Test initial render shows empty state
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|f| {
                session_list.render(f, f.area());
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let content = buffer_to_string(buffer);

        // Initial state should show empty sessions and search bar
        assert!(content.contains("Search Sessions"));
        assert!(content.contains("No sessions found"));
    }

    #[test]
    fn test_session_list_render_empty() {
        let mut session_list = SessionList::new();
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|f| {
                session_list.render(f, f.area());
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let content = buffer_to_string(buffer);

        assert!(content.contains("Search Sessions"));
        assert!(content.contains("No sessions found"));
    }

    #[test]
    fn test_session_list_render_loading() {
        let mut session_list = SessionList::new();
        session_list.set_is_loading(true);

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|f| {
                session_list.render(f, f.area());
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let content = buffer_to_string(buffer);

        assert!(content.contains("Loading..."));
    }

    #[test]
    fn test_session_list_render_with_sessions() {
        let mut session_list = SessionList::new();
        let sessions = vec![
            create_test_session_info("session1", "Hello world"),
            create_test_session_info("session2", "Goodbye world"),
        ];
        session_list.set_sessions(sessions);

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|f| {
                session_list.render(f, f.area());
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let content = buffer_to_string(buffer);

        assert!(content.contains("session1"));
        assert!(content.contains("session2"));
        assert!(content.contains("Hello world"));
        assert!(content.contains("Goodbye world"));
    }

    #[test]
    fn test_session_list_render_with_search() {
        let mut session_list = SessionList::new();
        session_list.set_query("test query".to_string());
        session_list.set_is_searching(true);

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|f| {
                session_list.render(f, f.area());
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let content = buffer_to_string(buffer);

        assert!(content.contains("Search Sessions"));
        assert!(content.contains("Search [searching...]"));
        assert!(content.contains("test query"));
    }

    #[test]
    fn test_session_list_key_input() {
        let mut session_list = SessionList::new();

        // Test character input
        let msg = session_list.handle_key(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::empty()));
        assert!(matches!(msg, Some(Message::SessionListQueryChanged(ref q)) if q == "a"));

        // Set the query based on the message to simulate state update
        session_list.set_query("a".to_string());

        // Test uppercase character
        let msg = session_list.handle_key(KeyEvent::new(KeyCode::Char('B'), KeyModifiers::SHIFT));
        assert!(matches!(msg, Some(Message::SessionListQueryChanged(ref q)) if q == "aB"));

        // Update query
        session_list.set_query("aB".to_string());

        // Test backspace
        let msg = session_list.handle_key(KeyEvent::new(KeyCode::Backspace, KeyModifiers::empty()));
        assert!(matches!(msg, Some(Message::SessionListQueryChanged(ref q)) if q == "a"));
    }

    #[test]
    fn test_session_list_navigation_keys() {
        let mut session_list = SessionList::new();

        // Test up/down navigation
        let msg = session_list.handle_key(KeyEvent::new(KeyCode::Up, KeyModifiers::empty()));
        assert!(matches!(msg, Some(Message::SessionListScrollUp)));

        let msg = session_list.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::empty()));
        assert!(matches!(msg, Some(Message::SessionListScrollDown)));

        // Test page navigation
        let msg = session_list.handle_key(KeyEvent::new(KeyCode::PageUp, KeyModifiers::empty()));
        assert!(matches!(msg, Some(Message::SessionListPageUp)));

        let msg = session_list.handle_key(KeyEvent::new(KeyCode::PageDown, KeyModifiers::empty()));
        assert!(matches!(msg, Some(Message::SessionListPageDown)));

        // Test half-page scrolling
        let msg = session_list.handle_key(KeyEvent::new(KeyCode::Char('u'), KeyModifiers::CONTROL));
        assert!(matches!(msg, Some(Message::SessionListHalfPageUp)));

        let msg = session_list.handle_key(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::CONTROL));
        assert!(matches!(msg, Some(Message::SessionListHalfPageDown)));
    }

    #[test]
    fn test_session_list_action_keys() {
        let mut session_list = SessionList::new();
        let sessions = vec![create_test_session_info("session1", "Test")];
        session_list.set_sessions(sessions);

        // Test Enter key
        let msg = session_list.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::empty()));
        assert!(matches!(msg, Some(Message::EnterSessionViewerFromList(_))));

        // Test Ctrl+S
        let msg = session_list.handle_key(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::CONTROL));
        assert!(matches!(msg, Some(Message::EnterSessionViewerFromList(_))));

        // Test Ctrl+T (toggle preview)
        let msg = session_list.handle_key(KeyEvent::new(KeyCode::Char('t'), KeyModifiers::CONTROL));
        assert!(matches!(msg, Some(Message::ToggleSessionListPreview)));
    }

    #[test]
    fn test_session_list_ignore_control_chars() {
        let mut session_list = SessionList::new();

        // Control characters should not be added to query (except specific ones)
        let msg = session_list.handle_key(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::CONTROL));
        assert!(msg.is_none());

        // Alt characters should not be added
        let msg = session_list.handle_key(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::ALT));
        assert!(msg.is_none());
    }

    #[test]
    fn test_cursor_position_preserved_on_set_query() {
        // This test ensures that cursor position is preserved when set_query is called
        // with the same value (simulating render cycles)
        let mut session_list = SessionList::new();

        // Type "hello"
        session_list.handle_key(KeyEvent::new(KeyCode::Char('h'), KeyModifiers::empty()));
        session_list.handle_key(KeyEvent::new(KeyCode::Char('e'), KeyModifiers::empty()));
        session_list.handle_key(KeyEvent::new(KeyCode::Char('l'), KeyModifiers::empty()));
        session_list.handle_key(KeyEvent::new(KeyCode::Char('l'), KeyModifiers::empty()));
        session_list.handle_key(KeyEvent::new(KeyCode::Char('o'), KeyModifiers::empty()));

        // Move cursor to beginning
        session_list.handle_key(KeyEvent::new(KeyCode::Home, KeyModifiers::empty()));

        // Simulate render cycle - set_query with same value
        session_list.set_query("hello".to_string());

        // Type 'X' - should appear at beginning if cursor position was preserved
        let msg = session_list.handle_key(KeyEvent::new(KeyCode::Char('X'), KeyModifiers::empty()));
        assert!(matches!(msg, Some(Message::SessionListQueryChanged(q)) if q == "Xhello"));
    }

    #[test]
    fn test_cursor_position_reset_on_different_query() {
        // This test ensures that cursor position is reset to end when set_query
        // is called with a different value
        let mut session_list = SessionList::new();

        // Type "test"
        session_list.handle_key(KeyEvent::new(KeyCode::Char('t'), KeyModifiers::empty()));
        session_list.handle_key(KeyEvent::new(KeyCode::Char('e'), KeyModifiers::empty()));
        session_list.handle_key(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::empty()));
        session_list.handle_key(KeyEvent::new(KeyCode::Char('t'), KeyModifiers::empty()));

        // Move cursor to beginning
        session_list.handle_key(KeyEvent::new(KeyCode::Home, KeyModifiers::empty()));

        // Set a different query - cursor should move to end
        session_list.set_query("different".to_string());

        // Type 'X' - should appear at end
        let msg = session_list.handle_key(KeyEvent::new(KeyCode::Char('X'), KeyModifiers::empty()));
        assert!(matches!(msg, Some(Message::SessionListQueryChanged(q)) if q == "differentX"));
    }

    #[test]
    fn test_arrow_keys_cursor_movement() {
        // This test specifically tests that arrow keys move the cursor correctly
        // and that the cursor position is preserved between keystrokes
        let mut session_list = SessionList::new();

        // Type "aaa"
        session_list.handle_key(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::empty()));
        session_list.handle_key(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::empty()));
        session_list.handle_key(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::empty()));

        // Press left arrow three times
        session_list.handle_key(KeyEvent::new(KeyCode::Left, KeyModifiers::empty()));
        session_list.handle_key(KeyEvent::new(KeyCode::Left, KeyModifiers::empty()));
        session_list.handle_key(KeyEvent::new(KeyCode::Left, KeyModifiers::empty()));

        // Now cursor should be at beginning, type 'X'
        let msg = session_list.handle_key(KeyEvent::new(KeyCode::Char('X'), KeyModifiers::empty()));
        assert!(matches!(msg, Some(Message::SessionListQueryChanged(q)) if q == "Xaaa"));

        // Press right arrow twice
        session_list.handle_key(KeyEvent::new(KeyCode::Right, KeyModifiers::empty()));
        session_list.handle_key(KeyEvent::new(KeyCode::Right, KeyModifiers::empty()));

        // Now cursor should be after "Xaa" (position 3), type 'Y'
        let msg = session_list.handle_key(KeyEvent::new(KeyCode::Char('Y'), KeyModifiers::empty()));
        assert!(matches!(msg, Some(Message::SessionListQueryChanged(q)) if q == "XaaYa"));
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
