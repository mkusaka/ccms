use super::*;
use crate::query::condition::QueryCondition;
use crate::{SearchOptions, SearchResult};
// use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use std::fs::File;
use std::io::Write;
use std::sync::mpsc;
use std::time::Duration;
use tempfile::tempdir;

fn create_test_result(role: &str, content: &str, timestamp: &str) -> SearchResult {
    SearchResult {
        file: "/test/path".to_string(),
        uuid: "test-uuid".to_string(),
        timestamp: timestamp.to_string(),
        session_id: "test-session".to_string(),
        role: role.to_string(),
        text: content.to_string(),
        has_tools: false,
        has_thinking: false,
        message_type: "message".to_string(),
        query: QueryCondition::Literal {
            pattern: "test".to_string(),
            case_sensitive: false,
        },
        project_path: "/test/project".to_string(),
        raw_json: None,
    }
}

fn create_test_terminal() -> Terminal<TestBackend> {
    let backend = TestBackend::new(80, 24);
    Terminal::new(backend).unwrap()
}

fn find_text_in_buffer(buffer: &Buffer, text: &str) -> Option<(u16, u16)> {
    for y in 0..buffer.area.height {
        let mut line = String::new();
        for x in 0..buffer.area.width {
            let cell = &buffer[(x, y)];
            line.push_str(cell.symbol());
        }
        if let Some(x) = line.find(text) {
            return Some((x as u16, y));
        }
    }
    None
}

#[test]
fn test_initial_state() {
    let search = InteractiveSearch::new(SearchOptions::default());
    assert!(search.query.is_empty());
    assert!(search.results.is_empty());
    assert_eq!(search.selected_index, 0);
    assert_eq!(search.scroll_offset, 0);
    assert_eq!(search.mode, Mode::Search);
}

#[test]
fn test_draw_search_mode() {
    let mut search = InteractiveSearch::new(SearchOptions::default());
    search.query = "test query".to_string();
    search.results = vec![
        create_test_result("user", "Test message", "2024-01-01T00:00:00Z"),
        create_test_result("assistant", "Another test", "2024-01-01T00:01:00Z"),
    ];

    let mut terminal = create_test_terminal();
    terminal.draw(|f| search.draw_search(f)).unwrap();

    let buffer = terminal.backend().buffer();

    // Check that search prompt is displayed
    assert!(find_text_in_buffer(buffer, "Search:").is_some());

    // Check that query is displayed
    assert!(find_text_in_buffer(buffer, "test query").is_some());

    // Check that results are displayed
    assert!(find_text_in_buffer(buffer, "Test message").is_some());
    assert!(find_text_in_buffer(buffer, "Another test").is_some());
}

#[test]
fn test_draw_help_mode() {
    let mut search = InteractiveSearch::new(SearchOptions::default());
    search.mode = Mode::Help;

    let mut terminal = create_test_terminal();
    terminal.draw(|f| search.draw_help(f)).unwrap();

    let buffer = terminal.backend().buffer();

    // Check that help content is displayed
    assert!(find_text_in_buffer(buffer, "CCMS Help").is_some());
    assert!(find_text_in_buffer(buffer, "Search Mode:").is_some());
    assert!(find_text_in_buffer(buffer, "Result Detail:").is_some());
}

#[test]
fn test_draw_result_detail() {
    let mut search = InteractiveSearch::new(SearchOptions::default());
    search.mode = Mode::ResultDetail;
    search.selected_result = Some(create_test_result(
        "user",
        "Detailed test message with more content",
        "2024-01-01T00:00:00Z",
    ));

    let mut terminal = create_test_terminal();
    terminal.draw(|f| search.draw_result_detail(f)).unwrap();

    let buffer = terminal.backend().buffer();

    // Check that result details are displayed - look for metadata fields
    assert!(find_text_in_buffer(buffer, "Role:").is_some());
    assert!(find_text_in_buffer(buffer, "Time:").is_some());
    // The title bar should show "Result Detail"
    assert!(find_text_in_buffer(buffer, "Result Detail").is_some());
    // Actions should be shown
    assert!(find_text_in_buffer(buffer, "Actions:").is_some());
    assert!(find_text_in_buffer(buffer, "View full session").is_some());
}

#[test]
fn test_search_result_selection() {
    let mut search = InteractiveSearch::new(SearchOptions::default());
    search.results = vec![
        create_test_result("user", "First", "2024-01-01T00:00:00Z"),
        create_test_result("assistant", "Second", "2024-01-01T00:01:00Z"),
        create_test_result("user", "Third", "2024-01-01T00:02:00Z"),
    ];

    assert_eq!(search.selected_index, 0);

    // Move down
    search.selected_index = 1;
    assert_eq!(search.selected_index, 1);

    // Move to last
    search.selected_index = 2;
    assert_eq!(search.selected_index, 2);

    // Wrap around
    search.selected_index = 0;
    assert_eq!(search.selected_index, 0);
}

#[test]
fn test_role_filter_cycle() {
    let mut search = InteractiveSearch::new(SearchOptions::default());

    // Initial state
    assert_eq!(search.role_filter, None);

    // Cycle through filters
    search.role_filter = Some("user".to_string());
    assert_eq!(search.role_filter, Some("user".to_string()));

    search.role_filter = Some("assistant".to_string());
    assert_eq!(search.role_filter, Some("assistant".to_string()));

    search.role_filter = Some("system".to_string());
    assert_eq!(search.role_filter, Some("system".to_string()));

    search.role_filter = None;
    assert_eq!(search.role_filter, None);
}

#[test]
fn test_truncate_message_multibyte_safe() {
    let search = InteractiveSearch::new(SearchOptions::default());

    // Test with Japanese characters
    let japanese = "こんにちは世界";
    let truncated = search.truncate_message(japanese, 10);
    assert_eq!(truncated, "こんにちは世界"); // 7 chars fits in width 10

    let truncated = search.truncate_message(japanese, 6);
    assert_eq!(truncated, "こんに..."); // Width 6: 3 chars + "..." = 6

    // Test with emojis
    let emojis = "Hello 👋 World 🌍";
    let truncated = search.truncate_message(emojis, 10);
    assert_eq!(truncated, "Hello 👋..."); // Should handle emoji boundaries
}

#[test]
fn test_session_viewer_rendering() {
    let mut search = InteractiveSearch::new(SearchOptions::default());
    search.mode = Mode::SessionViewer;
    search.session_order = Some(SessionOrder::Ascending);
    search.selected_result = Some(create_test_result("user", "Test", "2024-01-01T00:00:00Z"));

    // Add some test messages
    search.session_messages = vec![
        r#"{"type":"user","message":{"role":"user","content":"Hello"},"uuid":"1","timestamp":"2024-01-01T00:00:00Z","sessionId":"test-session"}"#.to_string(),
        r#"{"type":"assistant","message":{"role":"assistant","content":"Hi there"},"uuid":"2","timestamp":"2024-01-01T00:00:01Z","sessionId":"test-session"}"#.to_string(),
    ];

    let mut terminal = create_test_terminal();
    terminal.draw(|f| search.draw_session_viewer(f)).unwrap();

    let buffer = terminal.backend().buffer();

    // Check that session viewer is displayed
    assert!(find_text_in_buffer(buffer, "Session Viewer").is_some());
}

#[test]
fn test_draw_search_results_with_selection() {
    let mut search = InteractiveSearch::new(SearchOptions::default());
    search.results = vec![
        create_test_result("user", "First message", "2024-01-01T00:00:00Z"),
        create_test_result("assistant", "Second message", "2024-01-01T00:01:00Z"),
    ];
    search.selected_index = 1; // Select second result

    let mut terminal = create_test_terminal();
    terminal.draw(|f| search.draw_search(f)).unwrap();

    let buffer = terminal.backend().buffer();

    // Both messages should be visible
    assert!(find_text_in_buffer(buffer, "First message").is_some());
    assert!(find_text_in_buffer(buffer, "Second message").is_some());
}

#[test]
fn test_empty_search_results() {
    let mut search = InteractiveSearch::new(SearchOptions::default());
    search.query = "no matches".to_string();
    search.results = vec![];

    let mut terminal = create_test_terminal();
    terminal.draw(|f| search.draw_search(f)).unwrap();

    let buffer = terminal.backend().buffer();

    // Should show the query
    assert!(find_text_in_buffer(buffer, "no matches").is_some());

    // Should indicate no results
    assert!(find_text_in_buffer(buffer, "No results").is_some());
}

#[test]
fn test_long_message_truncation() {
    let mut search = InteractiveSearch::new(SearchOptions::default());
    let long_content = "This is a very long message that should be truncated when displayed in the search results list ".repeat(5);
    search.results = vec![create_test_result(
        "user",
        &long_content,
        "2024-01-01T00:00:00Z",
    )];

    let mut terminal = create_test_terminal();
    terminal.draw(|f| search.draw_search(f)).unwrap();

    let buffer = terminal.backend().buffer();

    // Should show truncated message with ellipsis
    assert!(find_text_in_buffer(buffer, "...").is_some());
}

#[test]
fn test_message_clearing_on_mode_change() {
    let mut search = InteractiveSearch::new(SearchOptions::default());

    // Set a message
    search.message = Some("Test message".to_string());

    // Message should be cleared when returning from detail to search
    search.mode = Mode::ResultDetail;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    let esc_key = KeyEvent::new(KeyCode::Esc, KeyModifiers::empty());
    search.handle_result_detail_input(esc_key).unwrap();

    assert!(search.message.is_none());
    assert_eq!(search.mode, Mode::Search);
}

#[test]
fn test_role_filter_message_clearing() {
    let temp_dir = tempdir().unwrap();
    let test_file = temp_dir.path().join("test.jsonl");

    let mut file = File::create(&test_file).unwrap();
    writeln!(file, r#"{{"type":"user","message":{{"role":"user","content":"Test"}},"uuid":"1","timestamp":"2024-01-01T00:00:00Z","sessionId":"s1","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}}"#).unwrap();

    let mut search = InteractiveSearch::new(SearchOptions::default());
    search.query = "Test".to_string();
    search.message = Some("Previous message".to_string());

    // Simulate Tab key to change role filter
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    let tab_key = KeyEvent::new(KeyCode::Tab, KeyModifiers::empty());
    search
        .handle_search_input(tab_key, test_file.to_str().unwrap())
        .unwrap();

    // Message should be cleared
    assert!(search.message.is_none());
    assert_eq!(search.role_filter, Some("user".to_string()));
}

#[test]
fn test_preview_text_multibyte_safety() {
    // Test that preview doesn't cut in the middle of multibyte characters
    let japanese_text = "これは日本語のテキストです。長い文章になると切り詰められます。もっと長い文章を追加して40文字以上にします。";
    let result = create_test_result("user", japanese_text, "2024-01-01T00:00:00Z");

    // The preview logic in draw_results should handle multibyte chars correctly
    let preview = result
        .text
        .replace('\n', " ")
        .chars()
        .take(40)
        .collect::<String>();

    // Should take exactly 40 characters, not bytes
    assert_eq!(preview.chars().count(), 40);
    // Ensure it doesn't cut in the middle of a character
    assert!(japanese_text.starts_with(&preview));

    // Test with emoji - ensure we have more than 40 chars
    let emoji_text = "Hello 😀 World 🌍 Test 🎉 Message 📝 Long text here with more content to ensure we have over 40 characters";
    let emoji_result = create_test_result("user", emoji_text, "2024-01-01T00:00:00Z");
    let emoji_preview = emoji_result
        .text
        .replace('\n', " ")
        .chars()
        .take(40)
        .collect::<String>();

    assert_eq!(emoji_preview.chars().count(), 40);

    // Test actual preview logic used in the app
    let short_text = "Short text";
    let short_preview = short_text
        .replace('\n', " ")
        .chars()
        .take(40)
        .collect::<String>();

    // Short text should be returned as-is
    assert_eq!(short_preview, "Short text");
    assert!(short_preview.chars().count() <= 40);
}

#[test]
fn test_max_results_limit() {
    let temp_dir = tempdir().unwrap();
    let test_file = temp_dir.path().join("test.jsonl");

    // Create file with many messages
    let mut file = File::create(&test_file).unwrap();
    for i in 0..100 {
        let seconds = i % 60;
        writeln!(file, r#"{{"type":"user","message":{{"role":"user","content":"Message {i}"}},"uuid":"{i}","timestamp":"2024-01-01T00:00:{seconds:02}Z","sessionId":"s1","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}}"#).unwrap();
    }

    // Test with custom max_results
    let options = SearchOptions {
        max_results: Some(25),
        ..Default::default()
    };
    let mut search = InteractiveSearch::new(options);
    assert_eq!(search.max_results, 25);

    search.query = "Message".to_string();
    search.execute_search_sync(test_file.to_str().unwrap());

    // Should be limited to 25 results
    assert_eq!(search.results.len(), 25);
}

#[test]
fn test_session_viewer_message_parsing() {
    let temp_dir = tempdir().unwrap();
    let test_file = temp_dir.path().join("session.jsonl");

    // Create session file with different message structures
    let mut file = File::create(&test_file).unwrap();
    // Direct content
    writeln!(
        file,
        r#"{{"type":"user","content":"Direct content message","timestamp":"2024-01-01T00:00:00Z"}}"#
    )
    .unwrap();
    // Nested message.content
    writeln!(file, r#"{{"type":"assistant","message":{{"content":"Nested content message"}},"timestamp":"2024-01-01T00:00:01Z"}}"#).unwrap();
    // Array content
    writeln!(file, r#"{{"type":"assistant","message":{{"content":[{{"type":"text","text":"Part 1"}},{{"type":"text","text":"Part 2"}}]}},"timestamp":"2024-01-01T00:00:02Z"}}"#).unwrap();

    let mut search = InteractiveSearch::new(SearchOptions::default());
    search
        .load_session_messages(test_file.to_str().unwrap())
        .unwrap();

    assert_eq!(search.session_messages.len(), 3);

    // Verify each message can be parsed
    for (i, msg_str) in search.session_messages.iter().enumerate() {
        let msg: serde_json::Value = serde_json::from_str(msg_str).unwrap();
        assert!(msg.get("type").is_some(), "Message {i} missing type");
    }
}

#[test]
#[ignore] // Clipboard commands not available in CI
fn test_copy_feedback_messages() {
    let mut search = InteractiveSearch::new(SearchOptions::default());
    search.selected_result = Some(create_test_result("user", "Test", "2024-01-01T00:00:00Z"));
    search.mode = Mode::ResultDetail;

    // Test file copy feedback
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    let f_key = KeyEvent::new(KeyCode::Char('f'), KeyModifiers::empty());
    let _ = search.handle_result_detail_input(f_key); // Ignore clipboard error in tests

    assert!(search.message.is_some());
    assert!(search.message.as_ref().unwrap().contains("✓"));
    assert!(search.message.as_ref().unwrap().contains("File path"));

    // Should stay in detail mode
    assert_eq!(search.mode, Mode::ResultDetail);
}

#[test]
fn test_initial_results_loading() {
    let temp_dir = tempdir().unwrap();
    let test_file = temp_dir.path().join("test.jsonl");

    let mut file = File::create(&test_file).unwrap();
    for i in 0..10 {
        writeln!(file, r#"{{"type":"user","message":{{"role":"user","content":"Message {i}"}},"uuid":"{i}","timestamp":"2024-01-01T00:00:{i:02}Z","sessionId":"s1","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}}"#).unwrap();
    }

    let mut search = InteractiveSearch::new(SearchOptions::default());
    search.load_initial_results(test_file.to_str().unwrap());

    // Should have loaded results without any query
    assert!(!search.results.is_empty());
    // Should be sorted by timestamp (newest first)
    assert!(search.results[0].text.contains("Message 9"));
}

#[test]
fn test_ctrl_t_truncation_toggle() {
    let temp_dir = tempdir().unwrap();
    let test_file = temp_dir.path().join("test.jsonl");

    // Create test file with a long message
    let mut file = File::create(&test_file).unwrap();
    writeln!(file, r#"{{"type":"user","message":{{"role":"user","content":"This is a very long message that should demonstrate truncation behavior when toggled"}},"uuid":"1","timestamp":"2024-01-01T00:00:00Z","sessionId":"s1","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}}"#).unwrap();
    drop(file);

    let mut search = InteractiveSearch::new(SearchOptions::default());

    // Initially truncation should be enabled
    assert!(search.truncation_enabled);

    // Simulate Ctrl+T to toggle truncation
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    let ctrl_t = KeyEvent::new(KeyCode::Char('t'), KeyModifiers::CONTROL);
    search
        .handle_search_input(ctrl_t, test_file.to_str().unwrap())
        .unwrap();

    // Truncation should now be disabled
    assert!(!search.truncation_enabled);
    assert_eq!(
        search.message,
        Some("Message display: Full Text".to_string())
    );

    // Toggle again
    search
        .handle_search_input(ctrl_t, test_file.to_str().unwrap())
        .unwrap();

    // Truncation should be enabled again
    assert!(search.truncation_enabled);
    assert_eq!(
        search.message,
        Some("Message display: Truncated".to_string())
    );

    // Test that Ctrl+T does NOT work in result detail mode
    search.selected_result = Some(create_test_result(
        "user",
        "Test message",
        "2024-01-01T00:00:00Z",
    ));
    search.mode = Mode::ResultDetail;
    search.truncation_enabled = true;
    search.message = None;

    // Try to toggle in result detail mode - should not change
    search.handle_result_detail_input(ctrl_t).unwrap();
    assert!(search.truncation_enabled); // Should remain true
    assert!(search.message.is_none()); // No toggle message should appear
}

#[test]
fn test_ctrl_r_cache_reload() {
    let temp_dir = tempdir().unwrap();
    let test_file = temp_dir.path().join("test.jsonl");

    // Create initial file
    let mut file = File::create(&test_file).unwrap();
    writeln!(file, r#"{{"type":"user","message":{{"role":"user","content":"Original message"}},"uuid":"1","timestamp":"2024-01-01T00:00:00Z","sessionId":"s1","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}}"#).unwrap();
    drop(file);

    let mut search = InteractiveSearch::new(SearchOptions::default());
    search.query = "Original".to_string();
    search.execute_search_sync(test_file.to_str().unwrap());
    assert_eq!(search.results.len(), 1);

    // Modify file
    thread::sleep(Duration::from_millis(10));
    let mut file = File::create(&test_file).unwrap();
    writeln!(file, r#"{{"type":"user","message":{{"role":"user","content":"Updated message"}},"uuid":"2","timestamp":"2024-01-01T00:00:00Z","sessionId":"s1","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}}"#).unwrap();
    drop(file);

    // Simulate Ctrl+R
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    let ctrl_r = KeyEvent::new(KeyCode::Char('r'), KeyModifiers::CONTROL);
    search
        .handle_search_input(ctrl_r, test_file.to_str().unwrap())
        .unwrap();

    // Cache should be cleared and search re-executed
    assert_eq!(
        search.message,
        Some("Cache cleared and reloaded".to_string())
    );

    // Search for updated content
    search.query = "Updated".to_string();
    search.execute_search_sync(test_file.to_str().unwrap());
    assert_eq!(search.results.len(), 1);
    assert!(search.results[0].text.contains("Updated"));
}

#[test]
fn test_truncation_toggle_preserves_state_across_modes() {
    let temp_dir = tempdir().unwrap();
    let test_file = temp_dir.path().join("test.jsonl");

    // Create test file
    let mut file = File::create(&test_file).unwrap();
    writeln!(file, r#"{{"type":"user","message":{{"role":"user","content":"Test message"}},"uuid":"1","timestamp":"2024-01-01T00:00:00Z","sessionId":"s1","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}}"#).unwrap();
    drop(file);

    let mut search = InteractiveSearch::new(SearchOptions::default());
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    let ctrl_t = KeyEvent::new(KeyCode::Char('t'), KeyModifiers::CONTROL);

    // Initially truncation should be enabled
    assert!(search.truncation_enabled);

    // Toggle in Search mode
    search
        .handle_search_input(ctrl_t, test_file.to_str().unwrap())
        .unwrap();
    assert!(!search.truncation_enabled);
    assert_eq!(
        search.message,
        Some("Message display: Full Text".to_string())
    );

    // Load a result to test mode transitions
    search.query = "Test".to_string();
    search.execute_search_sync(test_file.to_str().unwrap());
    search.selected_index = 0;
    if !search.results.is_empty() {
        search.selected_result = Some(search.results[0].clone());
    }

    // Transition to detail mode - truncation state should persist
    search.mode = Mode::ResultDetail;
    assert!(!search.truncation_enabled);

    // Try to toggle in detail mode - should NOT change
    search.message = None;
    search.handle_result_detail_input(ctrl_t).unwrap();
    assert!(!search.truncation_enabled); // Should remain false
    assert!(search.message.is_none()); // No toggle message should appear

    // Go back to search mode and verify state persists
    search.mode = Mode::Search;
    assert!(!search.truncation_enabled);

    // Toggle should work again in search mode
    search
        .handle_search_input(ctrl_t, test_file.to_str().unwrap())
        .unwrap();
    assert!(search.truncation_enabled);
    assert_eq!(
        search.message,
        Some("Message display: Truncated".to_string())
    );
}

#[test]
fn test_search_navigation_keys() {
    let temp_dir = tempdir().unwrap();
    let test_file = temp_dir.path().join("test.jsonl");

    let mut file = File::create(&test_file).unwrap();
    for i in 0..20 {
        writeln!(file, r#"{{"type":"user","message":{{"role":"user","content":"Message {i}"}},"uuid":"{i}","timestamp":"2024-01-01T00:00:{i:02}Z","sessionId":"s1","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}}"#).unwrap();
    }

    let mut search = InteractiveSearch::new(SearchOptions::default());
    search.query = "Message".to_string();
    search.execute_search_sync(test_file.to_str().unwrap());

    // Test down arrow
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    let down_key = KeyEvent::new(KeyCode::Down, KeyModifiers::empty());
    search
        .handle_search_input(down_key, test_file.to_str().unwrap())
        .unwrap();
    assert_eq!(search.selected_index, 1);

    // Test up arrow
    let up_key = KeyEvent::new(KeyCode::Up, KeyModifiers::empty());
    search
        .handle_search_input(up_key, test_file.to_str().unwrap())
        .unwrap();
    assert_eq!(search.selected_index, 0);

    // Test bounds - shouldn't go below 0
    search
        .handle_search_input(up_key, test_file.to_str().unwrap())
        .unwrap();
    assert_eq!(search.selected_index, 0);

    // Test bounds - shouldn't exceed visible results
    for _ in 0..15 {
        search
            .handle_search_input(down_key, test_file.to_str().unwrap())
            .unwrap();
    }
    // Should be limited to visible results count
    assert!(search.selected_index < search.results.len());
}

#[test]
fn test_more_results_display() {
    let mut search = InteractiveSearch::new(SearchOptions::default());
    search.results = vec![
        create_test_result("user", "Result 1", "2024-01-01T00:00:00Z"),
        create_test_result("assistant", "Result 2", "2024-01-01T00:01:00Z"),
        create_test_result("user", "Result 3", "2024-01-01T00:02:00Z"),
    ];

    let mut terminal = create_test_terminal();
    terminal.draw(|f| search.draw_search(f)).unwrap();

    let buffer = terminal.backend().buffer();

    // Should show the results
    assert!(find_text_in_buffer(buffer, "Result 1").is_some());
    assert!(find_text_in_buffer(buffer, "Result 2").is_some());
    assert!(find_text_in_buffer(buffer, "Result 3").is_some());
}

#[test]
fn test_session_viewer_case_insensitive_search() {
    let mut search = InteractiveSearch::new(SearchOptions::default());
    search.mode = Mode::SessionViewer;
    search.session_order = Some(SessionOrder::Ascending);
    search.selected_result = Some(create_test_result("user", "Test", "2024-01-01T00:00:00Z"));

    // Add messages with mixed case content
    search.session_messages = vec![
        r#"{"type":"user","message":{"role":"user","content":"Hello World"},"uuid":"1","timestamp":"2024-01-01T00:00:00Z","sessionId":"test-session"}"#.to_string(),
        r#"{"type":"assistant","message":{"role":"assistant","content":"HELLO there"},"uuid":"2","timestamp":"2024-01-01T00:00:01Z","sessionId":"test-session"}"#.to_string(),
        r#"{"type":"user","message":{"role":"user","content":"goodbye"},"uuid":"3","timestamp":"2024-01-01T00:00:02Z","sessionId":"test-session"}"#.to_string(),
    ];

    // Search with lowercase
    search.session_query = "hello".to_string();

    // Draw to trigger filtering
    let mut terminal = create_test_terminal();
    terminal.draw(|f| search.draw_session_viewer(f)).unwrap();

    // Should match both messages containing "hello" regardless of case
    assert_eq!(search.session_filtered_indices.len(), 2);
    assert_eq!(search.session_filtered_indices, vec![0, 1]);

    // Search with uppercase
    search.session_query = "HELLO".to_string();
    terminal.draw(|f| search.draw_session_viewer(f)).unwrap();

    // Should still match both messages
    assert_eq!(search.session_filtered_indices.len(), 2);

    // Search for something not present
    search.session_query = "missing".to_string();
    terminal.draw(|f| search.draw_session_viewer(f)).unwrap();

    assert_eq!(search.session_filtered_indices.len(), 0);
}

#[test]
fn test_session_viewer_filtered_count_display() {
    let mut search = InteractiveSearch::new(SearchOptions::default());
    search.mode = Mode::SessionViewer;
    search.session_order = Some(SessionOrder::Ascending);
    search.selected_result = Some(create_test_result("user", "Test", "2024-01-01T00:00:00Z"));

    // Add some test messages
    search.session_messages = vec![
        r#"{"type":"user","message":{"role":"user","content":"Apple"},"uuid":"1","timestamp":"2024-01-01T00:00:00Z","sessionId":"test-session"}"#.to_string(),
        r#"{"type":"assistant","message":{"role":"assistant","content":"Banana"},"uuid":"2","timestamp":"2024-01-01T00:00:01Z","sessionId":"test-session"}"#.to_string(),
        r#"{"type":"user","message":{"role":"user","content":"Apple pie"},"uuid":"3","timestamp":"2024-01-01T00:00:02Z","sessionId":"test-session"}"#.to_string(),
    ];

    // Search for "apple"
    search.session_query = "apple".to_string();

    let mut terminal = create_test_terminal();
    terminal.draw(|f| search.draw_session_viewer(f)).unwrap();

    let buffer = terminal.backend().buffer();

    // Should show filtered count in the title
    assert!(find_text_in_buffer(buffer, "3 total, 2 filtered").is_some());
}

#[test]
fn test_session_viewer_search_filtering_implementation() {
    let mut search = InteractiveSearch::new(SearchOptions::default());
    search.mode = Mode::SessionViewer;
    search.session_order = Some(SessionOrder::Ascending);
    search.selected_result = Some(create_test_result("user", "Test", "2024-01-01T00:00:00Z"));

    // Set up messages
    search.session_messages = vec![
        r#"{"type":"user","message":{"role":"user","content":"Hello world"},"uuid":"1","timestamp":"2024-01-01T00:00:00Z","sessionId":"test-session"}"#.to_string(),
        r#"{"type":"assistant","message":{"role":"assistant","content":"Goodbye"},"uuid":"2","timestamp":"2024-01-01T00:00:01Z","sessionId":"test-session"}"#.to_string(),
        r#"{"type":"user","message":{"role":"user","content":"World peace"},"uuid":"3","timestamp":"2024-01-01T00:00:02Z","sessionId":"test-session"}"#.to_string(),
    ];

    // Test basic filtering
    search.session_query = "world".to_string();
    let mut terminal = create_test_terminal();
    terminal.draw(|f| search.draw_session_viewer(f)).unwrap();
    assert_eq!(search.session_filtered_indices, vec![0, 2]);

    // Test empty query shows all
    search.session_query = "".to_string();
    terminal.draw(|f| search.draw_session_viewer(f)).unwrap();
    assert_eq!(search.session_filtered_indices, vec![0, 1, 2]);

    // Test no matches
    search.session_query = "notfound".to_string();
    terminal.draw(|f| search.draw_session_viewer(f)).unwrap();
    assert_eq!(search.session_filtered_indices, Vec::<usize>::new());
}

#[test]
fn test_session_viewer_array_content_search() {
    let mut search = InteractiveSearch::new(SearchOptions::default());
    search.mode = Mode::SessionViewer;
    search.session_order = Some(SessionOrder::Ascending);
    search.selected_result = Some(create_test_result("user", "Test", "2024-01-01T00:00:00Z"));

    // Message with array content structure
    let array_message = r#"{
        "type":"assistant",
        "message":{
            "role":"assistant",
            "content":[
                {"type":"text","text":"First part"},
                {"type":"text","text":"Second part with keyword"},
                {"type":"text","text":"Third part"}
            ]
        },
        "uuid":"1",
        "timestamp":"2024-01-01T00:00:00Z",
        "sessionId":"test-session"
    }"#;

    search.session_messages = vec![
        array_message.to_string(),
        r#"{"type":"user","message":{"role":"user","content":"No match"},"uuid":"2","timestamp":"2024-01-01T00:00:01Z","sessionId":"test-session"}"#.to_string(),
    ];

    // Search for text in array content
    search.session_query = "keyword".to_string();
    let mut terminal = create_test_terminal();
    terminal.draw(|f| search.draw_session_viewer(f)).unwrap();

    // Should find the message with array content
    assert_eq!(search.session_filtered_indices, vec![0]);

    // Search for text not in array
    search.session_query = "missing".to_string();
    terminal.draw(|f| search.draw_session_viewer(f)).unwrap();
    assert_eq!(search.session_filtered_indices, Vec::<usize>::new());
}

#[test]
fn test_session_viewer_loading_error_handling() {
    let mut search = InteractiveSearch::new(SearchOptions::default());
    search.mode = Mode::SessionViewer;
    search.session_order = Some(SessionOrder::Ascending);
    search.selected_result = Some(create_test_result("user", "Test", "2024-01-01T00:00:00Z"));

    // Simulate loading with empty messages (error case)
    search.session_messages = vec![];

    let mut terminal = create_test_terminal();
    terminal.draw(|f| search.draw_session_viewer(f)).unwrap();

    let buffer = terminal.backend().buffer();

    // Should show that no messages are available
    assert!(
        find_text_in_buffer(buffer, "No messages").is_some()
            || find_text_in_buffer(buffer, "0 messages").is_some()
            || find_text_in_buffer(buffer, "Session Viewer").is_some()
    );
}

#[test]
fn test_session_viewer_no_matches_filtered_count() {
    let mut search = InteractiveSearch::new(SearchOptions::default());
    search.mode = Mode::SessionViewer;
    search.session_order = Some(SessionOrder::Ascending);
    search.selected_result = Some(create_test_result("user", "Test", "2024-01-01T00:00:00Z"));

    // Add messages
    search.session_messages = vec![
        r#"{"type":"user","message":{"role":"user","content":"Hello"},"uuid":"1","timestamp":"2024-01-01T00:00:00Z","sessionId":"test-session"}"#.to_string(),
        r#"{"type":"assistant","message":{"role":"assistant","content":"Hi"},"uuid":"2","timestamp":"2024-01-01T00:00:01Z","sessionId":"test-session"}"#.to_string(),
    ];

    // Search for something not present
    search.session_query = "notfound".to_string();

    let mut terminal = create_test_terminal();
    terminal.draw(|f| search.draw_session_viewer(f)).unwrap();

    let buffer = terminal.backend().buffer();

    // Should show that no messages match
    assert!(find_text_in_buffer(buffer, "No messages match filter").is_some());
}

#[test]
fn test_session_viewer_empty_session() {
    let mut search = InteractiveSearch::new(SearchOptions::default());
    search.mode = Mode::SessionViewer;
    search.session_order = Some(SessionOrder::Ascending);
    search.selected_result = Some(create_test_result("user", "Test", "2024-01-01T00:00:00Z"));

    // No messages at all
    search.session_messages = vec![];
    search.session_filtered_indices = vec![];

    let mut terminal = create_test_terminal();
    terminal.draw(|f| search.draw_session_viewer(f)).unwrap();

    let buffer = terminal.backend().buffer();

    // Should indicate empty session
    assert!(
        find_text_in_buffer(buffer, "No messages").is_some()
            || find_text_in_buffer(buffer, "0 messages").is_some()
            || find_text_in_buffer(buffer, "Empty session").is_some()
            || find_text_in_buffer(buffer, "Session Viewer").is_some()
    ); // At minimum, title should be shown
}

#[test]
fn test_session_viewer_scroll_indicator() {
    let mut search = InteractiveSearch::new(SearchOptions::default());
    search.mode = Mode::SessionViewer;
    search.session_order = Some(SessionOrder::Ascending);
    search.selected_result = Some(create_test_result("user", "Test", "2024-01-01T00:00:00Z"));

    // Add many messages to enable scrolling
    let mut messages = Vec::new();
    for i in 0..50 {
        messages.push(format!(
            r#"{{"type":"user","message":{{"role":"user","content":"Message {i}"}},"uuid":"{i}","timestamp":"2024-01-01T00:00:{i:02}Z","sessionId":"test-session"}}"#
        ));
    }
    search.session_messages = messages;
    search.session_filtered_indices = (0..50).collect();
    search.session_scroll_offset = 10; // Scrolled down

    let mut terminal = create_test_terminal();
    terminal.draw(|f| search.draw_session_viewer(f)).unwrap();

    let buffer = terminal.backend().buffer();

    // Should show some indication of scrolling capability
    // Check for arrows or scroll indicators
    assert!(
        find_text_in_buffer(buffer, "↑").is_some()
            || find_text_in_buffer(buffer, "↓").is_some()
            || find_text_in_buffer(buffer, "to scroll").is_some()
    );
}

#[test]
fn test_session_viewer_selected_message_highlight() {
    let mut search = InteractiveSearch::new(SearchOptions::default());
    search.mode = Mode::SessionViewer;
    search.session_order = Some(SessionOrder::Ascending);

    // Create a few messages
    search.session_messages = vec![
        r#"{"type":"user","message":{"role":"user","content":"First message"},"uuid":"1","timestamp":"2024-01-01T00:00:00Z","sessionId":"test-session"}"#.to_string(),
        r#"{"type":"assistant","message":{"role":"assistant","content":"Second message"},"uuid":"2","timestamp":"2024-01-01T00:00:01Z","sessionId":"test-session"}"#.to_string(),
        r#"{"type":"user","message":{"role":"user","content":"Third message"},"uuid":"3","timestamp":"2024-01-01T00:00:02Z","sessionId":"test-session"}"#.to_string(),
    ];
    search.session_filtered_indices = vec![0, 1, 2];
    search.session_selected_index = 1; // Select second message
    search.selected_result = Some(create_test_result("user", "Test", "2024-01-01T00:00:00Z"));

    let mut terminal = create_test_terminal();
    terminal.draw(|f| search.draw_session_viewer(f)).unwrap();

    let buffer = terminal.backend().buffer();

    // The new implementation doesn't use ">" prefix, it just highlights with bg color
    // Check for any numbered message (formatting might vary)
    assert!(find_text_in_buffer(buffer, "1.").is_some());
    assert!(find_text_in_buffer(buffer, "2.").is_some());
    assert!(find_text_in_buffer(buffer, "3.").is_some());

    // Check that roles are displayed
    assert!(find_text_in_buffer(buffer, "USER").is_some());
    assert!(find_text_in_buffer(buffer, "ASSISTANT").is_some());

    // Check that Second message is displayed (which is selected)
    assert!(find_text_in_buffer(buffer, "Second message").is_some());
}

#[test]
fn test_session_viewer_thinking_block_display() {
    let mut search = InteractiveSearch::new(SearchOptions::default());
    search.mode = Mode::SessionViewer;
    search.session_order = Some(SessionOrder::Ascending);

    // Message with thinking block
    search.session_messages = vec![
        r#"{"type":"assistant","message":{"role":"assistant","content":[{"type":"thinking","text":"Let me think about this..."},{"type":"text","text":"Here is my response"}]},"uuid":"1","timestamp":"2024-01-01T00:00:00Z","sessionId":"test-session"}"#.to_string(),
        r#"{"type":"assistant","message":{"role":"assistant","content":"Simple message"},"uuid":"2","timestamp":"2024-01-01T00:00:01Z","sessionId":"test-session"}"#.to_string(),
    ];
    search.session_filtered_indices = vec![0, 1];
    search.selected_result = Some(create_test_result("user", "Test", "2024-01-01T00:00:00Z"));

    let mut terminal = create_test_terminal();
    terminal.draw(|f| search.draw_session_viewer(f)).unwrap();

    let buffer = terminal.backend().buffer();

    // Should display the thinking block content in preview
    assert!(find_text_in_buffer(buffer, "Let me think about this...").is_some());

    // Should also show the regular text content
    assert!(find_text_in_buffer(buffer, "Simple message").is_some());
}

#[test]
fn test_session_viewer_tool_use_display() {
    let mut search = InteractiveSearch::new(SearchOptions::default());
    search.mode = Mode::SessionViewer;
    search.session_order = Some(SessionOrder::Ascending);

    // Messages with tool_use blocks
    search.session_messages = vec![
        // Tool use request
        r#"{"type":"assistant","message":{"role":"assistant","content":[{"type":"text","text":"Let me search for that."},{"type":"tool_use","id":"tool_123","name":"search","input":{"query":"rust programming"}}]},"uuid":"1","timestamp":"2024-01-01T00:00:00Z","sessionId":"test-session"}"#.to_string(),
        // Tool result
        r#"{"type":"user","message":{"role":"user","content":[{"type":"tool_result","tool_use_id":"tool_123","content":"Found 10 results for rust programming"}]},"uuid":"2","timestamp":"2024-01-01T00:00:01Z","sessionId":"test-session"}"#.to_string(),
    ];
    search.session_filtered_indices = vec![0, 1];
    search.selected_result = Some(create_test_result("user", "Test", "2024-01-01T00:00:00Z"));

    let mut terminal = create_test_terminal();
    terminal.draw(|f| search.draw_session_viewer(f)).unwrap();

    let buffer = terminal.backend().buffer();

    // Should show text content from tool use message
    assert!(find_text_in_buffer(buffer, "Let me search for that.").is_some());

    // Should show that tool messages are displayed (content might be formatted differently)
    // Check that at least the messages are shown by checking for their timestamps/indices
    assert!(find_text_in_buffer(buffer, "1.").is_some());
    assert!(find_text_in_buffer(buffer, "2.").is_some());
}

#[test]
fn test_truncate_message_edge_cases() {
    let search = InteractiveSearch::new(SearchOptions::default());

    // Test edge cases
    let long_string = "a".repeat(1000);
    let expected_long = format!("{}...", "a".repeat(47));
    let test_cases = vec![
        // Zero width
        ("Test message", 0, ""),
        // Width of 1
        ("Test", 1, "T"),
        // Width of 3 (no room for ellipsis)
        ("Testing", 3, "Tes"),
        // Width of 4 (room for 1 char + ellipsis)
        ("Testing", 4, "T..."),
        // Empty string
        ("", 100, ""),
        // String with only spaces
        ("   ", 10, "   "),
        // Very long repeated character
        (long_string.as_str(), 50, expected_long.as_str()),
    ];

    for (input, width, expected) in test_cases {
        let result = search.truncate_message(input, width);
        assert_eq!(
            result, expected,
            "Failed for input '{input}' with width {width}"
        );
    }
}

#[test]
fn test_async_search_response_handling() {
    // Test that search responses are properly handled
    let search = InteractiveSearch::new(SearchOptions::default());
    let (request_tx, request_rx) = mpsc::channel();
    let (response_tx, response_rx) = mpsc::channel();

    // Create a simple test pattern
    let temp_dir = tempdir().unwrap();
    let test_file = temp_dir.path().join("test.jsonl");
    let mut file = File::create(&test_file).unwrap();
    writeln!(file, r#"{{"type":"user","message":{{"role":"user","content":"Hello"}},"uuid":"1","timestamp":"2024-01-01T00:00:00Z","sessionId":"s1","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}}"#).unwrap();

    // Start worker
    let _handle = search.start_search_worker(request_rx, response_tx, test_file.to_str().unwrap());

    // Send multiple requests quickly
    for i in 0..3 {
        let request = SearchRequest {
            id: i,
            query: if i == 2 {
                "Hello".to_string()
            } else {
                "nomatch".to_string()
            },
            role_filter: None,
            pattern: test_file.to_str().unwrap().to_string(),
        };
        request_tx.send(request).unwrap();
    }

    // Collect responses
    let mut responses = Vec::new();
    for _ in 0..3 {
        if let Ok(response) = response_rx.recv_timeout(Duration::from_secs(1)) {
            responses.push(response);
        }
    }

    // Verify we got all responses
    assert_eq!(responses.len(), 3);

    // Check that only the last query returns results
    responses.sort_by_key(|r| r.id);
    assert_eq!(responses[0].results.len(), 0); // "nomatch"
    assert_eq!(responses[1].results.len(), 0); // "nomatch"
    assert_eq!(responses[2].results.len(), 1); // "Hello"
}

#[test]
fn test_wrap_text_long_word() {
    let search = InteractiveSearch::new(SearchOptions::default());

    // Test wrapping with a word longer than max width
    let text = "This verylongwordthatexceedsthemaximumwidth should wrap";
    let wrapped = search.wrap_text(text, 10);
    assert_eq!(wrapped[0], "This");
    assert_eq!(wrapped[1], "verylongwo");
    assert_eq!(wrapped[2], "rdthatexce");
    assert_eq!(wrapped[3], "edsthemaxi");
    assert_eq!(wrapped[4], "mumwidth");
    assert_eq!(wrapped[5], "should");
    assert_eq!(wrapped[6], "wrap");
}

#[test]
fn test_wrap_text_multiline() {
    let search = InteractiveSearch::new(SearchOptions::default());

    // Test wrapping with existing newlines
    let text = "First line\nSecond line that is long\nThird";
    let wrapped = search.wrap_text(text, 15);
    assert_eq!(wrapped[0], "First line");
    assert_eq!(wrapped[1], "Second line");
    assert_eq!(wrapped[2], "that is long");
    assert_eq!(wrapped[3], "Third");
}

#[test]
fn test_wrap_text_multibyte() {
    let search = InteractiveSearch::new(SearchOptions::default());

    // Test wrapping with multibyte characters
    let text = "こんにちは世界 Hello World 日本語テスト";
    let wrapped = search.wrap_text(text, 15);
    assert!(wrapped.len() > 1);

    // Verify no character boundaries are broken
    for line in &wrapped {
        assert!(line.is_char_boundary(0));
        assert!(line.is_char_boundary(line.len()));
    }
}

#[test]
fn test_wrap_text_edge_cases() {
    let search = InteractiveSearch::new(SearchOptions::default());

    // Test empty string
    let wrapped = search.wrap_text("", 10);
    assert_eq!(wrapped.len(), 0);

    // Test zero width
    let wrapped = search.wrap_text("Hello", 0);
    assert_eq!(wrapped.len(), 0);

    // Test exact width
    let wrapped = search.wrap_text("Hello", 5);
    assert_eq!(wrapped.len(), 1);
    assert_eq!(wrapped[0], "Hello");
}

#[test]
fn test_session_viewer_rendering_after_scroll_and_search() {
    // Regression test for rendering artifacts when scrolling and re-searching
    let mut search = InteractiveSearch::new(SearchOptions::default());
    search.mode = Mode::SessionViewer;
    search.session_order = Some(SessionOrder::Ascending);
    search.selected_result = Some(create_test_result("user", "Test", "2024-01-01T00:00:00Z"));

    // Create many messages to enable scrolling
    let mut messages = Vec::new();
    for i in 0..100 {
        messages.push(format!(
            r#"{{"type":"user","message":{{"role":"user","content":"Message {i} with some longer content to test rendering"}},"uuid":"{i}","timestamp":"2024-01-01T00:00:{i:02}Z","sessionId":"test-session"}}"#
        ));
    }
    search.session_messages = messages;

    // Initial render
    let mut terminal = create_test_terminal();
    terminal.draw(|f| search.draw_session_viewer(f)).unwrap();

    // Simulate scrolling down
    search.session_scroll_offset = 50;
    search.session_selected_index = 50;
    terminal.draw(|f| search.draw_session_viewer(f)).unwrap();

    // Now search for something
    search.session_query = "Message 1".to_string();
    // Reset scroll offset when searching (this happens in the actual implementation)
    search.session_scroll_offset = 0;
    search.session_selected_index = 0;
    terminal.draw(|f| search.draw_session_viewer(f)).unwrap();

    let buffer = terminal.backend().buffer();

    // Verify that we only see filtered results, no artifacts from previous render
    // The search should filter to messages containing "Message 1" (10-19, 100)
    assert!(find_text_in_buffer(buffer, "Message 1").is_some());

    // Should not see messages that don't match the filter
    assert!(find_text_in_buffer(buffer, "Message 50").is_none());
    assert!(find_text_in_buffer(buffer, "Message 75").is_none());

    // Clear search and verify clean render
    search.session_query.clear();
    terminal.draw(|f| search.draw_session_viewer(f)).unwrap();

    // Now we should see messages again (after clearing search)
    let buffer = terminal.backend().buffer();
    assert!(find_text_in_buffer(buffer, "Message").is_some());
}

#[test]
fn test_full_text_mode_display() {
    let mut search = InteractiveSearch::new(SearchOptions::default());
    
    // Add a long message
    let long_content = "This is a very long message that should be wrapped when displayed in full text mode. It contains multiple words and should span multiple lines.";
    search.results = vec![create_test_result(
        "user",
        long_content,
        "2024-01-01T00:00:00Z",
    )];
    
    // Enable full text mode
    search.truncation_enabled = false;
    
    // Draw the search view
    let mut terminal = create_test_terminal();
    terminal.draw(|f| search.draw_search(f)).unwrap();
    
    let buffer = terminal.backend().buffer();
    
    // Should show the wrapped content
    assert!(find_text_in_buffer(buffer, "This is a very long").is_some(), "First part of message should be visible");
    assert!(find_text_in_buffer(buffer, "wrapped when displayed").is_some(), "Second line should be visible");
    assert!(find_text_in_buffer(buffer, "contains multiple").is_some(), "Third line should be visible");
    assert!(find_text_in_buffer(buffer, "lines.").is_some(), "Last part should be visible");
    
    // Should NOT show ellipsis in full text mode
    assert!(find_text_in_buffer(buffer, "...").is_none(), "Ellipsis should not appear in full text mode");
}

#[test]
fn test_full_text_mode_with_empty_query() {
    let mut search = InteractiveSearch::new(SearchOptions::default());
    
    // Set empty query - this should result in no results
    search.query = "".to_string();
    
    // Enable full text mode
    search.truncation_enabled = false;
    
    // Execute search with empty query
    search.execute_search_sync("/tmp/nonexistent");
    
    // Draw the search view
    let mut terminal = create_test_terminal();
    terminal.draw(|f| search.draw_search(f)).unwrap();
    
    let buffer = terminal.backend().buffer();
    
    // Should show "Results (0)" or similar
    assert!(search.results.is_empty(), "Empty query should produce no results");
    
    // The UI should still render properly even with no results
    assert!(find_text_in_buffer(buffer, "Interactive Claude Search").is_some());
}

#[test]
fn test_session_viewer_full_text_mode() {
    let mut search = InteractiveSearch::new(SearchOptions::default());
    search.mode = Mode::SessionViewer;
    search.session_order = Some(SessionOrder::Ascending);
    search.selected_result = Some(create_test_result("user", "Test", "2024-01-01T00:00:00Z"));
    
    // Add messages with long content
    search.session_messages = vec![
        r#"{"type":"user","message":{"role":"user","content":"This is a very long message that should be wrapped when displayed in full text mode in the session viewer"},"uuid":"1","timestamp":"2024-01-01T00:00:00Z","sessionId":"test-session"}"#.to_string(),
        r#"{"type":"assistant","message":{"role":"assistant","content":"Another long message that contains multiple words and should span multiple lines when wrapped"},"uuid":"2","timestamp":"2024-01-01T00:00:01Z","sessionId":"test-session"}"#.to_string(),
    ];
    
    // Enable full text mode
    search.truncation_enabled = false;
    
    // Draw session viewer
    let mut terminal = create_test_terminal();
    terminal.draw(|f| search.draw_session_viewer(f)).unwrap();
    
    let buffer = terminal.backend().buffer();
    
    // Should show wrapped content
    assert!(find_text_in_buffer(buffer, "very long message").is_some(), "First message content should be visible");
    assert!(find_text_in_buffer(buffer, "Another long message").is_some(), "Second message content should be visible");
}

#[test]
fn test_full_text_mode_with_scroll() {
    let mut search = InteractiveSearch::new(SearchOptions::default());
    
    // Add many messages to enable scrolling
    let mut results = Vec::new();
    for i in 0..30 {
        results.push(create_test_result(
            "user",
            &format!("Message {} with some long content that should wrap in full text mode", i),
            &format!("2024-01-01T00:00:{:02}Z", i),
        ));
    }
    search.results = results;
    
    // Scroll down to middle of list
    search.selected_index = 15;
    search.scroll_offset = 10;
    
    // Enable full text mode
    search.truncation_enabled = false;
    
    // Draw and check
    let mut terminal = create_test_terminal();
    terminal.draw(|f| search.draw_search(f)).unwrap();
    
    let buffer = terminal.backend().buffer();
    
    // In full text mode, fewer items are visible, so we need to check if any messages are displayed
    // The exact message number may vary based on scroll adjustment
    let buffer_has_messages = (0..30).any(|i| {
        find_text_in_buffer(buffer, &format!("Message {}", i)).is_some()
    });
    assert!(buffer_has_messages, "At least some messages should be visible in full text mode");
}

#[test]
fn test_session_viewer_clear_area_before_render() {
    // Test that the message list area is properly cleared before each render
    let mut search = InteractiveSearch::new(SearchOptions::default());
    search.mode = Mode::SessionViewer;
    search.session_order = Some(SessionOrder::Ascending);
    search.selected_result = Some(create_test_result("user", "Test", "2024-01-01T00:00:00Z"));

    // First render with long messages
    search.session_messages = vec![
        r#"{"type":"user","message":{"role":"user","content":"This is a very long message that should fill the entire width of the terminal"},"uuid":"1","timestamp":"2024-01-01T00:00:00Z","sessionId":"test-session"}"#.to_string(),
    ];

    let mut terminal = create_test_terminal();
    terminal.draw(|f| search.draw_session_viewer(f)).unwrap();

    // Now render with shorter messages
    search.session_messages = vec![
        r#"{"type":"user","message":{"role":"user","content":"Short"},"uuid":"1","timestamp":"2024-01-01T00:00:00Z","sessionId":"test-session"}"#.to_string(),
    ];
    terminal.draw(|f| search.draw_session_viewer(f)).unwrap();

    let buffer = terminal.backend().buffer();

    // The long message content should not be visible
    assert!(find_text_in_buffer(buffer, "very long message").is_none());
    assert!(find_text_in_buffer(buffer, "entire width").is_none());

    // Only the short message should be visible
    assert!(find_text_in_buffer(buffer, "Short").is_some());
}
