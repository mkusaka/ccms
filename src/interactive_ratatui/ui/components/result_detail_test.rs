#[cfg(test)]
mod tests {
    use super::super::result_detail::ResultDetail;
    use crate::interactive_ratatui::ui::components::Component;
    use crate::interactive_ratatui::ui::events::{CopyContent, Message};
    use crate::query::condition::{QueryCondition, SearchResult};
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use ratatui::{Terminal, backend::TestBackend, buffer::Buffer};

    fn create_test_result() -> SearchResult {
        SearchResult {
            file: "/path/to/test.jsonl".to_string(),
            project_path: "/path/to/project".to_string(),
            uuid: "12345678-1234-5678-1234-567812345678".to_string(),
            session_id: "session-123".to_string(),
            timestamp: "2024-01-01T00:00:00Z".to_string(),
            role: "user".to_string(),
            text: "This is a test message".to_string(),
            has_tools: false,
            has_thinking: false,
            message_type: "user".to_string(),
            query: QueryCondition::Literal {
                pattern: String::new(),
                case_sensitive: false,
            },
            raw_json: Some(
                r#"{"type":"user","message":{"content":"This is a test message"}}"#.to_string(),
            ),
        }
    }

    fn create_test_result_with_long_text() -> SearchResult {
        let mut result = create_test_result();
        result.text = "This is a very long message that should wrap when displayed in the terminal. It contains multiple sentences and should demonstrate the text wrapping functionality that we just implemented. The wrapping should respect word boundaries when possible and handle Unicode characters correctly.".to_string();
        result
    }

    fn create_test_result_with_long_file_path() -> SearchResult {
        let mut result = create_test_result();
        result.file = "/Users/masatomokusaka/.claude/projects/very-long-project-name/session-files/0ff88f7e-99a2-4c72-b7c1-fb95713d1832.jsonl".to_string();
        result
    }

    fn create_test_result_with_long_project_path() -> SearchResult {
        let mut result = create_test_result();
        result.project_path = "/Users/masatomokusaka/src/github/com/organization/very-long-project-name-with-multiple-segments/sub-project/workspace".to_string();
        result
    }

    fn create_test_result_with_all_long_fields() -> SearchResult {
        let mut result = create_test_result();
        result.file = "/Users/masatomokusaka/.claude/projects/very-long-project-name/session-files/0ff88f7e-99a2-4c72-b7c1-fb95713d1832.jsonl".to_string();
        result.project_path = "/Users/masatomokusaka/src/github/com/organization/very-long-project-name-with-multiple-segments/sub-project/workspace".to_string();
        result.session_id = "extremely-long-session-id-0ff88f7e-99a2-4c72-b7c1-fb95713d1832-with-additional-segments".to_string();
        result.uuid =
            "12345678-1234-5678-1234-567812345678-extra-long-uuid-with-additional-information"
                .to_string();
        result
    }

    fn render_component(component: &mut ResultDetail, width: u16, height: u16) -> Buffer {
        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|f| {
                component.render(f, f.area());
            })
            .unwrap();

        terminal.backend().buffer().clone()
    }

    #[test]
    fn test_result_detail_new() {
        let detail = ResultDetail::new();
        assert!(detail.result.is_none());
        assert_eq!(detail.scroll_offset, 0);
        assert!(detail.message.is_none());
    }

    #[test]
    fn test_set_result() {
        let mut detail = ResultDetail::new();
        let result = create_test_result();

        detail.set_result(result.clone());
        assert!(detail.result.is_some());
        assert_eq!(
            detail.result.unwrap().uuid,
            "12345678-1234-5678-1234-567812345678"
        );
        assert_eq!(detail.scroll_offset, 0);
    }

    #[test]
    fn test_clear() {
        let mut detail = ResultDetail::new();
        let result = create_test_result();

        detail.set_result(result);
        detail.scroll_offset = 5;
        detail.clear();

        assert!(detail.result.is_none());
        assert_eq!(detail.scroll_offset, 0);
    }

    #[test]
    fn test_set_message() {
        let mut detail = ResultDetail::new();

        detail.set_message(Some("Test message".to_string()));
        assert_eq!(detail.message, Some("Test message".to_string()));

        detail.set_message(None);
        assert!(detail.message.is_none());
    }

    #[test]
    fn test_text_wrapping() {
        let mut detail = ResultDetail::new();
        let result = create_test_result_with_long_text();
        detail.set_result(result);

        // Render with narrow width to test wrapping
        let buffer = render_component(&mut detail, 80, 30);

        // Convert buffer to string for easier inspection
        let content = buffer
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();

        // The long text should be wrapped across multiple lines
        // Check that the text exists somewhere in the rendered content
        assert!(content.contains("This is a very long message"));
        // The text might be wrapped, so just check for parts of it
        assert!(content.contains("wrap") || content.contains("displayed"));
    }

    #[test]
    fn test_long_file_path_wrapping() {
        let mut detail = ResultDetail::new();
        let result = create_test_result_with_long_file_path();
        detail.set_result(result);

        // Render with narrow width to force wrapping
        let buffer = render_component(&mut detail, 50, 30);

        // Convert buffer to string for easier inspection
        let content = buffer
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();

        // Check that file path components are present
        assert!(content.contains("File:"));
        assert!(content.contains("/Users/masatomokusaka"));
        assert!(content.contains(".jsonl"));

        // The title should also be present
        assert!(content.contains("Result Detail"));
    }

    #[test]
    fn test_long_project_path_wrapping() {
        let mut detail = ResultDetail::new();
        let result = create_test_result_with_long_project_path();
        detail.set_result(result);

        // Render with narrow width to force wrapping
        let buffer = render_component(&mut detail, 50, 30);

        // Convert buffer to string for easier inspection
        let content = buffer
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();

        // Check that project path components are present
        assert!(content.contains("Project:"));
        assert!(content.contains("/Users/masatomokusaka"));
        assert!(content.contains("workspace"));

        // The title should also be present
        assert!(content.contains("Result Detail"));
    }

    #[test]
    fn test_scroll_navigation() {
        let mut detail = ResultDetail::new();
        let result = create_test_result();
        detail.set_result(result);

        // Test scroll down
        let msg = detail.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::empty()));
        assert!(msg.is_none());
        assert_eq!(detail.scroll_offset, 1);

        // Test scroll down with 'j'
        let msg = detail.handle_key(KeyEvent::new(KeyCode::Char('j'), KeyModifiers::empty()));
        assert!(msg.is_none());
        assert_eq!(detail.scroll_offset, 2);

        // Test scroll up
        let msg = detail.handle_key(KeyEvent::new(KeyCode::Up, KeyModifiers::empty()));
        assert!(msg.is_none());
        assert_eq!(detail.scroll_offset, 1);

        // Test scroll up with 'k'
        let msg = detail.handle_key(KeyEvent::new(KeyCode::Char('k'), KeyModifiers::empty()));
        assert!(msg.is_none());
        assert_eq!(detail.scroll_offset, 0);

        // Test scroll up at top (should stay at 0)
        let msg = detail.handle_key(KeyEvent::new(KeyCode::Up, KeyModifiers::empty()));
        assert!(msg.is_none());
        assert_eq!(detail.scroll_offset, 0);
    }

    #[test]
    fn test_page_navigation() {
        let mut detail = ResultDetail::new();
        let result = create_test_result();
        detail.set_result(result);

        // Test page down
        let msg = detail.handle_key(KeyEvent::new(KeyCode::PageDown, KeyModifiers::empty()));
        assert!(msg.is_none());
        assert_eq!(detail.scroll_offset, 10);

        // Test page up
        let msg = detail.handle_key(KeyEvent::new(KeyCode::PageUp, KeyModifiers::empty()));
        assert!(msg.is_none());
        assert_eq!(detail.scroll_offset, 0);

        // Test page up from middle position
        detail.scroll_offset = 15;
        let msg = detail.handle_key(KeyEvent::new(KeyCode::PageUp, KeyModifiers::empty()));
        assert!(msg.is_none());
        assert_eq!(detail.scroll_offset, 5);
    }

    #[test]
    fn test_copy_shortcuts() {
        let mut detail = ResultDetail::new();
        let result = create_test_result();
        detail.set_result(result);

        // Test copy file path (F)
        let msg = detail.handle_key(KeyEvent::new(KeyCode::Char('f'), KeyModifiers::empty()));
        assert!(
            matches!(msg, Some(Message::CopyToClipboard(CopyContent::FilePath(path))) if path == "/path/to/test.jsonl")
        );

        // Test copy session ID (I)
        let msg = detail.handle_key(KeyEvent::new(KeyCode::Char('i'), KeyModifiers::empty()));
        assert!(
            matches!(msg, Some(Message::CopyToClipboard(CopyContent::SessionId(id))) if id == "session-123")
        );

        // Test copy project path (P)
        let msg = detail.handle_key(KeyEvent::new(KeyCode::Char('p'), KeyModifiers::empty()));
        assert!(
            matches!(msg, Some(Message::CopyToClipboard(CopyContent::ProjectPath(path))) if path == "/path/to/project")
        );

        // Test copy message text (M)
        let msg = detail.handle_key(KeyEvent::new(KeyCode::Char('m'), KeyModifiers::empty()));
        assert!(
            matches!(msg, Some(Message::CopyToClipboard(CopyContent::MessageContent(text))) if text == "This is a test message")
        );

        // Test copy raw JSON (R)
        let msg = detail.handle_key(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::empty()));
        assert!(
            matches!(msg, Some(Message::CopyToClipboard(CopyContent::JsonData(json))) if json.contains("user"))
        );

        // Test copy with 'c' (should copy message text)
        let msg = detail.handle_key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::empty()));
        assert!(
            matches!(msg, Some(Message::CopyToClipboard(CopyContent::MessageContent(text))) if text == "This is a test message")
        );
    }

    #[test]
    fn test_navigation_shortcuts() {
        let mut detail = ResultDetail::new();
        let result = create_test_result();
        detail.set_result(result);

        // Test enter session viewer (Ctrl+S)
        let msg = detail.handle_key(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::CONTROL));
        assert!(matches!(msg, Some(Message::EnterSessionViewer)));

        // Test exit to search (Esc)
        let msg = detail.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::empty()));
        assert!(matches!(msg, Some(Message::ExitToSearch)));
    }

    #[test]
    fn test_all_fields_wrapping() {
        let mut detail = ResultDetail::new();
        let result = create_test_result_with_all_long_fields();
        detail.set_result(result);

        // Render with very narrow width to force wrapping of all fields
        // Use larger height to see all wrapped content
        let buffer = render_component(&mut detail, 40, 50);

        // Convert buffer to string for easier inspection
        let content = buffer
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();

        // Check that all fields are present
        assert!(content.contains("File:"));
        assert!(content.contains("Project:"));
        assert!(content.contains("Session:"));
        assert!(content.contains("UUID:"));

        // Check that long values are present and wrapped
        assert!(content.contains("/Users/masatomokusaka"));
        // The wrapping has occurred, so check for wrapped parts
        assert!(content.contains("0ff88f7e")); // Part of the file name
        assert!(content.contains("sonl")); // "jsonl" is wrapped as "j" + "sonl"
        assert!(content.contains("rganization")); // "organization" is wrapped
        assert!(content.contains("ace")); // "workspace" is wrapped as "worksp" + "ace"
        assert!(content.contains("extremely-long-session-id")); // Session ID starts correctly
        assert!(content.contains("al-segments")); // Session ID ends correctly
        assert!(content.contains("xtra-long-uuid")); // UUID is wrapped as "e" + "xtra-long-uuid"
        assert!(content.contains("tion")); // UUID ends with "tion"
    }

    #[test]
    fn test_copy_without_result() {
        let mut detail = ResultDetail::new();
        // Don't set any result

        // All copy operations should return None when no result is set
        let msg = detail.handle_key(KeyEvent::new(KeyCode::Char('f'), KeyModifiers::empty()));
        assert!(msg.is_none());

        let msg = detail.handle_key(KeyEvent::new(KeyCode::Char('i'), KeyModifiers::empty()));
        assert!(msg.is_none());

        let msg = detail.handle_key(KeyEvent::new(KeyCode::Char('p'), KeyModifiers::empty()));
        assert!(msg.is_none());

        let msg = detail.handle_key(KeyEvent::new(KeyCode::Char('m'), KeyModifiers::empty()));
        assert!(msg.is_none());

        let msg = detail.handle_key(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::empty()));
        assert!(msg.is_none());

        let msg = detail.handle_key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::empty()));
        assert!(msg.is_none());
    }

    #[test]
    fn test_unicode_text_wrapping() {
        let mut detail = ResultDetail::new();
        let mut result = create_test_result();
        result.text = "これは日本語のテストメッセージです。絵文字も含まれています🎉。長いテキストが正しく折り返されることを確認します。".to_string();
        detail.set_result(result);

        // Render with narrow width to test Unicode wrapping
        let buffer = render_component(&mut detail, 40, 20);

        // The component should render without panicking on Unicode boundaries
        // This test mainly ensures the Unicode-safe wrapping logic works
        assert_eq!(buffer.area.width, 40);
        assert_eq!(buffer.area.height, 20);
    }

    #[test]
    fn test_uppercase_shortcuts() {
        let mut detail = ResultDetail::new();
        let result = create_test_result();
        detail.set_result(result);

        // Test uppercase variants of shortcuts
        let msg = detail.handle_key(KeyEvent::new(KeyCode::Char('F'), KeyModifiers::empty()));
        assert!(
            matches!(msg, Some(Message::CopyToClipboard(CopyContent::FilePath(path))) if path == "/path/to/test.jsonl")
        );

        let msg = detail.handle_key(KeyEvent::new(KeyCode::Char('I'), KeyModifiers::empty()));
        assert!(
            matches!(msg, Some(Message::CopyToClipboard(CopyContent::SessionId(id))) if id == "session-123")
        );

        // S key no longer triggers session viewer - need Ctrl+S instead
    }

    #[test]
    fn test_render_without_result() {
        let mut detail = ResultDetail::new();

        // Should not panic when rendering without a result
        let buffer = render_component(&mut detail, 80, 24);

        // The buffer should be mostly empty (just the default terminal state)
        let non_empty_cells = buffer
            .content
            .iter()
            .filter(|cell| cell.symbol() != " ")
            .count();
        assert_eq!(non_empty_cells, 0);
    }

    #[test]
    fn test_copy_raw_json_fallback() {
        let mut detail = ResultDetail::new();
        let mut result = create_test_result();
        result.raw_json = None; // No raw JSON available
        detail.set_result(result);

        // Should create a formatted string when raw_json is None
        let msg = detail.handle_key(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::empty()));
        if let Some(Message::CopyToClipboard(content)) = msg {
            match content {
                CopyContent::FullResultDetails(text) => {
                    assert!(text.contains("File: /path/to/test.jsonl"));
                    assert!(text.contains("UUID: 12345678-1234-5678-1234-567812345678"));
                    assert!(text.contains("Session ID: session-123"));
                    assert!(text.contains("Role: user"));
                    assert!(text.contains("Text: This is a test message"));
                    assert!(text.contains("Project: /path/to/project"));
                }
                _ => panic!("Expected FullResultDetails variant"),
            }
        } else {
            panic!("Expected CopyToClipboard message");
        }
    }
}
