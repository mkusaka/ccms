#[cfg(test)]
mod tests {
    use crate::interactive_ratatui::application::cache_service::CacheService;
    use crate::interactive_ratatui::application::session_service::SessionService;
    use crate::interactive_ratatui::ui::components::Component;
    use crate::interactive_ratatui::ui::components::session_viewer::SessionViewer;
    use std::fs;
    use std::path::PathBuf;
    use std::sync::{Arc, Mutex};
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

    #[test]
    fn test_session_viewer_displays_messages_from_file() {
        // Arrange: Prepare test file and services
        let (_temp_dir, file_path) = create_test_session_file();
        let cache = Arc::new(Mutex::new(CacheService::new()));
        let session_service = SessionService::new(cache);

        // Act: Load session
        let messages = session_service
            .load_session(file_path.to_str().unwrap())
            .unwrap();
        let raw_lines = session_service
            .get_raw_lines(file_path.to_str().unwrap())
            .unwrap();

        // Assert: Verify messages are loaded correctly
        assert_eq!(messages.len(), 4, "Should have 4 messages");
        assert_eq!(raw_lines.len(), 4, "Should have 4 raw lines");

        // Verify SessionViewer can display messages
        let mut viewer = SessionViewer::new();
        viewer.set_messages(raw_lines);

        // Test viewer rendering and verify it displays in UI
        use ratatui::{Terminal, backend::TestBackend};
        let backend = TestBackend::new(120, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|f| {
                viewer.render(f, f.area());
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let content = buffer
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();

        // Verify messages are displayed
        assert!(
            content.contains("Session Messages (1/4)"),
            "Should show message count"
        );
        assert!(
            !content.contains("No messages in session"),
            "Should not show empty message"
        );
    }

    #[test]
    fn test_cache_service_parses_all_message_types() {
        // Arrange: Test data containing various message types
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("full_session.jsonl");

        let test_data = r#"{"type":"summary","summary":"Test session summary","leafUuid":"leaf-uuid-123"}
{"type":"system","content":"System starting","uuid":"system-uuid-1","timestamp":"2024-01-01T00:00:00Z","sessionId":"test-session","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/test","version":"1.0","isMeta":true}
{"type":"user","message":{"role":"user","content":"Test message"},"uuid":"user-uuid-1","timestamp":"2024-01-01T00:00:01Z","sessionId":"test-session","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}
{"type":"assistant","message":{"id":"msg_01","type":"message","role":"assistant","model":"claude-3-5-sonnet","content":[{"type":"thinking","thinking":"Let me process this...","signature":"sig123"},{"type":"text","text":"Here's my response"}],"stop_reason":"end_turn","stop_sequence":null,"usage":{"input_tokens":10,"cache_creation_input_tokens":0,"cache_read_input_tokens":0,"output_tokens":20}},"uuid":"assistant-uuid-1","timestamp":"2024-01-01T00:00:02Z","sessionId":"test-session","parentUuid":"user-uuid-1","isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}"#;

        fs::write(&file_path, test_data).unwrap();

        // Act: Load with CacheService
        let mut cache = CacheService::new();
        let cached_file = cache.get_messages(&file_path).unwrap();

        // Assert: Verify all message types are parsed correctly
        assert_eq!(cached_file.messages.len(), 4, "Should parse all 4 messages");
        assert_eq!(
            cached_file.raw_lines.len(),
            4,
            "Should have all 4 raw lines"
        );

        // Verify each message type
        assert_eq!(cached_file.messages[0].get_type(), "summary");
        assert_eq!(cached_file.messages[1].get_type(), "system");
        assert_eq!(cached_file.messages[2].get_type(), "user");
        assert_eq!(cached_file.messages[3].get_type(), "assistant");

        // Verify assistant message contains thinking
        assert!(cached_file.messages[3].has_thinking());
    }

    #[test]
    fn test_session_viewer_message_display_format() {
        // Arrange: Prepare SessionViewer
        let mut viewer = SessionViewer::new();
        let messages = vec![
            r#"{"type":"user","message":{"role":"user","content":"こんにちは Claude"},"uuid":"user-uuid-1","timestamp":"2024-01-01T12:00:00Z","sessionId":"test-session","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}"#.to_string(),
            r#"{"type":"assistant","message":{"id":"msg_01","type":"message","role":"assistant","model":"claude-3-5-sonnet","content":[{"type":"text","text":"こんにちは！今日はどのようなお手伝いができますか？"}],"stop_reason":"end_turn","stop_sequence":null,"usage":{"input_tokens":10,"cache_creation_input_tokens":0,"cache_read_input_tokens":0,"output_tokens":20}},"uuid":"assistant-uuid-1","timestamp":"2024-01-01T12:00:01Z","sessionId":"test-session","parentUuid":"user-uuid-1","isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}"#.to_string(),
        ];

        // Act: Set messages
        viewer.set_messages(messages);

        // Assert: Verify messages are set correctly
        // Test viewer rendering and verify it displays in UI
        use ratatui::{Terminal, backend::TestBackend};
        let backend = TestBackend::new(120, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|f| {
                viewer.render(f, f.area());
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let content = buffer
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();

        // Output content for debugging
        println!("=== Rendered content ===");
        println!("{content}");
        println!("=== End of content ===");

        // Verify messages are displayed in UI
        assert!(content.contains("user"), "Should display user role");
        assert!(
            content.contains("assistant"),
            "Should display assistant role"
        );
        // Check that timestamp is displayed (will be in local timezone)
        // The timestamp "2024-01-01T12:00:00Z" will be converted to local time
        use chrono::{DateTime, Local, TimeZone};
        let utc_dt = DateTime::parse_from_rfc3339("2024-01-01T12:00:00Z").unwrap();
        let local_dt = Local.from_utc_datetime(&utc_dt.naive_utc());
        let expected_time = local_dt.format("%m/%d %H:%M").to_string();
        assert!(
            content.contains(&expected_time),
            "Should display timestamp in local timezone"
        );
        // Japanese characters might be displayed with space separation in terminal UI
        assert!(
            content.contains("こ")
                && content.contains("ん")
                && content.contains("に")
                && content.contains("ち")
                && content.contains("は"),
            "Should display Japanese characters"
        );
    }

    #[test]
    fn test_edge_case_empty_content_array() {
        // Arrange: Message with empty content array
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("edge_case.jsonl");

        let test_data = r#"{"type":"assistant","message":{"id":"msg_01","type":"message","role":"assistant","model":"claude-3-5-sonnet","content":[],"stop_reason":"end_turn","stop_sequence":null,"usage":{"input_tokens":10,"cache_creation_input_tokens":0,"cache_read_input_tokens":0,"output_tokens":0}},"uuid":"assistant-uuid-1","timestamp":"2024-01-01T00:00:00Z","sessionId":"test-session","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}"#;

        fs::write(&file_path, test_data).unwrap();

        // Act: Load with CacheService
        let mut cache = CacheService::new();
        let cached_file = cache.get_messages(&file_path).unwrap();

        // Assert: Verify no crash with empty content
        assert_eq!(cached_file.messages.len(), 1);
        assert_eq!(cached_file.messages[0].get_content_text(), "");
    }

    #[test]
    fn test_malformed_json_handling() {
        // Arrange: File with mixed invalid and valid JSON
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("malformed.jsonl");

        let test_data = r#"{"type":"user","message":{"role":"user","content":"Valid message"},"uuid":"user-uuid-1","timestamp":"2024-01-01T00:00:00Z","sessionId":"test-session","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}
{invalid json here
{"type":"system","content":"Another valid message","uuid":"system-uuid-1","timestamp":"2024-01-01T00:00:01Z","sessionId":"test-session","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/test","version":"1.0","isMeta":false}"#;

        fs::write(&file_path, test_data).unwrap();

        // Act: Load with CacheService
        let mut cache = CacheService::new();
        let cached_file = cache.get_messages(&file_path).unwrap();

        // Assert: Verify only valid messages are parsed
        assert_eq!(
            cached_file.messages.len(),
            2,
            "Should parse only valid messages"
        );
        assert_eq!(
            cached_file.raw_lines.len(),
            3,
            "Should keep all non-empty lines"
        );
        assert_eq!(cached_file.messages[0].get_type(), "user");
        assert_eq!(cached_file.messages[1].get_type(), "system");
    }

    #[test]
    fn test_real_claude_session_format() {
        // Arrange: Test actual Claude Code session format
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("claude_session.jsonl");

        // Actual Claude Code message examples (conforming to schema)
        let test_data = r#"{"type":"user","message":{"role":"user","content":"Help me implement a search feature"},"uuid":"01234567-89ab-cdef-0123-456789abcdef","timestamp":"2025-01-23T12:00:00.000Z","sessionId":"session-123","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/home/user/project","version":"1.0","gitBranch":"main"}
{"type":"assistant","message":{"id":"msg_01234567890","type":"message","role":"assistant","model":"claude-3-5-sonnet-20241022","content":[{"type":"text","text":"I'll help you implement a search feature. Let me first understand your requirements."}],"stop_reason":"end_turn","stop_sequence":null,"usage":{"input_tokens":100,"cache_creation_input_tokens":0,"cache_read_input_tokens":0,"output_tokens":50}},"uuid":"12345678-9abc-def0-1234-56789abcdef0","timestamp":"2025-01-23T12:00:01.000Z","sessionId":"session-123","parentUuid":"01234567-89ab-cdef-0123-456789abcdef","isSidechain":false,"userType":"external","cwd":"/home/user/project","version":"1.0","requestId":"req-123"}
{"type":"user","message":{"role":"user","content":[{"type":"text","text":"I need a full-text search"},{"type":"tool_result","tool_use_id":"tool-123","content":"File read successfully"}]},"uuid":"23456789-abcd-ef01-2345-6789abcdef01","timestamp":"2025-01-23T12:00:02.000Z","sessionId":"session-123","parentUuid":"12345678-9abc-def0-1234-56789abcdef0","isSidechain":false,"userType":"external","cwd":"/home/user/project","version":"1.0"}
{"type":"system","content":"Tool execution completed","uuid":"34567890-bcde-f012-3456-789abcdef012","timestamp":"2025-01-23T12:00:03.000Z","sessionId":"session-123","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/home/user/project","version":"1.0","isMeta":true,"toolUseID":"tool-123"}"#;

        fs::write(&file_path, test_data).unwrap();

        // Act: Load with SessionService
        let cache = Arc::new(Mutex::new(CacheService::new()));
        let session_service = SessionService::new(cache);
        let messages = session_service
            .load_session(file_path.to_str().unwrap())
            .unwrap();
        let raw_lines = session_service
            .get_raw_lines(file_path.to_str().unwrap())
            .unwrap();

        // Assert: Verify messages are loaded correctly
        assert_eq!(messages.len(), 4, "Should parse all 4 messages");
        assert_eq!(raw_lines.len(), 4, "Should have all 4 raw lines");

        // Verify message types
        assert_eq!(messages[0].get_type(), "user");
        assert_eq!(messages[1].get_type(), "assistant");
        assert_eq!(messages[2].get_type(), "user");
        assert_eq!(messages[3].get_type(), "system");

        // Verify content
        assert_eq!(
            messages[0].get_content_text(),
            "Help me implement a search feature"
        );
        assert!(
            messages[1]
                .get_content_text()
                .contains("I'll help you implement a search feature")
        );
        assert!(
            messages[2]
                .get_content_text()
                .contains("I need a full-text search")
        );
        assert_eq!(messages[3].get_content_text(), "Tool execution completed");

        // Verify display with SessionViewer
        let mut viewer = SessionViewer::new();
        viewer.set_messages(raw_lines);
        viewer.set_session_id(Some("session-123".to_string()));

        // Test rendering
        use ratatui::{Terminal, backend::TestBackend};
        let backend = TestBackend::new(150, 40);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|f| {
                viewer.render(f, f.area());
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let content = buffer
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();

        println!("=== Real Claude Session Rendered ===");
        println!("{content}");
        println!("=== End ===");

        // Verify messages are displayed
        assert!(
            content.contains("Session: session-123"),
            "Should display session ID"
        );
        assert!(
            content.contains("Session Messages (1/4)"),
            "Should show 4 messages"
        );
        assert!(content.contains("user"), "Should display user messages");
        assert!(
            content.contains("assistant"),
            "Should display assistant messages"
        );
        assert!(content.contains("system"), "Should display system messages");
        assert!(
            content.contains("Help me implement"),
            "Should display user content"
        );
    }
}
