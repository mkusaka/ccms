#[cfg(test)]
mod tests {
    use crate::interactive_ratatui::ui::components::Component;
    use crate::interactive_ratatui::ui::components::session_viewer::SessionViewer;
    use crate::query::condition::{QueryCondition, SearchResult};
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use ratatui::{Terminal, backend::TestBackend};
    use std::fs;
    use std::path::PathBuf;
    use tempfile::TempDir;

    fn create_test_session_file() -> (TempDir, PathBuf) {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test_session.jsonl");

        // Test data conforming to ccmeta schema
        let test_data = r#"{"type":"user","message":{"role":"user","content":"Hello Claude"},"uuid":"user-uuid-1","timestamp":"2024-01-01T00:00:00Z","sessionId":"test-session","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}
{"type":"assistant","message":{"id":"msg_01","type":"message","role":"assistant","model":"claude-3-5-sonnet","content":[{"type":"text","text":"Hello! How can I help you today?"}],"stop_reason":"end_turn","stop_sequence":null,"usage":{"input_tokens":10,"cache_creation_input_tokens":0,"cache_read_input_tokens":0,"output_tokens":20}},"uuid":"assistant-uuid-1","timestamp":"2024-01-01T00:00:01Z","sessionId":"test-session","parentUuid":"user-uuid-1","isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}
{"type":"user","message":{"role":"user","content":"Show me a file"},"uuid":"user-uuid-2","timestamp":"2024-01-01T00:00:02Z","sessionId":"test-session","parentUuid":"assistant-uuid-1","isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}
{"type":"assistant","message":{"id":"msg_02","type":"message","role":"assistant","model":"claude-3-5-sonnet","content":[{"type":"text","text":"I'll read the file for you."},{"type":"tool_use","id":"tool_1","name":"read_file","input":{"file_path":"test.txt"}}],"stop_reason":"tool_use","stop_sequence":null,"usage":{"input_tokens":15,"cache_creation_input_tokens":0,"cache_read_input_tokens":0,"output_tokens":25}},"uuid":"assistant-uuid-2","timestamp":"2024-01-01T00:00:03Z","sessionId":"test-session","parentUuid":"user-uuid-2","isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}"#;

        fs::write(&file_path, test_data).unwrap();
        (temp_dir, file_path)
    }

    fn create_test_search_results() -> Vec<SearchResult> {
        vec![
            SearchResult {
                file: "/path/to/session.jsonl".to_string(),
                uuid: "user-uuid-1".to_string(),
                timestamp: "2024-01-01T00:00:00Z".to_string(),
                session_id: "test-session".to_string(),
                role: "user".to_string(),
                text: "Hello Claude".to_string(),
                message_type: "message".to_string(),
                query: QueryCondition::Literal {
                    pattern: String::new(),
                    case_sensitive: false,
                },
                cwd: "/test".to_string(),
                raw_json: Some(r#"{"type":"user","message":{"role":"user","content":"Hello Claude"}}"#.to_string()),
            },
            SearchResult {
                file: "/path/to/session.jsonl".to_string(),
                uuid: "assistant-uuid-1".to_string(),
                timestamp: "2024-01-01T00:00:01Z".to_string(),
                session_id: "test-session".to_string(),
                role: "assistant".to_string(),
                text: "Hello! How can I help you today?".to_string(),
                message_type: "message".to_string(),
                query: QueryCondition::Literal {
                    pattern: String::new(),
                    case_sensitive: false,
                },
                cwd: "/test".to_string(),
                raw_json: Some(r#"{"type":"assistant","message":{"role":"assistant","content":"Hello! How can I help you today?"}}"#.to_string()),
            },
        ]
    }

    #[test]
    fn test_session_viewer_displays_messages() {
        let mut viewer = SessionViewer::new();

        // Set test data
        let results = create_test_search_results();
        viewer.set_results(results);
        viewer.set_session_id(Some("test-session".to_string()));
        viewer.set_file_path(Some("/path/to/session.jsonl".to_string()));

        // Render the viewer to test it doesn't panic
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|f| {
                viewer.render(f, f.area());
            })
            .unwrap();

        // Check that the buffer contains expected content
        let buffer = terminal.backend().buffer();
        let content = buffer
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();

        // Verify session viewer title is present
        assert!(content.contains("Session Viewer"));
    }

    #[test]
    fn test_session_viewer_search_functionality() {
        let mut viewer = SessionViewer::new();
        let results = create_test_search_results();
        viewer.set_results(results);

        // Start search mode
        let key = KeyEvent::new(KeyCode::Char('/'), KeyModifiers::NONE);
        viewer.handle_key(key);
        assert!(viewer.is_searching());

        // Type search query
        for ch in ['h', 'e', 'l', 'l', 'o'] {
            let key = KeyEvent::new(KeyCode::Char(ch), KeyModifiers::NONE);
            viewer.handle_key(key);
        }

        // Exit search mode
        let key = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
        viewer.handle_key(key);
        assert!(!viewer.is_searching());
    }

    #[test]
    fn test_session_viewer_navigation() {
        let mut viewer = SessionViewer::new();
        let results = create_test_search_results();
        viewer.set_results(results);

        // Test navigation keys
        let initial_index = viewer.get_selected_index();

        // Move down
        let key = KeyEvent::new(KeyCode::Down, KeyModifiers::NONE);
        viewer.handle_key(key);
        assert!(viewer.get_selected_index() > initial_index);

        // Move up
        let key = KeyEvent::new(KeyCode::Up, KeyModifiers::NONE);
        viewer.handle_key(key);
        assert_eq!(viewer.get_selected_index(), initial_index);
    }

    #[test]
    fn test_session_viewer_with_empty_results() {
        let mut viewer = SessionViewer::new();

        // Set empty results
        viewer.set_results(Vec::new());

        // Render should not panic
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|f| {
                viewer.render(f, f.area());
            })
            .unwrap();
    }

    #[test]
    fn test_session_viewer_copy_shortcuts() {
        let mut viewer = SessionViewer::new();

        viewer.set_file_path(Some("/test/path.jsonl".to_string()));
        viewer.set_session_id(Some("test-session-id".to_string()));

        // Test copy shortcuts when not in search mode
        use crate::interactive_ratatui::ui::events::{CopyContent, Message};

        // Test 'f' for file path copy
        let key = KeyEvent::new(KeyCode::Char('f'), KeyModifiers::NONE);
        let result = viewer.handle_key(key);
        assert!(matches!(
            result,
            Some(Message::CopyToClipboard(CopyContent::FilePath(_)))
        ));

        // Test 'i' for session ID copy
        let key = KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE);
        let result = viewer.handle_key(key);
        assert!(matches!(
            result,
            Some(Message::CopyToClipboard(CopyContent::SessionId(_)))
        ));
    }

    #[test]
    fn test_session_viewer_preview_toggle() {
        let mut viewer = SessionViewer::new();
        let results = create_test_search_results();
        viewer.set_results(results);

        // Toggle preview with Ctrl+T
        let key = KeyEvent::new(KeyCode::Char('t'), KeyModifiers::CONTROL);
        let result = viewer.handle_key(key);

        use crate::interactive_ratatui::ui::events::Message;
        assert!(matches!(result, Some(Message::ToggleSessionPreview)));
    }

    #[test]
    fn test_session_viewer_order_toggle() {
        let mut viewer = SessionViewer::new();

        // Toggle order with Ctrl+O
        let key = KeyEvent::new(KeyCode::Char('o'), KeyModifiers::CONTROL);
        let result = viewer.handle_key(key);

        use crate::interactive_ratatui::ui::events::Message;
        assert!(matches!(result, Some(Message::ToggleSessionOrder)));
    }

    #[test]
    fn test_session_viewer_role_filter_toggle() {
        let mut viewer = SessionViewer::new();

        // Toggle role filter with Tab
        let key = KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE);
        let result = viewer.handle_key(key);

        use crate::interactive_ratatui::ui::events::Message;
        assert!(matches!(result, Some(Message::ToggleSessionRoleFilter)));
    }
}
