#[cfg(test)]
mod tests {
    use crate::interactive_ratatui::ui::app_state::SessionInfo;
    use crate::interactive_ratatui::ui::components::session_list::SessionList;
    use crate::interactive_ratatui::ui::components::Component;
    use crate::interactive_ratatui::ui::events::Message;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;
    use ratatui::buffer::Buffer;

    fn create_test_sessions() -> Vec<SessionInfo> {
        vec![
            SessionInfo {
                file_path: "/path/to/session1.jsonl".to_string(),
                session_id: "session-1".to_string(),
                timestamp: "2024-01-01T12:00:00Z".to_string(),
                message_count: 10,
                first_message: "Hello from session 1".to_string(),
            },
            SessionInfo {
                file_path: "/path/to/session2.jsonl".to_string(),
                session_id: "session-2".to_string(),
                timestamp: "2024-01-01T13:00:00Z".to_string(),
                message_count: 20,
                first_message: "Hello from session 2".to_string(),
            },
            SessionInfo {
                file_path: "/path/to/session3.jsonl".to_string(),
                session_id: "session-3".to_string(),
                timestamp: "2024-01-01T14:00:00Z".to_string(),
                message_count: 30,
                first_message: "Hello from session 3".to_string(),
            },
        ]
    }

    #[test]
    fn test_navigation_up_down() {
        let mut session_list = SessionList::new();
        session_list.set_sessions(create_test_sessions());

        // Move down
        let msg = session_list.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::empty()));
        assert_eq!(msg, Some(Message::SessionListScrollDown));

        // Move up
        let msg = session_list.handle_key(KeyEvent::new(KeyCode::Up, KeyModifiers::empty()));
        assert_eq!(msg, Some(Message::SessionListScrollUp));
    }

    #[test]
    fn test_half_page_navigation() {
        let mut session_list = SessionList::new();
        session_list.set_sessions(create_test_sessions());

        // Ctrl+D - half page down
        let msg = session_list.handle_key(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::CONTROL));
        assert_eq!(msg, Some(Message::SessionListHalfPageDown));

        // Ctrl+U - half page up
        let msg = session_list.handle_key(KeyEvent::new(KeyCode::Char('u'), KeyModifiers::CONTROL));
        assert_eq!(msg, Some(Message::SessionListHalfPageUp));
    }

    #[test]
    fn test_page_navigation() {
        let mut session_list = SessionList::new();
        session_list.set_sessions(create_test_sessions());

        // PageDown
        let msg = session_list.handle_key(KeyEvent::new(KeyCode::PageDown, KeyModifiers::empty()));
        assert_eq!(msg, Some(Message::SessionListPageDown));

        // PageUp
        let msg = session_list.handle_key(KeyEvent::new(KeyCode::PageUp, KeyModifiers::empty()));
        assert_eq!(msg, Some(Message::SessionListPageUp));
    }

    #[test]
    fn test_toggle_preview() {
        let mut session_list = SessionList::new();
        session_list.set_sessions(create_test_sessions());

        // Toggle preview with Ctrl+T
        let msg = session_list.handle_key(KeyEvent::new(KeyCode::Char('t'), KeyModifiers::CONTROL));
        assert_eq!(msg, Some(Message::ToggleSessionListPreview));
    }

    #[test]
    fn test_enter_session_viewer() {
        let mut session_list = SessionList::new();
        session_list.set_sessions(create_test_sessions());

        // Press Enter
        let msg = session_list.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::empty()));
        assert_eq!(
            msg,
            Some(Message::EnterSessionViewerFromList(
                "/path/to/session1.jsonl".to_string()
            ))
        );

        // Press Ctrl+S (should do the same)
        let msg = session_list.handle_key(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::CONTROL));
        assert_eq!(
            msg,
            Some(Message::EnterSessionViewerFromList(
                "/path/to/session1.jsonl".to_string()
            ))
        );
    }

    #[test]
    fn test_empty_session_list() {
        let mut session_list = SessionList::new();

        // No sessions, so Enter should return None
        let msg = session_list.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::empty()));
        assert_eq!(msg, None);

        // Ctrl+S should also return None
        let msg = session_list.handle_key(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::CONTROL));
        assert_eq!(msg, None);
    }

    #[test]
    fn test_status_bar_text_with_preview() {
        let backend = TestBackend::new(120, 30);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|f| {
                let mut session_list = SessionList::new();
                session_list.set_sessions(create_test_sessions());
                session_list.set_preview_enabled(true);
                session_list.render(f, f.area());
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let status_text = buffer_contains(buffer, "Hide preview");
        assert!(
            status_text,
            "Status bar should show 'Hide preview' when preview is enabled"
        );
    }

    #[test]
    fn test_status_bar_text_without_preview() {
        let backend = TestBackend::new(120, 30);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|f| {
                let mut session_list = SessionList::new();
                session_list.set_sessions(create_test_sessions());
                session_list.set_preview_enabled(false);
                session_list.render(f, f.area());
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let status_text = buffer_contains(buffer, "Show preview");
        assert!(
            status_text,
            "Status bar should show 'Show preview' when preview is disabled"
        );
    }

    // Helper function to check if buffer contains text
    fn buffer_contains(buffer: &Buffer, text: &str) -> bool {
        let content = buffer
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        content.contains(text)
    }
}