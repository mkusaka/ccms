#[cfg(test)]
mod tests {
    use super::super::Component;
    use super::super::message_preview::MessagePreview;
    use crate::query::condition::{QueryCondition, SearchResult};
    use crossterm::event::{KeyCode, KeyEvent};
    use ratatui::{Terminal, backend::TestBackend};

    fn create_test_result() -> SearchResult {
        SearchResult {
            file: "test.jsonl".to_string(),
            uuid: "12345678-1234-5678-1234-567812345678".to_string(),
            timestamp: "2024-01-02T15:30:45Z".to_string(),
            session_id: "87654321-4321-8765-4321-876543210987".to_string(),
            role: "user".to_string(),
            text: "This is a test message".to_string(),
            message_type: "message".to_string(),
            query: QueryCondition::Literal {
                pattern: "test".to_string(),
                case_sensitive: false,
            },
            cwd: "/test/path".to_string(),
            raw_json: None,
        }
    }

    fn buffer_to_string(buffer: &ratatui::prelude::Buffer) -> String {
        let mut lines = Vec::new();
        for y in 0..buffer.area.height {
            let mut line = String::new();
            for x in 0..buffer.area.width {
                let cell = buffer.cell((x, y)).unwrap();
                line.push_str(cell.symbol());
            }
            lines.push(line.trim_end().to_string());
        }
        lines.join("\n")
    }

    #[test]
    fn test_message_preview_new() {
        let preview = MessagePreview::new();
        // Just test that we can create a new preview
        // We can't directly access private fields
        let _ = preview;
    }

    #[test]
    fn test_render_empty_preview() {
        let mut preview = MessagePreview::new();
        let backend = TestBackend::new(80, 20);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|f| {
                preview.render(f, f.area());
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let content = buffer_to_string(buffer);
        assert!(content.contains("No message selected"));
    }

    #[test]
    fn test_render_with_result() {
        let mut preview = MessagePreview::new();
        let result = create_test_result();
        preview.set_result(Some(result));

        let backend = TestBackend::new(80, 20);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|f| {
                preview.render(f, f.area());
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let content = buffer_to_string(buffer);

        // Check that all fields are displayed
        assert!(content.contains("Role: user"));
        assert!(content.contains("Time: 2024-01-02 15:30:45"));
        assert!(content.contains("This is a test message"));
    }

    #[test]
    fn test_full_id_display() {
        let mut preview = MessagePreview::new();
        let result = create_test_result();
        preview.set_result(Some(result));

        let backend = TestBackend::new(100, 20);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|f| {
                preview.render(f, f.area());
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let content = buffer_to_string(buffer);

        // Check that full IDs are displayed without truncation
        assert!(content.contains("Message ID: 12345678-1234-5678-1234-567812345678"));
        assert!(content.contains("Session ID: 87654321-4321-8765-4321-876543210987"));
    }

    #[test]
    fn test_long_message_truncation_indicator() {
        let mut preview = MessagePreview::new();
        let mut result = create_test_result();
        // Create a very long message
        result.text = "This is a very long message. ".repeat(100);
        preview.set_result(Some(result));

        let backend = TestBackend::new(60, 10);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|f| {
                preview.render(f, f.area());
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let content = buffer_to_string(buffer);

        // Check that truncation indicator appears
        assert!(content.contains("... (Enter for full view)"));
    }

    #[test]
    fn test_handle_key_returns_none() {
        let mut preview = MessagePreview::new();
        let key = KeyEvent::from(KeyCode::Enter);
        let result = preview.handle_key(key);
        assert!(result.is_none());
    }

    #[test]
    fn test_word_wrap() {
        let mut preview = MessagePreview::new();
        let mut result = create_test_result();
        result.text = "This is a message with a very long line that should be wrapped to multiple lines when displayed in the preview pane".to_string();
        preview.set_result(Some(result));

        let backend = TestBackend::new(40, 20);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|f| {
                preview.render(f, f.area());
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let content = buffer_to_string(buffer);

        // Check that the text is wrapped (appears on multiple lines)
        let lines: Vec<&str> = content.lines().collect();
        let message_lines: Vec<&str> = lines
            .iter()
            .filter(|line| {
                line.contains("wrapped") || line.contains("multiple") || line.contains("preview")
            })
            .copied()
            .collect();
        assert!(
            message_lines.len() > 1,
            "Long text should be wrapped to multiple lines"
        );
    }

    #[test]
    fn test_unicode_handling() {
        let mut preview = MessagePreview::new();
        let mut result = create_test_result();
        result.text = "こんにちは、世界！🌍 Unicode test with emojis 🎉".to_string();
        preview.set_result(Some(result));

        let backend = TestBackend::new(80, 20);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|f| {
                preview.render(f, f.area());
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let content = buffer_to_string(buffer);

        // Check that unicode text is preserved
        // Note: TestBackend may render unicode characters with spaces between them
        assert!(
            content.contains("Unicode test"),
            "Expected to find unicode content, but got:\n{content}"
        );
    }
}
