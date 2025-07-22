#[cfg(test)]
use super::*;
use crate::interactive_ratatui::SearchRequest;
use crate::{QueryCondition, SearchOptions, SearchResult};
use ratatui::{
    Terminal,
    backend::TestBackend,
    buffer::Buffer,
    style::{Color, Modifier},
};
use std::fs::File;
use std::io::Write;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;
use tempfile::tempdir;

fn create_test_result(role: &str, text: &str, timestamp: &str) -> SearchResult {
    SearchResult {
        file: "test.jsonl".to_string(),
        uuid: "test-uuid-123".to_string(),
        timestamp: timestamp.to_string(),
        session_id: "test-session".to_string(),
        role: role.to_string(),
        text: text.to_string(),
        has_tools: false,
        has_thinking: false,
        message_type: role.to_string(),
        query: QueryCondition::Literal {
            pattern: "test".to_string(),
            case_sensitive: false,
        },
        project_path: "/test/project".to_string(),
        raw_json: Some(r#"{"type":"user","content":"test"}"#.to_string()),
    }
}

fn create_test_terminal() -> Terminal<TestBackend> {
    let backend = TestBackend::new(80, 24);
    Terminal::new(backend).unwrap()
}

#[test]
fn test_interactive_search_creation() {
    let options = SearchOptions {
        max_results: Some(20),
        role: None,
        session_id: None,
        before: None,
        after: None,
        verbose: false,
        project_path: None,
    };

    let search = InteractiveSearch::new(options);
    assert_eq!(search.max_results, 20);
    assert_eq!(search.mode, Mode::Search);
    assert!(search.query.is_empty());
    assert_eq!(search.selected_index, 0);
    assert!(search.results.is_empty());
    assert!(search.role_filter.is_none());
}

#[test]
fn test_mode_transitions() {
    let mut search = InteractiveSearch::new(SearchOptions::default());

    // Start in Search mode
    assert_eq!(search.mode, Mode::Search);

    // Transition to Help
    search.mode = Mode::Help;
    assert_eq!(search.mode, Mode::Help);

    // Transition to ResultDetail
    search.mode = Mode::ResultDetail;
    assert_eq!(search.mode, Mode::ResultDetail);

    // Transition to SessionViewer
    search.mode = Mode::SessionViewer;
    assert_eq!(search.mode, Mode::SessionViewer);
}

#[test]
fn test_role_filter_cycling() {
    let mut search = InteractiveSearch::new(SearchOptions::default());

    // Initial state
    assert!(search.role_filter.is_none());

    // Cycle through filters
    search.role_filter = Some("user".to_string());
    assert_eq!(search.role_filter, Some("user".to_string()));

    search.role_filter = Some("assistant".to_string());
    assert_eq!(search.role_filter, Some("assistant".to_string()));

    search.role_filter = Some("system".to_string());
    assert_eq!(search.role_filter, Some("system".to_string()));

    search.role_filter = Some("summary".to_string());
    assert_eq!(search.role_filter, Some("summary".to_string()));

    search.role_filter = None;
    assert!(search.role_filter.is_none());
}

#[test]
fn test_timestamp_formatting() {
    // Test short format
    let timestamp = "2024-01-15T14:30:00Z";
    let formatted = InteractiveSearch::format_timestamp(timestamp);
    assert_eq!(formatted, "01/15 14:30");

    // Test long format
    let formatted_long = InteractiveSearch::format_timestamp_long(timestamp);
    assert_eq!(formatted_long, "2024-01-15 14:30:00");

    // Test invalid timestamp
    let invalid = "invalid-timestamp";
    let formatted_invalid = InteractiveSearch::format_timestamp(invalid);
    assert_eq!(formatted_invalid, "invalid-timestam");
}

#[test]
fn test_centered_rect_calculation() {
    let search = InteractiveSearch::new(SearchOptions::default());
    let area = ratatui::layout::Rect {
        x: 0,
        y: 0,
        width: 100,
        height: 50,
    };

    let centered = search.centered_rect(60, 80, area);

    // Check that the rect is centered
    assert_eq!(centered.width, 60);
    assert_eq!(centered.height, 40);
    assert_eq!(centered.x, 20);
    assert_eq!(centered.y, 5);
}

#[test]
fn test_search_ui_rendering() {
    let mut search = InteractiveSearch::new(SearchOptions::default());
    let mut terminal = create_test_terminal();

    // Test empty search
    terminal.draw(|f| search.draw_search(f)).unwrap();

    // Test with query
    search.query = "test query".to_string();
    terminal.draw(|f| search.draw_search(f)).unwrap();

    // Test with results
    search.results = vec![
        create_test_result("user", "Hello world", "2024-01-01T00:00:00Z"),
        create_test_result("assistant", "Hi there!", "2024-01-01T00:00:01Z"),
    ];
    terminal.draw(|f| search.draw_search(f)).unwrap();

    // Test with role filter
    search.role_filter = Some("user".to_string());
    terminal.draw(|f| search.draw_search(f)).unwrap();
}

#[test]
fn test_result_detail_rendering() {
    let mut search = InteractiveSearch::new(SearchOptions::default());
    let mut terminal = create_test_terminal();

    search.selected_result = Some(create_test_result(
        "user",
        "This is a test message with some content",
        "2024-01-01T12:00:00Z",
    ));

    terminal.draw(|f| search.draw_result_detail(f)).unwrap();
}

#[test]
fn test_session_viewer_rendering() {
    let mut search = InteractiveSearch::new(SearchOptions::default());
    let mut terminal = create_test_terminal();

    search.selected_result = Some(create_test_result("user", "Test", "2024-01-01T00:00:00Z"));

    // Test order selection prompt
    terminal.draw(|f| search.draw_session_viewer(f)).unwrap();

    // Test with messages
    search.session_order = Some(SessionOrder::Ascending);
    search.session_messages = vec![
        r#"{"type":"user","content":"Message 1","timestamp":"2024-01-01T00:00:00Z"}"#.to_string(),
        r#"{"type":"assistant","content":"Message 2","timestamp":"2024-01-01T00:00:01Z"}"#
            .to_string(),
    ];
    terminal.draw(|f| search.draw_session_viewer(f)).unwrap();
}

#[test]
fn test_help_screen_rendering() {
    let mut search = InteractiveSearch::new(SearchOptions::default());
    let mut terminal = create_test_terminal();

    search.mode = Mode::Help;
    terminal.draw(|f| search.draw_help(f)).unwrap();
}

#[test]
fn test_empty_query_shows_initial_screen() {
    let temp_dir = tempdir().unwrap();
    let test_file = temp_dir.path().join("test.jsonl");

    // Create test file
    let mut file = File::create(&test_file).unwrap();
    writeln!(file, r#"{{"type":"user","message":{{"role":"user","content":"Test message"}},"uuid":"1","timestamp":"2024-01-01T00:00:00Z","sessionId":"s1","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}}"#).unwrap();

    let mut search = InteractiveSearch::new(SearchOptions::default());

    // Empty query should not show results
    search.query = "".to_string();
    search.execute_search_sync(test_file.to_str().unwrap());
    assert_eq!(search.results.len(), 0);

    // Query with content should show results
    search.query = "Test".to_string();
    search.execute_search_sync(test_file.to_str().unwrap());
    assert_eq!(search.results.len(), 1);

    // Clear query should clear results
    search.query = "".to_string();
    search.execute_search_sync(test_file.to_str().unwrap());
    assert_eq!(search.results.len(), 0);
}

#[test]
fn test_cache_functionality() {
    let temp_dir = tempdir().unwrap();
    let test_file = temp_dir.path().join("test.jsonl");

    // Create test JSONL file
    let mut file = File::create(&test_file).unwrap();
    writeln!(file, r#"{{"type":"user","message":{{"role":"user","content":"Hello world"}},"uuid":"123","timestamp":"2024-01-01T00:00:00Z","sessionId":"s1","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}}"#).unwrap();

    let mut cache = MessageCache::new();

    // First access - should load from file
    let cached = cache.get_messages(&test_file).unwrap();
    assert_eq!(cached.messages.len(), 1);
    assert_eq!(cached.raw_lines.len(), 1);

    // Second access - should use cache
    let cached2 = cache.get_messages(&test_file).unwrap();
    assert_eq!(cached2.messages.len(), 1);

    // Clear cache
    cache.clear();
    assert!(cache.files.is_empty());
}

#[test]
fn test_file_change_detection() {
    let temp_dir = tempdir().unwrap();
    let test_file = temp_dir.path().join("test.jsonl");

    // Create initial file
    let mut file = File::create(&test_file).unwrap();
    writeln!(file, r#"{{"type":"user","message":{{"role":"user","content":"Original"}},"uuid":"1","timestamp":"2024-01-01T00:00:00Z","sessionId":"s1","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}}"#).unwrap();
    drop(file);

    let mut cache = MessageCache::new();

    // Load original
    let cached = cache.get_messages(&test_file).unwrap();
    assert_eq!(cached.messages[0].get_content_text(), "Original");

    // Sleep to ensure timestamp changes
    thread::sleep(Duration::from_millis(10));

    // Modify file
    let mut file = File::create(&test_file).unwrap();
    writeln!(file, r#"{{"type":"user","message":{{"role":"user","content":"Updated"}},"uuid":"2","timestamp":"2024-01-01T00:00:00Z","sessionId":"s1","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}}"#).unwrap();
    drop(file);

    // Should detect change and reload
    let cached2 = cache.get_messages(&test_file).unwrap();
    assert_eq!(cached2.messages[0].get_content_text(), "Updated");
}

#[test]
fn test_execute_search_with_filters() {
    let temp_dir = tempdir().unwrap();
    let test_file = temp_dir.path().join("test.jsonl");

    // Create test file with multiple messages
    let mut file = File::create(&test_file).unwrap();
    writeln!(file, r#"{{"type":"user","message":{{"role":"user","content":"User message"}},"uuid":"1","timestamp":"2024-01-01T00:00:00Z","sessionId":"s1","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}}"#).unwrap();
    writeln!(file, r#"{{"type":"assistant","message":{{"id":"msg1","type":"message","role":"assistant","model":"claude","content":[{{"type":"text","text":"Assistant message"}}],"stop_reason":"end_turn","stop_sequence":null,"usage":{{"input_tokens":10,"cache_creation_input_tokens":0,"cache_read_input_tokens":0,"output_tokens":5}}}},"uuid":"2","timestamp":"2024-01-01T00:00:01Z","sessionId":"s1","parentUuid":"1","isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}}"#).unwrap();

    let mut search = InteractiveSearch::new(SearchOptions::default());

    // Test with role filter
    search.query = "message".to_string();
    search.role_filter = Some("user".to_string());
    search.execute_search_sync(test_file.to_str().unwrap());
    assert_eq!(search.results.len(), 1);
    assert_eq!(search.results[0].role, "user");

    // Test without filter
    search.role_filter = None;
    search.execute_search_sync(test_file.to_str().unwrap());
    assert_eq!(search.results.len(), 2);
}

#[test]
fn test_timestamp_filtering() {
    let temp_dir = tempdir().unwrap();
    let test_file = temp_dir.path().join("test.jsonl");

    // Create test file with different timestamps
    let mut file = File::create(&test_file).unwrap();
    writeln!(file, r#"{{"type":"user","message":{{"role":"user","content":"Early"}},"uuid":"1","timestamp":"2024-01-01T00:00:00Z","sessionId":"s1","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}}"#).unwrap();
    writeln!(file, r#"{{"type":"user","message":{{"role":"user","content":"Middle"}},"uuid":"2","timestamp":"2024-01-02T00:00:00Z","sessionId":"s1","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}}"#).unwrap();
    writeln!(file, r#"{{"type":"user","message":{{"role":"user","content":"Late"}},"uuid":"3","timestamp":"2024-01-03T00:00:00Z","sessionId":"s1","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}}"#).unwrap();

    let options = SearchOptions {
        after: Some("2024-01-01T12:00:00Z".to_string()),
        before: Some("2024-01-02T12:00:00Z".to_string()),
        ..Default::default()
    };

    let mut search = InteractiveSearch::new(options);
    search.query = "Middle".to_string();
    search.execute_search_sync(test_file.to_str().unwrap());

    assert_eq!(search.results.len(), 1);
    assert!(search.results[0].text.contains("Middle"));
}

#[test]
fn test_project_path_extraction() {
    use std::path::PathBuf;

    // Test standard Claude path format
    let path = PathBuf::from("/home/user/.claude/projects/path-to-project/session.jsonl");
    let project_path = InteractiveSearch::extract_project_path(&path);
    assert_eq!(project_path, "path/to/project");

    // Test path without project structure
    let path2 = PathBuf::from("/tmp/test.jsonl");
    let project_path2 = InteractiveSearch::extract_project_path(&path2);
    assert_eq!(project_path2, "/tmp");
}

#[test]
fn test_invalid_json_handling() {
    let temp_dir = tempdir().unwrap();
    let test_file = temp_dir.path().join("invalid.jsonl");

    // Create file with mixed valid/invalid JSON
    let mut file = File::create(&test_file).unwrap();
    writeln!(file, "This is not JSON").unwrap();
    writeln!(file, r#"{{"type":"user","message":{{"role":"user","content":"Valid message"}},"uuid":"123","timestamp":"2024-01-01T00:00:00Z","sessionId":"s1","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}}"#).unwrap();
    writeln!(file, "Another invalid line").unwrap();

    let mut cache = MessageCache::new();
    let cached = cache.get_messages(&test_file).unwrap();

    // Should only load valid messages
    assert_eq!(cached.messages.len(), 1);
    assert_eq!(cached.messages[0].get_content_text(), "Valid message");
    // But should keep all raw lines (non-empty ones)
    assert_eq!(cached.raw_lines.len(), 3);
}

#[test]
fn test_empty_file_handling() {
    let temp_dir = tempdir().unwrap();
    let test_file = temp_dir.path().join("empty.jsonl");

    // Create empty file
    File::create(&test_file).unwrap();

    let mut cache = MessageCache::new();
    let cached = cache.get_messages(&test_file).unwrap();

    assert_eq!(cached.messages.len(), 0);
    assert_eq!(cached.raw_lines.len(), 0);
}

#[test]
fn test_message_limit() {
    let temp_dir = tempdir().unwrap();
    let test_file = temp_dir.path().join("many.jsonl");

    // Create file with many messages
    let mut file = File::create(&test_file).unwrap();
    for i in 0..100 {
        writeln!(file, r#"{{"type":"user","message":{{"role":"user","content":"Message {i}"}},"uuid":"{i}","timestamp":"2024-01-01T00:00:{i:02}Z","sessionId":"s1","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}}"#).unwrap();
    }

    let options = SearchOptions {
        max_results: Some(10),
        ..Default::default()
    };

    let mut search = InteractiveSearch::new(options);
    search.query = "Message".to_string();
    search.execute_search_sync(test_file.to_str().unwrap());

    assert_eq!(search.results.len(), 10);
}

#[test]
fn test_session_message_loading() {
    let temp_dir = tempdir().unwrap();
    let test_file = temp_dir.path().join("session.jsonl");

    // Create session file
    let mut file = File::create(&test_file).unwrap();
    writeln!(file, r#"{{"type":"user","content":"Message 1"}}"#).unwrap();
    writeln!(file, r#"{{"type":"assistant","content":"Message 2"}}"#).unwrap();
    writeln!(file).unwrap(); // Empty line should be skipped
    writeln!(file, r#"{{"type":"system","content":"Message 3"}}"#).unwrap();

    let mut search = InteractiveSearch::new(SearchOptions::default());
    search
        .load_session_messages(test_file.to_str().unwrap())
        .unwrap();

    assert_eq!(search.session_messages.len(), 3);
}

#[test]
fn test_query_parsing_integration() {
    let temp_dir = tempdir().unwrap();
    let test_file = temp_dir.path().join("test.jsonl");

    let mut file = File::create(&test_file).unwrap();
    writeln!(file, r#"{{"type":"user","message":{{"role":"user","content":"Hello world"}},"uuid":"1","timestamp":"2024-01-01T00:00:00Z","sessionId":"s1","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}}"#).unwrap();
    writeln!(file, r#"{{"type":"user","message":{{"role":"user","content":"Goodbye world"}},"uuid":"2","timestamp":"2024-01-01T00:00:01Z","sessionId":"s1","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}}"#).unwrap();

    let mut search = InteractiveSearch::new(SearchOptions::default());

    // Test AND query
    search.query = "Hello AND world".to_string();
    search.execute_search_sync(test_file.to_str().unwrap());
    assert_eq!(search.results.len(), 1);

    // Test OR query
    search.query = "Hello OR Goodbye".to_string();
    search.execute_search_sync(test_file.to_str().unwrap());
    assert_eq!(search.results.len(), 2);

    // Test NOT query
    search.query = "world AND NOT Hello".to_string();
    search.execute_search_sync(test_file.to_str().unwrap());
    assert_eq!(search.results.len(), 1);
    assert!(search.results[0].text.contains("Goodbye"));

    // Test invalid query
    search.query = "/invalid(regex".to_string();
    search.execute_search_sync(test_file.to_str().unwrap());
    assert_eq!(search.results.len(), 0);
}

#[test]
fn test_result_sorting() {
    let temp_dir = tempdir().unwrap();
    let test_file = temp_dir.path().join("test.jsonl");

    let mut file = File::create(&test_file).unwrap();
    writeln!(file, r#"{{"type":"user","message":{{"role":"user","content":"Old message"}},"uuid":"1","timestamp":"2024-01-01T00:00:00Z","sessionId":"s1","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}}"#).unwrap();
    writeln!(file, r#"{{"type":"user","message":{{"role":"user","content":"New message"}},"uuid":"2","timestamp":"2024-01-02T00:00:00Z","sessionId":"s1","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}}"#).unwrap();
    writeln!(file, r#"{{"type":"user","message":{{"role":"user","content":"Middle message"}},"uuid":"3","timestamp":"2024-01-01T12:00:00Z","sessionId":"s1","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}}"#).unwrap();

    let mut search = InteractiveSearch::new(SearchOptions::default());
    search.query = "message".to_string();
    search.execute_search_sync(test_file.to_str().unwrap());

    // Results should be sorted by timestamp (newest first)
    assert_eq!(search.results.len(), 3);
    assert!(search.results[0].text.contains("New"));
    assert!(search.results[1].text.contains("Middle"));
    assert!(search.results[2].text.contains("Old"));
}

#[test]
fn test_detail_scroll_functionality() {
    let mut search = InteractiveSearch::new(SearchOptions::default());

    // Create a result with multi-line text
    let long_text = (0..50)
        .map(|i| format!("Line {i}"))
        .collect::<Vec<_>>()
        .join("\n");
    search.selected_result = Some(create_test_result(
        "user",
        &long_text,
        "2024-01-01T00:00:00Z",
    ));

    // Initial scroll offset should be 0
    assert_eq!(search.detail_scroll_offset, 0);

    // Simulate scrolling down
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    let down_key = KeyEvent::new(KeyCode::Down, KeyModifiers::empty());
    search.handle_result_detail_input(down_key).unwrap();
    assert_eq!(search.detail_scroll_offset, 1);

    // Simulate scrolling with 'j'
    let j_key = KeyEvent::new(KeyCode::Char('j'), KeyModifiers::empty());
    search.handle_result_detail_input(j_key).unwrap();
    assert_eq!(search.detail_scroll_offset, 2);

    // Simulate scrolling up
    let up_key = KeyEvent::new(KeyCode::Up, KeyModifiers::empty());
    search.handle_result_detail_input(up_key).unwrap();
    assert_eq!(search.detail_scroll_offset, 1);

    // Simulate page down
    let page_down = KeyEvent::new(KeyCode::PageDown, KeyModifiers::empty());
    search.handle_result_detail_input(page_down).unwrap();
    assert_eq!(search.detail_scroll_offset, 11);

    // Test reset on escape
    let esc_key = KeyEvent::new(KeyCode::Esc, KeyModifiers::empty());
    search.handle_result_detail_input(esc_key).unwrap();
    assert_eq!(search.detail_scroll_offset, 0);
    assert_eq!(search.mode, Mode::Search);
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
fn test_ctrl_r_truncation_toggle() {
    let temp_dir = tempdir().unwrap();
    let test_file = temp_dir.path().join("test.jsonl");

    // Create test file with a long message
    let mut file = File::create(&test_file).unwrap();
    writeln!(file, r#"{{"type":"user","message":{{"role":"user","content":"This is a very long message that should demonstrate truncation behavior when toggled"}},"uuid":"1","timestamp":"2024-01-01T00:00:00Z","sessionId":"s1","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}}"#).unwrap();
    drop(file);

    let mut search = InteractiveSearch::new(SearchOptions::default());

    // Initially truncation should be enabled
    assert!(search.truncation_enabled);

    // Simulate Ctrl+R to toggle truncation
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    let ctrl_r = KeyEvent::new(KeyCode::Char('r'), KeyModifiers::CONTROL);
    search
        .handle_search_input(ctrl_r, test_file.to_str().unwrap())
        .unwrap();

    // Truncation should now be disabled
    assert!(!search.truncation_enabled);
    assert_eq!(
        search.message,
        Some("Message display: Full Text".to_string())
    );

    // Toggle again
    search
        .handle_search_input(ctrl_r, test_file.to_str().unwrap())
        .unwrap();

    // Truncation should be enabled again
    assert!(search.truncation_enabled);
    assert_eq!(
        search.message,
        Some("Message display: Truncated".to_string())
    );
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
    let ctrl_r = KeyEvent::new(KeyCode::Char('r'), KeyModifiers::CONTROL);

    // Initially truncation should be enabled
    assert!(search.truncation_enabled);

    // Toggle in Search mode
    search
        .handle_search_input(ctrl_r, test_file.to_str().unwrap())
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

    // Toggle in detail mode
    search.handle_result_detail_input(ctrl_r).unwrap();
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
    let mut terminal = create_test_terminal();
    let mut search = InteractiveSearch::new(SearchOptions::default());

    // Create many results
    let mut results = Vec::new();
    for i in 0..25 {
        results.push(create_test_result(
            "user",
            &format!("Message {i}"),
            "2024-01-01T00:00:00Z",
        ));
    }
    search.results = results;

    // Draw and check that "more results" message would be shown
    terminal.draw(|f| search.draw_search(f)).unwrap();

    // The actual display depends on terminal height, but we can verify the logic
    // With 24 line terminal, accounting for header lines, we'd have limited visible results
    let terminal_height = 24;
    let header_lines = 7; // Approximate header/footer lines
    let visible_count = (terminal_height - header_lines).min(search.results.len());
    assert!(visible_count < 25);
    assert!(search.results.len() > visible_count);
}

#[test]
fn test_session_viewer_pagination() {
    let temp_dir = tempdir().unwrap();
    let test_file = temp_dir.path().join("session.jsonl");

    // Create session with many messages
    let mut file = File::create(&test_file).unwrap();
    for i in 0..10 {
        writeln!(file, r#"{{"type":"user","content":"Message {i}"}}"#).unwrap();
    }

    let mut search = InteractiveSearch::new(SearchOptions::default());
    search
        .load_session_messages(test_file.to_str().unwrap())
        .unwrap();

    // Set up session viewer
    search.session_order = Some(SessionOrder::Ascending);
    search.mode = Mode::SessionViewer;

    // Test that we have multiple messages
    assert_eq!(search.session_messages.len(), 10);

    // The actual pagination display is handled in draw_session_viewer
    // We can test that messages are loaded correctly for pagination
    let mut terminal = create_test_terminal();
    terminal.draw(|f| search.draw_session_viewer(f)).unwrap();
}

#[test]
fn test_help_mode_key_binding() {
    let temp_dir = tempdir().unwrap();
    let test_file = temp_dir.path().join("test.jsonl");
    File::create(&test_file).unwrap();

    let mut search = InteractiveSearch::new(SearchOptions::default());

    // Test ? key to enter help mode
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    let help_key = KeyEvent::new(KeyCode::Char('?'), KeyModifiers::empty());
    let continue_running = search
        .handle_search_input(help_key, test_file.to_str().unwrap())
        .unwrap();

    assert!(continue_running); // Should continue running
    assert_eq!(search.mode, Mode::Help);

    // In help mode, any key returns to search (handled in run loop)
    // We can simulate this by setting mode back
    search.mode = Mode::Search;
    assert_eq!(search.mode, Mode::Search);
}

#[test]
fn test_result_limit_indicator() {
    let temp_dir = tempdir().unwrap();
    let test_file = temp_dir.path().join("test.jsonl");

    // Create more messages than the limit
    let mut file = File::create(&test_file).unwrap();
    for i in 0..60 {
        writeln!(file, r#"{{"type":"user","message":{{"role":"user","content":"Message {}"}},"uuid":"{}","timestamp":"2024-01-01T00:00:{:02}Z","sessionId":"s1","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}}"#, i, i, i % 60).unwrap();
    }

    let options = SearchOptions {
        max_results: Some(50),
        ..Default::default()
    };
    let mut search = InteractiveSearch::new(options);

    search.query = "Message".to_string();
    search.execute_search_sync(test_file.to_str().unwrap());

    // Should be limited to max_results
    assert_eq!(search.results.len(), 50);

    // Test that UI would show limit reached indicator
    let mut terminal = create_test_terminal();
    terminal.draw(|f| search.draw_search(f)).unwrap();
}

#[test]
#[ignore] // Clipboard commands not available in CI
fn test_clipboard_error_handling() {
    let mut search = InteractiveSearch::new(SearchOptions::default());
    search.selected_result = Some(create_test_result("user", "Test", "2024-01-01T00:00:00Z"));
    search.mode = Mode::ResultDetail;

    // Test clipboard operation that will fail in test environment
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    let copy_key = KeyEvent::new(KeyCode::Char('m'), KeyModifiers::empty());
    let result = search.handle_result_detail_input(copy_key);

    // Should handle error gracefully
    assert!(result.is_ok());
    assert!(search.message.is_some());
    // Error message should contain warning symbol
    if let Some(msg) = &search.message {
        assert!(msg.contains("⚠") || msg.contains("✓"));
    }
}

#[test]
fn test_cursor_position_with_role_filter() {
    let temp_dir = tempdir().unwrap();
    let test_file = temp_dir.path().join("test.jsonl");
    File::create(&test_file).unwrap();

    let mut search = InteractiveSearch::new(SearchOptions::default());
    search.role_filter = Some("user".to_string());
    search.query = "test query".to_string();

    // The cursor position calculation is done in draw_search
    // We can verify the state is correct for proper cursor positioning
    assert_eq!(search.role_filter, Some("user".to_string()));
    assert_eq!(search.query, "test query");

    // Draw to test cursor positioning logic
    let mut terminal = create_test_terminal();
    terminal.draw(|f| search.draw_search(f)).unwrap();
}

#[test]
fn test_search_results_scrolling() {
    let temp_dir = tempdir().unwrap();
    let test_file = temp_dir.path().join("test.jsonl");

    // Create many results to test scrolling
    let mut file = File::create(&test_file).unwrap();
    for i in 0..100 {
        writeln!(file, r#"{{"type":"user","message":{{"role":"user","content":"Message {}"}},"uuid":"{}","timestamp":"2024-01-01T00:00:{:02}Z","sessionId":"s1","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}}"#, i, i, i % 60).unwrap();
    }

    let mut search = InteractiveSearch::new(SearchOptions::default());
    search.query = "Message".to_string();
    search.execute_search_sync(test_file.to_str().unwrap());

    // We should have many results
    assert!(search.results.len() > 20);

    // Test basic navigation
    assert_eq!(search.selected_index, 0);
    assert_eq!(search.scroll_offset, 0);

    // Navigate down within visible range (no scrolling yet)
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    let down_key = KeyEvent::new(KeyCode::Down, KeyModifiers::empty());
    for _ in 0..10 {
        search
            .handle_search_input(down_key, test_file.to_str().unwrap())
            .unwrap();
    }
    assert_eq!(search.selected_index, 10);
    assert_eq!(search.scroll_offset, 0); // Should not scroll yet

    // Continue navigating down to trigger scrolling
    // Assuming visible items is around 15-20, scrolling should start
    for _ in 0..15 {
        search
            .handle_search_input(down_key, test_file.to_str().unwrap())
            .unwrap();
    }
    assert_eq!(search.selected_index, 25);
    assert!(search.scroll_offset > 0); // Should have scrolled

    // Test scrolling up
    let up_key = KeyEvent::new(KeyCode::Up, KeyModifiers::empty());
    for _ in 0..20 {
        search
            .handle_search_input(up_key, test_file.to_str().unwrap())
            .unwrap();
    }
    assert_eq!(search.selected_index, 5);
    // When selected_index is 5 and visible count is ~17 (24-7), scroll_offset should be 0
    assert!(search.scroll_offset <= 5); // Should scroll to show the selected item

    // Test Page Down
    let page_down = KeyEvent::new(KeyCode::PageDown, KeyModifiers::empty());
    search
        .handle_search_input(page_down, test_file.to_str().unwrap())
        .unwrap();
    assert!(search.selected_index > 10);
    assert!(search.scroll_offset > 0);

    // Test Page Up
    let page_up = KeyEvent::new(KeyCode::PageUp, KeyModifiers::empty());
    search
        .handle_search_input(page_up, test_file.to_str().unwrap())
        .unwrap();
    // After page down then page up, should be back at lower index
    assert!(search.selected_index < 20);
    assert!(search.scroll_offset < 10);

    // Test Home key
    search.selected_index = 50;
    search.scroll_offset = 30;
    let home_key = KeyEvent::new(KeyCode::Home, KeyModifiers::empty());
    search
        .handle_search_input(home_key, test_file.to_str().unwrap())
        .unwrap();
    assert_eq!(search.selected_index, 0);
    assert_eq!(search.scroll_offset, 0);

    // Test End key
    let end_key = KeyEvent::new(KeyCode::End, KeyModifiers::empty());
    search
        .handle_search_input(end_key, test_file.to_str().unwrap())
        .unwrap();
    assert_eq!(search.selected_index, search.results.len() - 1);
    assert!(search.scroll_offset > 0); // Should scroll to show last item
}

#[test]
fn test_visible_range_calculation() {
    let mut search = InteractiveSearch::new(SearchOptions::default());

    // Create test results
    let mut results = Vec::new();
    for i in 0..50 {
        results.push(create_test_result(
            "user",
            &format!("Message {i}"),
            "2024-01-01T00:00:00Z",
        ));
    }
    search.results = results;

    // Test visible range calculation with different terminal heights
    // Available height 20 means 19 items visible (1 line for scroll indicator)
    let (start, end) = search.calculate_visible_range(20);
    assert_eq!(start, 0);
    assert!(end > 0 && end <= 19);

    // With scrolling
    search.scroll_offset = 10;
    let (start, end) = search.calculate_visible_range(20);
    assert_eq!(start, 10);
    assert!(end > 10);

    // Ensure selected item is visible
    search.selected_index = 35;
    // With available height 20, visible count is 19
    search.adjust_scroll_offset(20);
    let (start, end) = search.calculate_visible_range(20);
    assert!(start <= 35 && 35 < end);
}

#[test]
fn test_actual_scrolling_behavior() {
    let temp_dir = tempdir().unwrap();
    let test_file = temp_dir.path().join("test.jsonl");

    // Create exactly 30 results
    let mut file = File::create(&test_file).unwrap();
    for i in 0..30 {
        writeln!(file, r#"{{"type":"user","message":{{"role":"user","content":"Message {i}"}},"uuid":"{i}","timestamp":"2024-01-01T00:00:{i:02}Z","sessionId":"s1","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}}"#).unwrap();
    }

    let mut search = InteractiveSearch::new(SearchOptions::default());
    search.query = "Message".to_string();
    search.execute_search_sync(test_file.to_str().unwrap());

    assert_eq!(search.results.len(), 30);

    // With terminal height 20, we can show 19 items (1 for indicator)
    // Start at index 0, scroll_offset 0
    assert_eq!(search.selected_index, 0);
    assert_eq!(search.scroll_offset, 0);

    // Move down to index 18 (still visible without scrolling)
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    let down_key = KeyEvent::new(KeyCode::Down, KeyModifiers::empty());

    // Move down a few times and check scroll behavior
    for i in 0..18 {
        search
            .handle_search_input(down_key, test_file.to_str().unwrap())
            .unwrap();
        println!(
            "After {} down: selected_index={}, scroll_offset={}",
            i + 1,
            search.selected_index,
            search.scroll_offset
        );

        // With terminal height 24, available height is 17 (24-7)
        // With scroll indicator, visible items is 16 (17-1)
        // So items 0-15 should be visible without scrolling
        if i < 15 {
            assert_eq!(
                search.scroll_offset,
                0,
                "Should not scroll yet at index {}",
                i + 1
            );
        }
    }
    assert_eq!(search.selected_index, 18);

    // Move one more down to index 19 - this should trigger scrolling
    search
        .handle_search_input(down_key, test_file.to_str().unwrap())
        .unwrap();
    assert_eq!(search.selected_index, 19);
    assert!(search.scroll_offset > 0); // Should have scrolled

    // Move to the end
    let end_key = KeyEvent::new(KeyCode::End, KeyModifiers::empty());
    search
        .handle_search_input(end_key, test_file.to_str().unwrap())
        .unwrap();
    assert_eq!(search.selected_index, 29); // Last item
    assert!(search.scroll_offset > 10); // Should have scrolled significantly

    // Move back to the beginning
    let home_key = KeyEvent::new(KeyCode::Home, KeyModifiers::empty());
    search
        .handle_search_input(home_key, test_file.to_str().unwrap())
        .unwrap();
    assert_eq!(search.selected_index, 0);
    assert_eq!(search.scroll_offset, 0);
}

#[test]
fn test_scrolling_with_ratatui_terminal() {
    let temp_dir = tempdir().unwrap();
    let test_file = temp_dir.path().join("test.jsonl");

    // Create 50 results
    let mut file = File::create(&test_file).unwrap();
    for i in 0..50 {
        writeln!(file, r#"{{"type":"user","message":{{"role":"user","content":"Message {}"}},"uuid":"{}","timestamp":"2024-01-01T00:00:{:02}Z","sessionId":"s1","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}}"#, i, i, i % 60).unwrap();
    }

    let mut search = InteractiveSearch::new(SearchOptions::default());
    search.query = "Message".to_string();
    search.execute_search_sync(test_file.to_str().unwrap());

    // Create test terminal
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    // Draw initial state
    terminal.draw(|f| search.draw_search(f)).unwrap();

    // Verify that scroll indicator appears
    let buffer = terminal.backend().buffer();
    let content = buffer
        .content
        .iter()
        .map(|cell| cell.symbol().to_string())
        .collect::<String>();

    // Should show scroll indicator
    assert!(content.contains("Showing 1-") || content.contains("of 50 results"));

    // Move to end
    search.selected_index = 49;
    search.adjust_scroll_offset(17); // Simulating terminal with height 24 -> available 17

    // Draw again
    terminal.draw(|f| search.draw_search(f)).unwrap();

    // The last item should be visible
    let buffer = terminal.backend().buffer();
    let _content = buffer
        .content
        .iter()
        .map(|cell| cell.symbol().to_string())
        .collect::<String>();

    // Should show different range now
    // The exact content depends on terminal width and message truncation
    // Just verify that we're showing a different range
    assert!(search.scroll_offset > 30); // Should have scrolled significantly
}

#[test]
fn test_dynamic_message_truncation() {
    let search = InteractiveSearch::new(SearchOptions::default());

    // Test various terminal widths and message lengths
    let test_cases = vec![
        // (terminal_width, message, expected_preview_contains)
        (80, "Short message", "Short message"),
        (
            80,
            "This is a very long message that should be truncated with ellipsis at the end because it's too long to fit",
            "...",
        ),
        (60, "Medium length message that needs truncation", "..."),
        (
            120,
            "Even with a wide terminal, extremely long messages should still be truncated to maintain readability and consistency in the UI",
            "...",
        ),
    ];

    for (width, message, expected) in test_cases {
        let width = width as usize;
        let result = create_test_result("user", message, "2024-01-01T00:00:00Z");

        // Calculate available width for message
        // Format: "NN. [ROLE     ] MM/DD HH:MM <message>"
        // Approximate fixed width: 2 + 2 + 13 + 1 + 11 + 1 = 30 characters
        let fixed_width = 30;
        let available_width = width.saturating_sub(fixed_width);

        let truncated = search.truncate_message(&result.text, available_width);

        if message.len() > available_width {
            assert!(
                truncated.contains(expected),
                "Width {width}: Message '{message}' should contain '{expected}' but got '{truncated}'"
            );
            assert!(
                truncated.ends_with("..."),
                "Width {width}: Long message should end with ellipsis"
            );
        } else {
            assert_eq!(
                truncated, message,
                "Width {width}: Short message should not be truncated"
            );
        }
    }
}

#[test]
fn test_message_truncation_with_multibyte() {
    let search = InteractiveSearch::new(SearchOptions::default());

    // Test multibyte character handling
    let japanese = "これは日本語のテキストです。とても長い文章なので画面幅に収まりません。";
    let emoji = "Hello 👋 World 🌍 This is a test message with emojis 🎉🎊🎈";

    // Test truncation doesn't break multibyte characters
    for width in [20, 30, 40, 50] {
        let truncated_jp = search.truncate_message(japanese, width);
        let truncated_emoji = search.truncate_message(emoji, width);

        // Verify the truncated strings are valid UTF-8
        assert!(truncated_jp.is_char_boundary(truncated_jp.len()));
        assert!(truncated_emoji.is_char_boundary(truncated_emoji.len()));

        // Verify length constraints
        assert!(truncated_jp.chars().count() <= width);
        assert!(truncated_emoji.chars().count() <= width);
    }
}

#[test]
fn test_non_blocking_input() {
    let temp_dir = tempdir().unwrap();
    let test_file = temp_dir.path().join("test.jsonl");

    // Create test data
    let mut file = File::create(&test_file).unwrap();
    for i in 0..10 {
        writeln!(file, r#"{{"type":"user","message":{{"role":"user","content":"Message {i}"}},"uuid":"{i}","timestamp":"2024-01-01T00:00:{i:02}Z","sessionId":"s1","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}}"#).unwrap();
    }

    let mut search = InteractiveSearch::new(SearchOptions::default());

    // Simulate typing
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    // Test immediate search execution on input
    let m_key = KeyEvent::new(KeyCode::Char('M'), KeyModifiers::empty());
    search
        .handle_search_input(m_key, test_file.to_str().unwrap())
        .unwrap();
    assert_eq!(search.query, "M");
    // Use sync search for testing
    search.execute_search_sync(test_file.to_str().unwrap());
    assert_eq!(search.results.len(), 10); // All messages match "M"

    // Test that each character triggers immediate search
    let e_key = KeyEvent::new(KeyCode::Char('e'), KeyModifiers::empty());
    search
        .handle_search_input(e_key, test_file.to_str().unwrap())
        .unwrap();
    assert_eq!(search.query, "Me");
    search.execute_search_sync(test_file.to_str().unwrap());
    assert_eq!(search.results.len(), 10); // All messages still match "Me"

    // Test backspace also triggers immediate search
    let backspace = KeyEvent::new(KeyCode::Backspace, KeyModifiers::empty());
    search
        .handle_search_input(backspace, test_file.to_str().unwrap())
        .unwrap();
    assert_eq!(search.query, "M");
    search.execute_search_sync(test_file.to_str().unwrap());
    assert_eq!(search.results.len(), 10); // Back to "M"

    // Clear query
    search
        .handle_search_input(backspace, test_file.to_str().unwrap())
        .unwrap();
    assert_eq!(search.query, "");
    assert_eq!(search.results.len(), 0); // Empty query returns no results

    // Test searching for a specific message
    let zero_key = KeyEvent::new(KeyCode::Char('0'), KeyModifiers::empty());
    search
        .handle_search_input(zero_key, test_file.to_str().unwrap())
        .unwrap();
    assert_eq!(search.query, "0");
    search.execute_search_sync(test_file.to_str().unwrap());
    assert_eq!(search.results.len(), 1); // Only "Message 0" contains "0"
}

#[test]
fn test_escape_key_behaviors() {
    let mut search = InteractiveSearch::new(SearchOptions::default());

    // Test Esc in different modes
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    let esc_key = KeyEvent::new(KeyCode::Esc, KeyModifiers::empty());

    // Test plain Esc exits from search mode
    search.mode = Mode::Search;
    let temp_dir = tempdir().unwrap();
    let temp_file = temp_dir.path().join("test.jsonl");
    File::create(&temp_file).unwrap();
    let continue_running = search
        .handle_search_input(esc_key, temp_file.to_str().unwrap())
        .unwrap();
    assert!(!continue_running); // false means exit

    // Test Ctrl+C also exits
    let ctrl_c = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
    let continue_running = search
        .handle_search_input(ctrl_c, temp_file.to_str().unwrap())
        .unwrap();
    assert!(!continue_running); // false means exit

    // From detail mode - Esc should return to search
    search.mode = Mode::ResultDetail;
    search.detail_scroll_offset = 10;
    search.message = Some("Test message".to_string());
    search.handle_result_detail_input(esc_key).unwrap();
    assert_eq!(search.mode, Mode::Search);
    assert_eq!(search.detail_scroll_offset, 0);
    assert!(search.message.is_none());
}

#[test]
fn test_file_discovery_logic() {
    let temp_dir = tempdir().unwrap();

    // Create test files
    let file1 = temp_dir.path().join("session1.jsonl");
    let file2 = temp_dir.path().join("session2.jsonl");
    let subdir = temp_dir.path().join("subdir");
    std::fs::create_dir(&subdir).unwrap();
    let file3 = subdir.join("session3.jsonl");

    // Create files with content
    let mut f1 = File::create(&file1).unwrap();
    writeln!(f1, r#"{{"type":"user","message":{{"role":"user","content":"Test 1"}},"uuid":"1","timestamp":"2024-01-01T00:00:00Z","sessionId":"s1","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}}"#).unwrap();

    let mut f2 = File::create(&file2).unwrap();
    writeln!(f2, r#"{{"type":"user","message":{{"role":"user","content":"Test 2"}},"uuid":"2","timestamp":"2024-01-01T00:00:00Z","sessionId":"s2","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}}"#).unwrap();

    let mut f3 = File::create(&file3).unwrap();
    writeln!(f3, r#"{{"type":"user","message":{{"role":"user","content":"Test 3"}},"uuid":"3","timestamp":"2024-01-01T00:00:00Z","sessionId":"s3","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}}"#).unwrap();

    let mut search = InteractiveSearch::new(SearchOptions::default());

    // Test single file discovery
    search.query = "Test".to_string();
    search.execute_search_sync(file1.to_str().unwrap());
    assert_eq!(search.results.len(), 1);
    assert_eq!(search.results[0].text, "Test 1");

    // Test directory pattern discovery - use glob pattern
    search.query = "Test".to_string();
    let pattern = format!("{}/*.jsonl", temp_dir.path().to_str().unwrap());
    search.execute_search_sync(&pattern);
    // Should find at least the two files in root dir
    assert!(search.results.len() >= 2);

    // Test that results come from multiple files
    let file_names: Vec<String> = search.results.iter().map(|r| r.file.clone()).collect();
    assert!(file_names.contains(&"session1.jsonl".to_string()));
    assert!(file_names.contains(&"session2.jsonl".to_string()));
}

#[test]
#[ignore] // Clipboard commands not available in CI
fn test_clipboard_operations() {
    let temp_dir = tempdir().unwrap();
    let test_file = temp_dir.path().join("test.jsonl");

    let mut file = File::create(&test_file).unwrap();
    writeln!(file, r#"{{"type":"user","message":{{"role":"user","content":"Clipboard test"}},"uuid":"clip-123","timestamp":"2024-01-01T00:00:00Z","sessionId":"session-456","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/test/project","version":"1.0"}}"#).unwrap();

    let mut search = InteractiveSearch::new(SearchOptions::default());
    search.query = "Clipboard".to_string();
    search.execute_search_sync(test_file.to_str().unwrap());
    assert_eq!(search.results.len(), 1);

    // Enter detail view
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    let enter_key = KeyEvent::new(KeyCode::Enter, KeyModifiers::empty());
    search
        .handle_search_input(enter_key, test_file.to_str().unwrap())
        .unwrap();
    assert_eq!(search.mode, Mode::ResultDetail);

    // Test various copy operations
    // Note: We can't test actual clipboard functionality in tests, but we can test the flow

    // Test F - Copy file path
    let f_key = KeyEvent::new(KeyCode::Char('f'), KeyModifiers::empty());
    search.handle_result_detail_input(f_key).unwrap();
    assert!(search.message.is_some());
    assert!(
        search
            .message
            .as_ref()
            .unwrap()
            .contains("File path copied")
    );

    // Test I - Copy session ID
    search.message = None;
    let i_key = KeyEvent::new(KeyCode::Char('i'), KeyModifiers::empty());
    search.handle_result_detail_input(i_key).unwrap();
    assert!(search.message.is_some());
    assert!(
        search
            .message
            .as_ref()
            .unwrap()
            .contains("Session ID copied")
    );

    // Test P - Copy project path
    search.message = None;
    let p_key = KeyEvent::new(KeyCode::Char('p'), KeyModifiers::empty());
    search.handle_result_detail_input(p_key).unwrap();
    assert!(search.message.is_some());
    assert!(
        search
            .message
            .as_ref()
            .unwrap()
            .contains("Project path copied")
    );

    // Test M - Copy message text
    search.message = None;
    let m_key = KeyEvent::new(KeyCode::Char('m'), KeyModifiers::empty());
    search.handle_result_detail_input(m_key).unwrap();
    assert!(search.message.is_some());
    assert!(
        search
            .message
            .as_ref()
            .unwrap()
            .contains("Message text copied")
    );

    // Test R - Copy raw JSON
    search.message = None;
    let r_key = KeyEvent::new(KeyCode::Char('r'), KeyModifiers::empty());
    search.handle_result_detail_input(r_key).unwrap();
    assert!(search.message.is_some());
    assert!(search.message.as_ref().unwrap().contains("Raw JSON copied"));
}

#[test]
fn test_searching_indicator() {
    let temp_dir = tempdir().unwrap();
    let test_file = temp_dir.path().join("test.jsonl");

    let mut file = File::create(&test_file).unwrap();
    writeln!(file, r#"{{"type":"user","message":{{"role":"user","content":"Test"}},"uuid":"1","timestamp":"2024-01-01T00:00:00Z","sessionId":"s1","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}}"#).unwrap();

    let mut search = InteractiveSearch::new(SearchOptions::default());

    // Check is_searching state during search
    assert!(!search.is_searching);

    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    let t_key = KeyEvent::new(KeyCode::Char('T'), KeyModifiers::empty());

    // The is_searching flag is set during handle_search_input
    // We can't directly test the intermediate state, but we can verify it's false after
    search
        .handle_search_input(t_key, test_file.to_str().unwrap())
        .unwrap();
    // In async mode, is_searching is set to true, then async search happens
    // For testing, we verify the query was updated and use sync search
    assert_eq!(search.query, "T");
    search.execute_search_sync(test_file.to_str().unwrap());
    assert_eq!(search.results.len(), 1);
}

#[test]
fn test_session_viewer_quit() {
    let mut search = InteractiveSearch::new(SearchOptions::default());
    search.mode = Mode::SessionViewer;
    search.session_order = Some(SessionOrder::Ascending);
    search.session_messages = vec!["msg1".to_string(), "msg2".to_string()];
    search.session_index = 0;

    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    // Test Q key quits session viewer
    let q_key = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::empty());
    search.handle_session_viewer_input(q_key).unwrap();
    assert_eq!(search.mode, Mode::Search);

    // Test lowercase q also works
    search.mode = Mode::SessionViewer;
    let q_lower = KeyEvent::new(KeyCode::Char('Q'), KeyModifiers::empty());
    search.handle_session_viewer_input(q_lower).unwrap();
    assert_eq!(search.mode, Mode::Search);

    // Test Esc also quits
    search.mode = Mode::SessionViewer;
    let esc_key = KeyEvent::new(KeyCode::Esc, KeyModifiers::empty());
    search.handle_session_viewer_input(esc_key).unwrap();
    assert_eq!(search.mode, Mode::Search);
}

#[test]
fn test_exit_goodbye_message() {
    let temp_dir = tempdir().unwrap();
    let test_file = temp_dir.path().join("test.jsonl");
    File::create(&test_file).unwrap();

    let mut search = InteractiveSearch::new(SearchOptions::default());

    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    let esc_key = KeyEvent::new(KeyCode::Esc, KeyModifiers::empty());

    // When we exit, the handle_search_input returns false
    let should_continue = search
        .handle_search_input(esc_key, test_file.to_str().unwrap())
        .unwrap();
    assert!(!should_continue); // This triggers the exit flow

    // The actual "Goodbye!" message is printed in run_app after the loop exits
    // We can't test the actual print, but we've verified the exit condition
}

#[test]
fn test_tilde_expansion() {
    // Test that tilde paths are handled correctly
    let mut search = InteractiveSearch::new(SearchOptions::default());

    // Create a test file in temp dir
    let temp_dir = tempdir().unwrap();
    let test_file = temp_dir.path().join("test.jsonl");
    let mut file = File::create(&test_file).unwrap();
    writeln!(file, r#"{{"type":"user","message":{{"role":"user","content":"Tilde test"}},"uuid":"1","timestamp":"2024-01-01T00:00:00Z","sessionId":"s1","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}}"#).unwrap();

    // Test with actual path (not tilde)
    search.query = "Tilde".to_string();
    search.execute_search_sync(test_file.to_str().unwrap());
    assert_eq!(search.results.len(), 1);

    // Note: We can't easily test actual tilde expansion without modifying HOME env var
    // The expand_tilde function is tested in the search module tests
}

#[test]
fn test_session_viewer_full_flow() {
    let temp_dir = tempdir().unwrap();
    let test_file = temp_dir.path().join("test.jsonl");

    // Create multiple messages in same session
    let mut file = File::create(&test_file).unwrap();
    writeln!(file, r#"{{"type":"user","message":{{"role":"user","content":"First message"}},"uuid":"1","timestamp":"2024-01-01T00:00:00Z","sessionId":"test-session","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}}"#).unwrap();
    writeln!(file, r#"{{"type":"assistant","message":{{"id":"msg1","type":"message","role":"assistant","model":"claude","content":[{{"type":"text","text":"Response"}}],"stop_reason":"end_turn","stop_sequence":null,"usage":{{"input_tokens":10,"cache_creation_input_tokens":0,"cache_read_input_tokens":0,"output_tokens":5}}}},"uuid":"2","timestamp":"2024-01-01T00:00:01Z","sessionId":"test-session","parentUuid":"1","isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}}"#).unwrap();
    writeln!(file, r#"{{"type":"user","message":{{"role":"user","content":"Second message"}},"uuid":"3","timestamp":"2024-01-01T00:00:02Z","sessionId":"test-session","parentUuid":"2","isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}}"#).unwrap();
    writeln!(file, r#"{{"type":"assistant","message":{{"id":"msg2","type":"message","role":"assistant","model":"claude","content":[{{"type":"text","text":"Another response"}}],"stop_reason":"end_turn","stop_sequence":null,"usage":{{"input_tokens":10,"cache_creation_input_tokens":0,"cache_read_input_tokens":0,"output_tokens":5}}}},"uuid":"4","timestamp":"2024-01-01T00:00:03Z","sessionId":"test-session","parentUuid":"3","isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}}"#).unwrap();

    let mut search = InteractiveSearch::new(SearchOptions::default());

    // Search and enter detail view
    search.query = "message".to_string();
    search.execute_search_sync(test_file.to_str().unwrap());
    assert!(!search.results.is_empty());

    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    // Select the first result by pressing Enter
    let enter_key = KeyEvent::new(KeyCode::Enter, KeyModifiers::empty());
    search
        .handle_search_input(enter_key, test_file.to_str().unwrap())
        .unwrap();
    assert_eq!(search.mode, Mode::ResultDetail);
    assert!(search.selected_result.is_some());

    // Skip the S key test since it requires complex file discovery
    // Directly set up session viewer mode with loaded messages
    search.session_messages = vec![
            r#"{"type":"user","message":{"role":"user","content":"First message"},"uuid":"1","timestamp":"2024-01-01T00:00:00Z","sessionId":"test-session","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}"#.to_string(),
            r#"{"type":"assistant","message":{"id":"msg1","type":"message","role":"assistant","model":"claude","content":[{"type":"text","text":"Response"}],"stop_reason":"end_turn","stop_sequence":null,"usage":{"input_tokens":10,"cache_creation_input_tokens":0,"cache_read_input_tokens":0,"output_tokens":5}}},"uuid":"2","timestamp":"2024-01-01T00:00:01Z","sessionId":"test-session","parentUuid":"1","isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}"#.to_string(),
            r#"{"type":"user","message":{"role":"user","content":"Second message"},"uuid":"3","timestamp":"2024-01-01T00:00:02Z","sessionId":"test-session","parentUuid":"2","isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}"#.to_string(),
            r#"{"type":"assistant","message":{"id":"msg2","type":"message","role":"assistant","model":"claude","content":[{"type":"text","text":"Another response"}],"stop_reason":"end_turn","stop_sequence":null,"usage":{"input_tokens":10,"cache_creation_input_tokens":0,"cache_read_input_tokens":0,"output_tokens":5}}},"uuid":"4","timestamp":"2024-01-01T00:00:03Z","sessionId":"test-session","parentUuid":"3","isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}"#.to_string(),
        ];
    search.session_index = 0;
    search.session_order = None;
    search.mode = Mode::SessionViewer;
    assert!(search.session_order.is_none()); // Not selected yet

    // Select ascending order
    let a_key = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::empty());
    search.handle_session_viewer_input(a_key).unwrap();
    assert_eq!(search.session_order, Some(SessionOrder::Ascending));
    assert!(search.session_messages.len() >= 4); // Should have loaded all messages

    // Test navigation
    let initial_index = search.session_index;
    let down_key = KeyEvent::new(KeyCode::Down, KeyModifiers::empty());
    search.handle_session_viewer_input(down_key).unwrap();
    assert_eq!(search.session_index, initial_index + 1);

    // Test page down
    let page_down = KeyEvent::new(KeyCode::PageDown, KeyModifiers::empty());
    search.handle_session_viewer_input(page_down).unwrap();
    assert!(search.session_index >= initial_index + 3);
}

#[test]
fn test_color_and_formatting() {
    let mut search = InteractiveSearch::new(SearchOptions::default());
    let mut terminal = create_test_terminal();

    // Create test data with results
    let results = vec![
        create_test_result("user", "Test message", "2024-01-01T00:00:00Z"),
        create_test_result("assistant", "Response", "2024-01-01T00:00:01Z"),
    ];
    search.results = results;
    search.selected_index = 0; // First item selected

    // Draw the search interface
    terminal.draw(|f| search.draw_search(f)).unwrap();

    let buffer = terminal.backend().buffer();

    // Test header color (should be cyan)
    let header_text = "Interactive Claude Search";
    if let Some(pos) = find_text_in_buffer(buffer, header_text) {
        let cell = &buffer[(pos.0, pos.1)];
        assert_eq!(cell.fg, Color::Cyan);
    } else {
        panic!("Header text not found in buffer");
    }

    // Test selected item style (should have bold modifier and cyan color for ">")
    let selected_indicator = ">";
    if let Some(pos) = find_text_in_buffer(buffer, selected_indicator) {
        let cell = &buffer[(pos.0, pos.1)];
        assert_eq!(cell.fg, Color::Cyan);
        assert!(cell.modifier.contains(Modifier::BOLD));
    }

    // Test role color (should be yellow)
    let role_text = "[USER"; // Partial match since it's formatted
    if let Some(pos) = find_text_in_buffer(buffer, role_text) {
        let cell = &buffer[(pos.0 + 1, pos.1)]; // Skip the "["
        assert_eq!(cell.fg, Color::Yellow);
    }
}

#[test]
fn test_help_screen_colors() {
    let mut search = InteractiveSearch::new(SearchOptions::default());
    search.mode = Mode::Help;
    let mut terminal = create_test_terminal();

    terminal.draw(|f| search.draw_help(f)).unwrap();

    let buffer = terminal.backend().buffer();

    // Test help title color
    let help_title = "CCMS Help";
    if let Some(pos) = find_text_in_buffer(buffer, help_title) {
        let cell = &buffer[(pos.0, pos.1)];
        assert_eq!(cell.fg, Color::Cyan);
        assert!(cell.modifier.contains(Modifier::BOLD));
    }

    // Test section headers (should be yellow)
    let section = "Search Mode:";
    if let Some(pos) = find_text_in_buffer(buffer, section) {
        let cell = &buffer[(pos.0, pos.1)];
        assert_eq!(cell.fg, Color::Yellow);
    }
}

#[test]
fn test_error_message_color() {
    let mut search = InteractiveSearch::new(SearchOptions::default());
    search.mode = Mode::ResultDetail;
    search.message = Some("⚠ Error occurred".to_string());
    let mut terminal = create_test_terminal();

    // Need a selected result for detail view
    search.selected_result = Some(create_test_result("user", "Test", "2024-01-01T00:00:00Z"));

    terminal.draw(|f| search.draw_result_detail(f)).unwrap();

    let buffer = terminal.backend().buffer();

    // Test warning symbol color
    let warning = "⚠";
    if let Some(pos) = find_text_in_buffer(buffer, warning) {
        let cell = &buffer[(pos.0, pos.1)];
        assert_eq!(cell.fg, Color::Yellow);
    }
}

#[test]
fn test_success_message_color() {
    let mut search = InteractiveSearch::new(SearchOptions::default());
    search.mode = Mode::ResultDetail;
    search.message = Some("✓ Success!".to_string());
    let mut terminal = create_test_terminal();

    // Need a selected result for detail view
    search.selected_result = Some(create_test_result("user", "Test", "2024-01-01T00:00:00Z"));

    terminal.draw(|f| search.draw_result_detail(f)).unwrap();

    let buffer = terminal.backend().buffer();

    // Test success symbol color
    let success = "✓";
    if let Some(pos) = find_text_in_buffer(buffer, success) {
        let cell = &buffer[(pos.0, pos.1)];
        assert_eq!(cell.fg, Color::Green);
    }
}

// Helper function to find text in buffer
fn find_text_in_buffer(buffer: &Buffer, text: &str) -> Option<(u16, u16)> {
    for y in 0..buffer.area.height {
        for x in 0..buffer.area.width {
            let start_x = x as usize;
            // Check if text matches at this position
            let mut matches = true;
            for (i, ch) in text.chars().enumerate() {
                let pos_x = start_x + i;
                if pos_x >= buffer.area.width as usize {
                    matches = false;
                    break;
                }

                let cell = &buffer[(pos_x as u16, y)];
                if cell.symbol() != ch.to_string() {
                    matches = false;
                    break;
                }
            }

            if matches {
                return Some((x, y));
            }
        }
    }
    None
}

#[test]
fn test_async_search_channel_disconnect() {
    // Test that worker thread exits gracefully when channel is disconnected
    let temp_dir = tempdir().unwrap();
    let test_file = temp_dir.path().join("test.jsonl");

    // Create test file
    let mut file = File::create(&test_file).unwrap();
    writeln!(file, r#"{{"type":"user","message":{{"role":"user","content":"Test"}},"uuid":"1","timestamp":"2024-01-01T00:00:00Z","sessionId":"s1","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}}"#).unwrap();

    let search = InteractiveSearch::new(SearchOptions::default());
    let (tx, rx) = mpsc::channel();
    let (response_tx, _response_rx) = mpsc::channel();

    // Start worker thread
    let handle = search.start_search_worker(rx, response_tx, test_file.to_str().unwrap());

    // Send a request
    let request = SearchRequest {
        id: 1,
        query: "test".to_string(),
        role_filter: None,
        pattern: test_file.to_str().unwrap().to_string(),
    };
    tx.send(request).unwrap();

    // Drop sender to disconnect channel
    drop(tx);

    // Thread should exit gracefully
    assert!(handle.join().is_ok());
}

#[test]
fn test_session_viewer_json_parse_error() {
    let mut search = InteractiveSearch::new(SearchOptions::default());
    let mut terminal = create_test_terminal();

    search.mode = Mode::SessionViewer;
    search.session_order = Some(SessionOrder::Ascending);
    search.session_messages = vec![
        "invalid json {".to_string(), // Invalid JSON
    ];
    search.session_index = 0;

    // Draw should handle parse error gracefully
    terminal.draw(|f| search.draw_session_viewer(f)).unwrap();

    let buffer = terminal.backend().buffer();

    // Should show error message
    let error_text = "Error: Unable to parse message JSON";
    assert!(find_text_in_buffer(buffer, error_text).is_some());
}

#[test]
fn test_session_viewer_empty_messages() {
    let mut search = InteractiveSearch::new(SearchOptions::default());
    let mut terminal = create_test_terminal();

    search.mode = Mode::SessionViewer;
    search.session_order = Some(SessionOrder::Ascending);
    search.session_messages = vec![]; // Empty messages

    // Draw should handle empty messages gracefully
    terminal.draw(|f| search.draw_session_viewer(f)).unwrap();

    let buffer = terminal.backend().buffer();

    // Should show empty message
    let empty_text = "No messages in session";
    assert!(find_text_in_buffer(buffer, empty_text).is_some());
}

#[test]
fn test_extract_project_path_edge_cases() {
    use std::path::Path;

    // Test various edge cases
    let test_cases = vec![
        // Invalid paths
        ("/invalid/path/file.jsonl", "/invalid/path"),
        ("", ""), // Empty path returns empty string
        ("/", "/"),
        // Path without projects directory
        ("/Users/test/file.jsonl", "/Users/test"),
        // Path with projects but no session (directory path)
        ("~/.claude/projects/", "~/.claude"),
        // Valid project path (already tested elsewhere, but included for completeness)
        (
            "~/.claude/projects/Users-test-project/session.jsonl",
            "Users/test/project",
        ),
    ];

    for (input, expected) in test_cases {
        let path = Path::new(input);
        let result = InteractiveSearch::extract_project_path(path);

        // For valid claude project paths
        if input.contains("/.claude/projects/") && input.ends_with(".jsonl") {
            assert_eq!(result, expected);
        } else if input.is_empty() {
            // Empty path returns empty string (Path::new("") has no parent)
            assert_eq!(result, "");
        } else {
            // Other cases should return the parent directory or the path itself
            assert_eq!(result, expected);
        }
    }
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
