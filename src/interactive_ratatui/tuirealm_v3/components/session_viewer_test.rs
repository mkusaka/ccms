#[cfg(test)]
mod session_viewer_tests {
    use super::super::*;
    use crate::interactive_ratatui::tuirealm_v3::messages::AppMessage;
    use tuirealm::command::{Cmd, CmdResult};
    use tuirealm::event::{Event, Key, KeyEvent, KeyModifiers};
    use tuirealm::props::{AttrValue, Attribute};
    use tuirealm::{Component, MockComponent};

    fn create_session_viewer() -> SessionViewer {
        SessionViewer::new()
    }

    fn setup_session_viewer_with_data(viewer: &mut SessionViewer) {
        let session_texts = [
            r#"{"role": "User", "content": "First message", "timestamp": "2024-01-01T10:00:00Z"}"#,
            r#"{"role": "Assistant", "content": "Second message", "timestamp": "2024-01-01T10:01:00Z"}"#,
            r#"{"role": "User", "content": "Third message", "timestamp": "2024-01-01T10:02:00Z"}"#,
        ].join("\n\n");
        
        viewer.attr(
            Attribute::Custom("session_texts"),
            AttrValue::String(session_texts)
        );
        viewer.attr(
            Attribute::Custom("message_count"),
            AttrValue::String("3".to_string())
        );
        viewer.attr(
            Attribute::Custom("session_id"),
            AttrValue::String("test-session-123".to_string())
        );
        viewer.attr(
            Attribute::Value,
            AttrValue::String("1".to_string())
        );
    }

    #[test]
    fn test_session_viewer_new() {
        let viewer = create_session_viewer();
        
        // Check initial state
        assert_eq!(viewer.props.get(Attribute::Value), Some(AttrValue::Length(0)));
        assert!(viewer.props.get(Attribute::Borders).is_some());
    }

    #[test]
    fn test_session_viewer_attributes() {
        let mut viewer = create_session_viewer();
        setup_session_viewer_with_data(&mut viewer);
        
        // Verify attributes
        assert_eq!(
            viewer.query(Attribute::Custom("session_id")),
            Some(AttrValue::String("test-session-123".to_string()))
        );
        assert_eq!(
            viewer.query(Attribute::Custom("message_count")),
            Some(AttrValue::String("3".to_string()))
        );
    }

    #[test]
    fn test_session_viewer_exit_keys() {
        let mut viewer = create_session_viewer();
        
        // Esc
        assert_eq!(
            viewer.on(Event::Keyboard(KeyEvent {
                code: Key::Esc,
                modifiers: KeyModifiers::empty(),
            })),
            Some(AppMessage::ExitSessionViewer)
        );
        
        // q
        assert_eq!(
            viewer.on(Event::Keyboard(KeyEvent {
                code: Key::Char('q'),
                modifiers: KeyModifiers::empty(),
            })),
            Some(AppMessage::ExitSessionViewer)
        );
    }

    #[test]
    fn test_session_viewer_navigation() {
        let mut viewer = create_session_viewer();
        setup_session_viewer_with_data(&mut viewer);
        
        // Up/Down arrows
        assert_eq!(
            viewer.on(Event::Keyboard(KeyEvent {
                code: Key::Up,
                modifiers: KeyModifiers::empty(),
            })),
            Some(AppMessage::SessionScrollUp)
        );
        
        assert_eq!(
            viewer.on(Event::Keyboard(KeyEvent {
                code: Key::Down,
                modifiers: KeyModifiers::empty(),
            })),
            Some(AppMessage::SessionScrollDown)
        );
        
        // Vim keys
        assert_eq!(
            viewer.on(Event::Keyboard(KeyEvent {
                code: Key::Char('k'),
                modifiers: KeyModifiers::empty(),
            })),
            Some(AppMessage::SessionScrollUp)
        );
        
        assert_eq!(
            viewer.on(Event::Keyboard(KeyEvent {
                code: Key::Char('j'),
                modifiers: KeyModifiers::empty(),
            })),
            Some(AppMessage::SessionScrollDown)
        );
    }

    #[test]
    fn test_session_viewer_page_navigation() {
        let mut viewer = create_session_viewer();
        
        assert_eq!(
            viewer.on(Event::Keyboard(KeyEvent {
                code: Key::PageUp,
                modifiers: KeyModifiers::empty(),
            })),
            Some(AppMessage::SessionPageUp)
        );
        
        assert_eq!(
            viewer.on(Event::Keyboard(KeyEvent {
                code: Key::PageDown,
                modifiers: KeyModifiers::empty(),
            })),
            Some(AppMessage::SessionPageDown)
        );
        
        // Ctrl+B/F in normal mode are not handled
        assert_eq!(
            viewer.on(Event::Keyboard(KeyEvent {
                code: Key::Char('b'),
                modifiers: KeyModifiers::CONTROL,
            })),
            None
        );
        
        assert_eq!(
            viewer.on(Event::Keyboard(KeyEvent {
                code: Key::Char('f'),
                modifiers: KeyModifiers::CONTROL,
            })),
            None
        );
    }

    #[test]
    fn test_session_viewer_search() {
        let mut viewer = create_session_viewer();
        
        // Start search with '/'
        assert_eq!(
            viewer.on(Event::Keyboard(KeyEvent {
                code: Key::Char('/'),
                modifiers: KeyModifiers::empty(),
            })),
            Some(AppMessage::SessionSearchStart)
        );
    }

    #[test]
    fn test_session_viewer_search_mode() {
        let mut viewer = create_session_viewer();
        viewer.attr(Attribute::Custom("is_searching"), AttrValue::Flag(true));
        viewer.attr(Attribute::Custom("search_query"), AttrValue::String("test".to_string()));
        
        // Type character in search mode
        assert_eq!(
            viewer.on(Event::Keyboard(KeyEvent {
                code: Key::Char('s'),
                modifiers: KeyModifiers::empty(),
            })),
            Some(AppMessage::SessionQueryChanged("tests".to_string()))
        );
        
        // Backspace in search mode
        viewer.attr(Attribute::Custom("search_query"), AttrValue::String("test".to_string()));
        assert_eq!(
            viewer.on(Event::Keyboard(KeyEvent {
                code: Key::Backspace,
                modifiers: KeyModifiers::empty(),
            })),
            Some(AppMessage::SessionQueryChanged("tes".to_string()))
        );
        
        // Esc to end search
        assert_eq!(
            viewer.on(Event::Keyboard(KeyEvent {
                code: Key::Esc,
                modifiers: KeyModifiers::empty(),
            })),
            Some(AppMessage::SessionSearchEnd)
        );
        
        // Enter to end search
        assert_eq!(
            viewer.on(Event::Keyboard(KeyEvent {
                code: Key::Enter,
                modifiers: KeyModifiers::empty(),
            })),
            Some(AppMessage::SessionSearchEnd)
        );
    }

    #[test]
    fn test_session_viewer_toggle_order() {
        let mut viewer = create_session_viewer();
        
        assert_eq!(
            viewer.on(Event::Keyboard(KeyEvent {
                code: Key::Char('o'),
                modifiers: KeyModifiers::empty(),
            })),
            Some(AppMessage::SessionToggleOrder)
        );
    }

    #[test]
    fn test_session_viewer_toggle_truncation() {
        let mut viewer = create_session_viewer();
        
        // 't' without modifiers toggles truncation
        assert_eq!(
            viewer.on(Event::Keyboard(KeyEvent {
                code: Key::Char('t'),
                modifiers: KeyModifiers::empty(),
            })),
            Some(AppMessage::ToggleTruncation)
        );
    }

    #[test]
    fn test_session_viewer_copy_operations() {
        let mut viewer = create_session_viewer();
        setup_session_viewer_with_data(&mut viewer);
        
        // Copy message (c)
        assert_eq!(
            viewer.on(Event::Keyboard(KeyEvent {
                code: Key::Char('c'),
                modifiers: KeyModifiers::empty(),
            })),
            Some(AppMessage::CopyMessage)
        );
        
        // Copy session ID (Shift+Y)
        assert_eq!(
            viewer.on(Event::Keyboard(KeyEvent {
                code: Key::Char('Y'),
                modifiers: KeyModifiers::SHIFT,
            })),
            Some(AppMessage::CopySessionId)
        );
        
        // Copy raw JSON (C)
        assert_eq!(
            viewer.on(Event::Keyboard(KeyEvent {
                code: Key::Char('C'),
                modifiers: KeyModifiers::SHIFT,
            })),
            Some(AppMessage::CopyRawJson)
        );
    }

    #[test]
    fn test_session_viewer_with_messages() {
        let mut viewer = create_session_viewer();
        setup_session_viewer_with_data(&mut viewer);
        
        // Verify that messages are set correctly via attributes
        assert_eq!(
            viewer.query(Attribute::Custom("message_count")),
            Some(AttrValue::String("3".to_string()))
        );
        
        // Session texts should be set
        assert!(viewer.query(Attribute::Custom("session_texts")).is_some());
    }

    #[test]
    fn test_session_viewer_empty_messages() {
        let mut viewer = create_session_viewer();
        viewer.attr(
            Attribute::Custom("session_texts"),
            AttrValue::String("".to_string())
        );
        
        // Should have empty session texts
        assert_eq!(
            viewer.query(Attribute::Custom("session_texts")),
            Some(AttrValue::String("".to_string()))
        );
    }

    #[test]
    fn test_session_viewer_order_display() {
        let mut viewer = create_session_viewer();
        
        // Test different order settings
        viewer.attr(
            Attribute::Custom("order"),
            AttrValue::String("asc".to_string())
        );
        assert_eq!(
            viewer.query(Attribute::Custom("order")),
            Some(AttrValue::String("asc".to_string()))
        );
        
        viewer.attr(
            Attribute::Custom("order"),
            AttrValue::String("desc".to_string())
        );
        assert_eq!(
            viewer.query(Attribute::Custom("order")),
            Some(AttrValue::String("desc".to_string()))
        );
        
        viewer.attr(
            Attribute::Custom("order"),
            AttrValue::String("original".to_string())
        );
        assert_eq!(
            viewer.query(Attribute::Custom("order")),
            Some(AttrValue::String("original".to_string()))
        );
    }

    #[test]
    fn test_session_viewer_scroll_offset() {
        let mut viewer = create_session_viewer();
        
        viewer.attr(
            Attribute::Custom("scroll_offset"),
            AttrValue::String("5".to_string())
        );
        
        assert_eq!(
            viewer.query(Attribute::Custom("scroll_offset")),
            Some(AttrValue::String("5".to_string()))
        );
    }

    #[test]
    fn test_session_viewer_perform() {
        let mut viewer = create_session_viewer();
        
        // perform should always return None
        assert_eq!(viewer.perform(Cmd::Cancel), CmdResult::None);
        assert_eq!(viewer.perform(Cmd::Submit), CmdResult::None);
    }

    #[test]
    fn test_session_viewer_state() {
        let viewer = create_session_viewer();
        
        // State contains selected index and search cursor
        // Just verify it doesn't panic
        let _ = viewer.state();
    }

    #[test]
    fn test_session_viewer_mixed_message_formats() {
        let mut viewer = create_session_viewer();
        
        // Test with different message formats
        let session_texts = [
            r#"{"role": "User", "content": "Simple text"}"#,
            r#"{"role": "Assistant", "content": [{"type": "text", "text": "Array format"}]}"#,
            r#"{"role": "System", "message": {"content": "Nested format"}}"#,
            "Plain text message without JSON",
        ].join("\n\n");
        
        viewer.attr(
            Attribute::Custom("session_texts"),
            AttrValue::String(session_texts.clone())
        );
        
        // Verify that mixed format texts are stored
        assert_eq!(
            viewer.query(Attribute::Custom("session_texts")),
            Some(AttrValue::String(session_texts))
        );
        
        // Message count should be set
        viewer.attr(
            Attribute::Custom("message_count"),
            AttrValue::String("4".to_string())
        );
        assert_eq!(
            viewer.query(Attribute::Custom("message_count")),
            Some(AttrValue::String("4".to_string()))
        );
    }

    #[test]
    fn test_session_viewer_backspace_empty_search() {
        let mut viewer = create_session_viewer();
        viewer.attr(Attribute::Custom("is_searching"), AttrValue::Flag(true));
        viewer.attr(Attribute::Custom("search_query"), AttrValue::String("".to_string()));
        
        // Backspace on empty search does nothing (cursor position is 0)
        assert_eq!(
            viewer.on(Event::Keyboard(KeyEvent {
                code: Key::Backspace,
                modifiers: KeyModifiers::empty(),
            })),
            None
        );
    }
}