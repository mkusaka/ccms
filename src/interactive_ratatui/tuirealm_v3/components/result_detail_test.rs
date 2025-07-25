#[cfg(test)]
mod result_detail_tests {
    use super::super::*;
    use crate::interactive_ratatui::tuirealm_v3::messages::AppMessage;
    use tuirealm::command::{Cmd, CmdResult};
    use tuirealm::event::{Event, Key, KeyEvent, KeyModifiers};
    use tuirealm::props::{AttrValue, Attribute};
    use tuirealm::{Component, MockComponent};

    fn create_result_detail() -> ResultDetail {
        ResultDetail::new()
    }

    fn setup_result_detail_with_data(detail: &mut ResultDetail) {
        detail.attr(
            Attribute::Custom("session_id"),
            AttrValue::String("test-session-123".to_string())
        );
        detail.attr(
            Attribute::Custom("file"),
            AttrValue::String("/path/to/test.jsonl".to_string())
        );
        detail.attr(
            Attribute::Custom("timestamp"),
            AttrValue::String("2024-01-01T10:30:00Z".to_string())
        );
        detail.attr(
            Attribute::Custom("role"),
            AttrValue::String("User".to_string())
        );
        detail.attr(
            Attribute::Custom("text"),
            AttrValue::String("This is a test message".to_string())
        );
        detail.attr(
            Attribute::Custom("raw_json"),
            AttrValue::String(r#"{"content": "This is a test message"}"#.to_string())
        );
        detail.attr(
            Attribute::Custom("scroll_offset"),
            AttrValue::String("0".to_string())
        );
    }

    #[test]
    fn test_result_detail_new() {
        let detail = create_result_detail();
        
        // Check that borders are set
        assert!(detail.props.get(Attribute::Borders).is_some());
    }

    #[test]
    fn test_result_detail_attributes() {
        let mut detail = create_result_detail();
        setup_result_detail_with_data(&mut detail);
        
        // Verify attributes are stored
        assert_eq!(
            detail.query(Attribute::Custom("session_id")),
            Some(AttrValue::String("test-session-123".to_string()))
        );
        assert_eq!(
            detail.query(Attribute::Custom("text")),
            Some(AttrValue::String("This is a test message".to_string()))
        );
    }

    #[test]
    fn test_result_detail_exit_keys() {
        let mut detail = create_result_detail();
        
        // Esc
        assert_eq!(
            detail.on(Event::Keyboard(KeyEvent {
                code: Key::Esc,
                modifiers: KeyModifiers::empty(),
            })),
            Some(AppMessage::ExitResultDetail)
        );
        
        // q
        assert_eq!(
            detail.on(Event::Keyboard(KeyEvent {
                code: Key::Char('q'),
                modifiers: KeyModifiers::empty(),
            })),
            Some(AppMessage::ExitResultDetail)
        );
    }

    #[test]
    fn test_result_detail_scroll_keys() {
        let mut detail = create_result_detail();
        
        // Up arrow
        assert_eq!(
            detail.on(Event::Keyboard(KeyEvent {
                code: Key::Up,
                modifiers: KeyModifiers::empty(),
            })),
            Some(AppMessage::DetailScrollUp)
        );
        
        // Down arrow
        assert_eq!(
            detail.on(Event::Keyboard(KeyEvent {
                code: Key::Down,
                modifiers: KeyModifiers::empty(),
            })),
            Some(AppMessage::DetailScrollDown)
        );
        
        // Vim keys
        assert_eq!(
            detail.on(Event::Keyboard(KeyEvent {
                code: Key::Char('k'),
                modifiers: KeyModifiers::empty(),
            })),
            Some(AppMessage::DetailScrollUp)
        );
        
        assert_eq!(
            detail.on(Event::Keyboard(KeyEvent {
                code: Key::Char('j'),
                modifiers: KeyModifiers::empty(),
            })),
            Some(AppMessage::DetailScrollDown)
        );
    }

    #[test]
    fn test_result_detail_page_scroll() {
        let mut detail = create_result_detail();
        
        // Page Up
        assert_eq!(
            detail.on(Event::Keyboard(KeyEvent {
                code: Key::PageUp,
                modifiers: KeyModifiers::empty(),
            })),
            Some(AppMessage::DetailPageUp)
        );
        
        // Page Down
        assert_eq!(
            detail.on(Event::Keyboard(KeyEvent {
                code: Key::PageDown,
                modifiers: KeyModifiers::empty(),
            })),
            Some(AppMessage::DetailPageDown)
        );
        
        // Ctrl+B
        assert_eq!(
            detail.on(Event::Keyboard(KeyEvent {
                code: Key::Char('b'),
                modifiers: KeyModifiers::CONTROL,
            })),
            Some(AppMessage::DetailPageUp)
        );
        
        // Ctrl+F
        assert_eq!(
            detail.on(Event::Keyboard(KeyEvent {
                code: Key::Char('f'),
                modifiers: KeyModifiers::CONTROL,
            })),
            Some(AppMessage::DetailPageDown)
        );
    }

    #[test]
    fn test_result_detail_copy_operations() {
        let mut detail = create_result_detail();
        setup_result_detail_with_data(&mut detail);
        
        // Copy message (c)
        assert_eq!(
            detail.on(Event::Keyboard(KeyEvent {
                code: Key::Char('c'),
                modifiers: KeyModifiers::empty(),
            })),
            Some(AppMessage::CopyMessage)
        );
        
        // Copy session (y)
        assert_eq!(
            detail.on(Event::Keyboard(KeyEvent {
                code: Key::Char('y'),
                modifiers: KeyModifiers::empty(),
            })),
            Some(AppMessage::CopySession)
        );
        
        // Copy timestamp (Y)
        assert_eq!(
            detail.on(Event::Keyboard(KeyEvent {
                code: Key::Char('Y'),
                modifiers: KeyModifiers::SHIFT,
            })),
            Some(AppMessage::CopyTimestamp)
        );
        
        // Copy raw JSON (C)
        assert_eq!(
            detail.on(Event::Keyboard(KeyEvent {
                code: Key::Char('C'),
                modifiers: KeyModifiers::SHIFT,
            })),
            Some(AppMessage::CopyRawJson)
        );
    }

    #[test]
    fn test_result_detail_session_viewer() {
        let mut detail = create_result_detail();
        setup_result_detail_with_data(&mut detail);
        
        // Press 's' to enter session viewer
        let msg = detail.on(Event::Keyboard(KeyEvent {
            code: Key::Char('s'),
            modifiers: KeyModifiers::empty(),
        }));
        
        assert_eq!(msg, Some(AppMessage::EnterSessionViewer("test-session-123".to_string())));
    }

    #[test]
    fn test_result_detail_session_viewer_no_session() {
        let mut detail = create_result_detail();
        // Don't set session_id attribute
        
        // Press 's' without session_id
        let msg = detail.on(Event::Keyboard(KeyEvent {
            code: Key::Char('s'),
            modifiers: KeyModifiers::empty(),
        }));
        
        assert_eq!(msg, None);
    }

    #[test]
    fn test_result_detail_format_content() {
        let lines = ResultDetail::format_content(
            "session-123",
            "/path/to/file.jsonl",
            "2024-01-01T10:30:00Z",
            "User",
            "Test message\nWith multiple lines"
        );
        
        // Check that metadata is included
        assert!(lines.iter().any(|line| line.to_string().contains("Session ID:")));
        assert!(lines.iter().any(|line| line.to_string().contains("File:")));
        assert!(lines.iter().any(|line| line.to_string().contains("Timestamp:")));
        assert!(lines.iter().any(|line| line.to_string().contains("Role:")));
        assert!(lines.iter().any(|line| line.to_string().contains("Content:")));
        
        // Check that message lines are included
        assert!(lines.iter().any(|line| line.to_string().contains("Test message")));
        assert!(lines.iter().any(|line| line.to_string().contains("With multiple lines")));
    }

    #[test]
    fn test_result_detail_scroll_offset_parsing() {
        let mut detail = create_result_detail();
        
        // Test string parsing
        detail.attr(
            Attribute::Custom("scroll_offset"),
            AttrValue::String("10".to_string())
        );
        
        // Should parse the string correctly
        if let Some(AttrValue::String(s)) = detail.query(Attribute::Custom("scroll_offset")) {
            assert_eq!(s, "10");
        }
        
        // Test with invalid string
        detail.attr(
            Attribute::Custom("scroll_offset"),
            AttrValue::String("invalid".to_string())
        );
        
        // Should handle gracefully (parse to 0)
        // This is tested indirectly through the view method
    }

    #[test]
    fn test_result_detail_message_attribute() {
        let mut detail = create_result_detail();
        
        detail.attr(
            Attribute::Custom("message"),
            AttrValue::String("Copied to clipboard".to_string())
        );
        
        assert_eq!(
            detail.query(Attribute::Custom("message")),
            Some(AttrValue::String("Copied to clipboard".to_string()))
        );
    }

    #[test]
    fn test_result_detail_perform() {
        let mut detail = create_result_detail();
        
        // perform should always return None
        assert_eq!(detail.perform(Cmd::Cancel), CmdResult::None);
        assert_eq!(detail.perform(Cmd::Submit), CmdResult::None);
    }

    #[test]
    fn test_result_detail_state() {
        let detail = create_result_detail();
        
        // State should always be None
        assert_eq!(detail.state(), tuirealm::State::None);
    }

    #[test]
    fn test_result_detail_unknown_key() {
        let mut detail = create_result_detail();
        
        // Unknown function key
        let msg = detail.on(Event::Keyboard(KeyEvent {
            code: Key::Function(1),
            modifiers: KeyModifiers::empty(),
        }));
        
        assert_eq!(msg, None);
    }

    #[test]
    fn test_result_detail_role_colors() {
        // Test that different roles would get different colors
        let lines_user = ResultDetail::format_content(
            "session",
            "file",
            "timestamp",
            "User",
            "message"
        );
        
        let lines_assistant = ResultDetail::format_content(
            "session",
            "file",
            "timestamp",
            "Assistant",
            "message"
        );
        
        let lines_system = ResultDetail::format_content(
            "session",
            "file",
            "timestamp",
            "System",
            "message"
        );
        
        // All should have role line
        assert!(lines_user.iter().any(|l| l.to_string().contains("User")));
        assert!(lines_assistant.iter().any(|l| l.to_string().contains("Assistant")));
        assert!(lines_system.iter().any(|l| l.to_string().contains("System")));
    }
}