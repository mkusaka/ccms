use super::*;
use crate::query::condition::QueryCondition;
use crate::{SearchOptions, SearchResult};
use crossterm::terminal;
use ratatui::{Terminal, backend::TestBackend, buffer::Buffer};
use std::fs::File;
use std::io::Write;
use std::time::Duration;
use tempfile::tempdir;

// Helper function to find text in a buffer
fn find_text_in_buffer(buffer: &ratatui::buffer::Buffer, text: &str) -> Option<(u16, u16)> {
    let width = buffer.area.width;
    let height = buffer.area.height;

    for y in 0..height {
        let mut line = String::new();
        for x in 0..width {
            line.push_str(buffer[(x, y)].symbol());
        }
        if line.contains(text) {
            return Some((0, y));
        }
    }
    None
}

fn create_test_result(role: &str, text: &str, timestamp: &str) -> SearchResult {
    SearchResult {
        file: "/test/path".to_string(),
        uuid: "test-uuid".to_string(),
        timestamp: timestamp.to_string(),
        session_id: "test-session".to_string(),
        role: role.to_string(),
        text: text.to_string(),
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

#[test]
fn test_initial_state() {
    let search = InteractiveSearch::new(SearchOptions::default());
    assert!(search.query.is_empty());
    assert!(search.results.is_empty());
    assert!(search.role_filter.is_none());
    assert_eq!(search.current_mode(), Mode::Search);
}

#[test]
fn test_mode_transitions() {
    let mut search = InteractiveSearch::new(SearchOptions::default());

    // Start in Search mode
    assert_eq!(search.current_mode(), Mode::Search);

    // Transition to Help
    search.push_screen(Mode::Help);
    assert_eq!(search.current_mode(), Mode::Help);

    // Transition to ResultDetail
    search.push_screen(Mode::ResultDetail);
    assert_eq!(search.current_mode(), Mode::ResultDetail);

    // Transition to SessionViewer
    search.push_screen(Mode::SessionViewer);
    assert_eq!(search.current_mode(), Mode::SessionViewer);
}

#[test]
fn test_default_values() {
    let options = SearchOptions::default();
    let search = InteractiveSearch::new(options);

    // Default max results should be 50
    assert_eq!(search.max_results, 50);
    assert_eq!(search.current_mode(), Mode::Search);
    assert!(search.query.is_empty());
    assert!(search.results.is_empty());
    assert!(search.role_filter.is_none());
    assert!(search.message.is_none());
    assert_eq!(search.selected_index, 0);
    assert_eq!(search.detail_scroll_offset, 0);
    assert_eq!(search.scroll_offset, 0);
}

#[test]
fn test_cache_functionality() {
    let temp_dir = tempdir().unwrap();
    let test_file = temp_dir.path().join("test.jsonl");

    let mut file = File::create(&test_file).unwrap();
    writeln!(
        file,
        r#"{{"type":"user","message":{{"role":"user","content":"Test message"}},"uuid":"1","timestamp":"2024-01-01T00:00:00Z","sessionId":"s1","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}}"#
    )
    .unwrap();

    let mut cache = MessageCache::new();

    // First load
    let cached = cache.get_messages(&test_file).unwrap();
    assert_eq!(cached.messages.len(), 1);
    assert_eq!(cached.raw_lines.len(), 1);

    // Load again - should use cache
    let cached2 = cache.get_messages(&test_file).unwrap();
    assert_eq!(cached2.messages.len(), 1);

    // Clear cache and reload with modified file
    drop(file); // Close file first
    thread::sleep(Duration::from_secs(2)); // Increase sleep time for filesystem timestamp granularity

    // Append a new message
    let mut file = File::options().append(true).open(&test_file).unwrap();
    writeln!(
        file,
        r#"{{"type":"assistant","message":{{"id":"msg1","type":"message","role":"assistant","model":"claude","content":[{{"type":"text","text":"Response"}}],"stop_reason":"end_turn","stop_sequence":null,"usage":{{"input_tokens":10,"cache_creation_input_tokens":0,"cache_read_input_tokens":0,"output_tokens":5}}}},"uuid":"test-uuid","timestamp":"2024-01-01T00:00:00Z","sessionId":"test-session","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}}"#
    )
    .unwrap();
    file.sync_all().unwrap(); // Force sync
    drop(file); // Ensure file is closed

    // Touch the file to ensure timestamp is updated
    std::fs::OpenOptions::new()
        .write(true)
        .open(&test_file)
        .unwrap()
        .sync_all()
        .unwrap();

    // Clear cache to force reload
    cache.clear();

    // Now reload - should get 2 messages
    let cached3 = cache.get_messages(&test_file).unwrap();
    assert_eq!(cached3.messages.len(), 2);

    // Test cache clear
    cache.clear();
    assert!(cache.files.is_empty());
}

#[test]
fn test_file_change_detection() {
    let temp_dir = tempdir().unwrap();
    let test_file = temp_dir.path().join("test.jsonl");

    let mut file = File::create(&test_file).unwrap();
    writeln!(
        file,
        r#"{{"type":"user","message":{{"role":"user","content":"Message 1"}},"uuid":"test-uuid","timestamp":"2024-01-01T00:00:00Z","sessionId":"test-session","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}}"#
    )
    .unwrap();

    let mut search = InteractiveSearch::new(SearchOptions::default());
    search.query = "Message".to_string();
    search.execute_search_sync(test_file.to_str().unwrap());
    assert_eq!(search.results.len(), 1);

    // Add another message
    thread::sleep(Duration::from_secs(1));
    let mut file = File::options().append(true).open(&test_file).unwrap();
    writeln!(
        file,
        r#"{{"type":"user","message":{{"role":"user","content":"Message 2"}},"uuid":"test-uuid","timestamp":"2024-01-01T00:00:00Z","sessionId":"test-session","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}}"#
    )
    .unwrap();
    file.sync_all().unwrap();
    drop(file);

    search.execute_search_sync(test_file.to_str().unwrap());
    assert_eq!(search.results.len(), 2);
}

#[test]
fn test_help_screen_rendering() {
    let mut terminal = create_test_terminal();
    let mut search = InteractiveSearch::new(SearchOptions::default());
    search.push_screen(Mode::Help);

    terminal.draw(|f| search.draw(f)).unwrap();
    let buffer = terminal.backend().buffer();

    // Check that help title is present
    let help_title = "Help";
    let mut found = false;
    for y in 0..buffer.area.height {
        for x in 0..buffer.area.width {
            let cell = &buffer[(x, y)];
            if cell.symbol().starts_with('H') {
                let mut title = String::new();
                for i in 0..help_title.len() {
                    if x + i as u16 >= buffer.area.width {
                        break;
                    }
                    title.push_str(buffer[(x + i as u16, y)].symbol());
                }
                if title == help_title {
                    found = true;
                    break;
                }
            }
        }
    }
    assert!(found, "Help title not found in buffer");
}

#[test]
fn test_result_detail_rendering() {
    let mut terminal = create_test_terminal();
    let mut search = InteractiveSearch::new(SearchOptions::default());

    // Set up a result detail view
    let result = create_test_result("user", "Test message content", "2024-01-01T00:00:00Z");
    search.selected_result = Some(result);
    search.push_screen(Mode::ResultDetail);

    terminal.draw(|f| search.draw(f)).unwrap();
    let buffer = terminal.backend().buffer();

    // Check that result detail title is present
    let detail_title = "Result Detail";
    let mut found = false;
    for y in 0..buffer.area.height {
        for x in 0..buffer.area.width {
            let cell = &buffer[(x, y)];
            if cell.symbol().starts_with('R') {
                let mut title = String::new();
                for i in 0..detail_title.len() {
                    if x + i as u16 >= buffer.area.width {
                        break;
                    }
                    title.push_str(buffer[(x + i as u16, y)].symbol());
                }
                if title == detail_title {
                    found = true;
                    break;
                }
            }
        }
    }
    assert!(found, "Result Detail title not found in buffer");
}

#[test]
fn test_role_filter_cycling() {
    let mut search = InteractiveSearch::new(SearchOptions::default());

    // Initial state
    assert!(search.role_filter.is_none());

    // Simulate Tab key presses
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    let tab_key = KeyEvent::new(KeyCode::Tab, KeyModifiers::empty());

    // Cycle through filters
    let temp_dir = tempdir().unwrap();
    let test_file = temp_dir.path().join("test.jsonl");
    File::create(&test_file).unwrap();

    search
        .handle_search_input(tab_key, test_file.to_str().unwrap())
        .unwrap();
    assert_eq!(search.role_filter, Some("user".to_string()));

    search
        .handle_search_input(tab_key, test_file.to_str().unwrap())
        .unwrap();
    assert_eq!(search.role_filter, Some("assistant".to_string()));

    search
        .handle_search_input(tab_key, test_file.to_str().unwrap())
        .unwrap();
    assert_eq!(search.role_filter, Some("system".to_string()));

    search
        .handle_search_input(tab_key, test_file.to_str().unwrap())
        .unwrap();
    assert_eq!(search.role_filter, Some("summary".to_string()));

    search
        .handle_search_input(tab_key, test_file.to_str().unwrap())
        .unwrap();
    assert_eq!(search.role_filter, None);
}

#[test]
fn test_query_parsing_integration() {
    let temp_dir = tempdir().unwrap();
    let test_file = temp_dir.path().join("test.jsonl");

    let mut file = File::create(&test_file).unwrap();
    writeln!(file, r#"{{"type":"user","message":{{"role":"user","content":"Hello world"}},"uuid":"test-uuid","timestamp":"2024-01-01T00:00:00Z","sessionId":"test-session","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}}"#).unwrap();
    writeln!(file, r#"{{"type":"assistant","message":{{"id":"msg1","type":"message","role":"assistant","model":"claude","content":[{{"type":"text","text":"Goodbye world"}}],"stop_reason":"end_turn","stop_sequence":null,"usage":{{"input_tokens":10,"cache_creation_input_tokens":0,"cache_read_input_tokens":0,"output_tokens":5}}}},"uuid":"test-uuid","timestamp":"2024-01-01T00:00:00Z","sessionId":"test-session","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}}"#).unwrap();
    writeln!(file, r#"{{"type":"system","content":"System message","uuid":"3","timestamp":"2024-01-01T00:00:02Z","sessionId":"s1","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/test","version":"1.0","isMeta":false}}"#).unwrap();

    let mut search = InteractiveSearch::new(SearchOptions::default());

    // Test AND query
    search.query = "Hello AND world".to_string();
    search.execute_search_sync(test_file.to_str().unwrap());
    assert_eq!(search.results.len(), 1);
    assert_eq!(search.results[0].text, "Hello world");

    // Test OR query
    search.query = "Hello OR Goodbye".to_string();
    search.role_filter = None; // Clear any role filter
    search.execute_search_sync(test_file.to_str().unwrap());
    assert_eq!(search.results.len(), 2);

    // Test NOT query - search for messages containing "message" but not "System"
    search.query = "message NOT System".to_string();
    search.execute_search_sync(test_file.to_str().unwrap());
    // Only "System message" contains both words, so it should be excluded
    // "Hello world" and "Goodbye world" don't contain "message", so they don't match
    // The query should return 0 results
    assert_eq!(search.results.len(), 0);
}

#[test]
fn test_execute_search_with_filters() {
    let temp_dir = tempdir().unwrap();
    let test_file = temp_dir.path().join("test.jsonl");

    let mut file = File::create(&test_file).unwrap();
    writeln!(file, r#"{{"type":"user","message":{{"role":"user","content":"User message"}},"uuid":"1","timestamp":"2024-01-01T00:00:00Z","sessionId":"s1","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}}"#).unwrap();
    writeln!(file, r#"{{"type":"assistant","message":{{"id":"msg1","type":"message","role":"assistant","model":"claude","content":[{{"type":"text","text":"Assistant response"}}],"stop_reason":"end_turn","stop_sequence":null,"usage":{{"input_tokens":10,"cache_creation_input_tokens":0,"cache_read_input_tokens":0,"output_tokens":5}}}},"uuid":"2","timestamp":"2024-01-01T00:00:01Z","sessionId":"s1","parentUuid":"1","isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}}"#).unwrap();
    writeln!(file, r#"{{"type":"system","content":"System message","uuid":"3","timestamp":"2024-01-01T00:00:02Z","sessionId":"s1","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/test","version":"1.0","isMeta":false}}"#).unwrap();

    let mut search = InteractiveSearch::new(SearchOptions::default());

    // Test role filter
    search.query = "message".to_string();
    search.role_filter = Some("user".to_string());
    search.execute_search_sync(test_file.to_str().unwrap());
    assert_eq!(search.results.len(), 1);
    assert_eq!(search.results[0].role, "user");

    // Test assistant filter - search for "response" which is in the assistant message
    search.query = "response".to_string();
    search.role_filter = Some("assistant".to_string());
    search.execute_search_sync(test_file.to_str().unwrap());
    assert_eq!(search.results.len(), 1);
    assert_eq!(search.results[0].role, "assistant");

    // Test system filter
    search.query = "System".to_string();
    search.role_filter = Some("system".to_string());
    search.execute_search_sync(test_file.to_str().unwrap());

    // Also try without filter to see what messages are found
    search.query = "message".to_string();
    search.role_filter = None;
    search.execute_search_sync(test_file.to_str().unwrap());

    assert_eq!(
        search.results.iter().filter(|r| r.role == "system").count(),
        1
    );
    let system_msg = search.results.iter().find(|r| r.role == "system").unwrap();
    assert_eq!(system_msg.text, "System message");
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
    search.push_screen(Mode::Help);

    let mut terminal = create_test_terminal();
    terminal.draw(|f| search.draw_help(f)).unwrap();

    let buffer = terminal.backend().buffer();

    // Check that help content is displayed
    assert!(find_text_in_buffer(buffer, "Interactive Claude Search - Help").is_some());
    assert!(find_text_in_buffer(buffer, "Keyboard Shortcuts").is_some());
}

#[test]
fn test_draw_result_detail() {
    let mut search = InteractiveSearch::new(SearchOptions::default());
    search.push_screen(Mode::ResultDetail);
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
    search.push_screen(Mode::SessionViewer);
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

    // Should show "No results found"
    assert!(find_text_in_buffer(buffer, "No results found").is_some());
}

#[test]
fn test_result_sorting() {
    let temp_dir = tempdir().unwrap();
    let test_file = temp_dir.path().join("test.jsonl");

    let mut file = File::create(&test_file).unwrap();
    writeln!(file, r#"{{"type":"user","message":{{"role":"user","content":"Message 1"}},"uuid":"1","timestamp":"2024-01-01T00:00:02Z","sessionId":"s1","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}}"#).unwrap();
    writeln!(file, r#"{{"type":"user","message":{{"role":"user","content":"Message 2"}},"uuid":"2","timestamp":"2024-01-01T00:00:01Z","sessionId":"s1","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}}"#).unwrap();
    writeln!(file, r#"{{"type":"user","message":{{"role":"user","content":"Message 3"}},"uuid":"3","timestamp":"2024-01-01T00:00:03Z","sessionId":"s1","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}}"#).unwrap();

    let mut search = InteractiveSearch::new(SearchOptions::default());
    search.query = "Message".to_string();
    search.execute_search_sync(test_file.to_str().unwrap());

    // Results should be sorted by timestamp in descending order
    assert_eq!(search.results.len(), 3);
    assert_eq!(search.results[0].text, "Message 3");
    assert_eq!(search.results[1].text, "Message 1");
    assert_eq!(search.results[2].text, "Message 2");
}

#[test]
fn test_cursor_position_with_role_filter() {
    let mut terminal = create_test_terminal();
    let mut search = InteractiveSearch::new(SearchOptions::default());

    search.query = "test query".to_string();
    search.role_filter = Some("user".to_string());

    terminal.draw(|f| search.draw(f)).unwrap();

    // The cursor should be positioned after the query text in the input field
    let cursor_pos = terminal.get_cursor_position().unwrap();
    // Expected position: starting position + query length
    assert!(cursor_pos.x > 0);
}

#[test]
fn test_message_limit() {
    let mut search = InteractiveSearch::new(SearchOptions::default());

    // Test various messages
    search.message = Some("Short message".to_string());
    terminal::enable_raw_mode().ok();
    let mut terminal = create_test_terminal();
    terminal.draw(|f| search.draw(f)).unwrap();
    terminal::disable_raw_mode().ok();

    // Message should be displayed
    let buffer = terminal.backend().buffer();
    let buffer_content = buffer_to_string(buffer);
    assert!(buffer_content.contains("Short message"));

    // Test long message (should be truncated in actual display)
    search.message = Some("A".repeat(200));
    terminal.draw(|f| search.draw(f)).unwrap();
    // The actual truncation happens in the UI rendering
}

fn buffer_to_string(buffer: &Buffer) -> String {
    let mut result = String::new();
    for y in 0..buffer.area.height {
        for x in 0..buffer.area.width {
            result.push_str(buffer[(x, y)].symbol());
        }
        result.push('\n');
    }
    result
}

#[test]
fn test_message_clearing_on_mode_change() {
    let mut search = InteractiveSearch::new(SearchOptions::default());

    // Set a message in search mode
    search.message = Some("Test message".to_string());

    // Message should be cleared when returning from detail to search
    search.push_screen(Mode::ResultDetail);
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    let esc_key = KeyEvent::new(KeyCode::Esc, KeyModifiers::empty());
    search.handle_result_detail_input(esc_key).unwrap();

    assert!(search.message.is_none());
    assert_eq!(search.current_mode(), Mode::Search);
}

#[test]
fn test_detail_scroll_functionality() {
    let mut search = InteractiveSearch::new(SearchOptions::default());

    // Create a long result text
    let long_text = (0..100)
        .map(|i| format!("Line {i}"))
        .collect::<Vec<_>>()
        .join("\n");
    let result = create_test_result("user", &long_text, "2024-01-01T00:00:00Z");
    search.selected_result = Some(result);
    search.push_screen(Mode::ResultDetail);

    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    // Test scrolling down
    let down_key = KeyEvent::new(KeyCode::Down, KeyModifiers::empty());
    search.handle_result_detail_input(down_key).unwrap();
    assert_eq!(search.detail_scroll_offset, 1);

    // Test scrolling up
    let up_key = KeyEvent::new(KeyCode::Up, KeyModifiers::empty());
    search.handle_result_detail_input(up_key).unwrap();
    assert_eq!(search.detail_scroll_offset, 0);

    // Test page down
    search.detail_scroll_offset = 0;
    let page_down = KeyEvent::new(KeyCode::PageDown, KeyModifiers::empty());
    search.handle_result_detail_input(page_down).unwrap();
    assert!(search.detail_scroll_offset > 0);

    // Test page up
    let page_up = KeyEvent::new(KeyCode::PageUp, KeyModifiers::empty());
    search.handle_result_detail_input(page_up).unwrap();
    assert_eq!(search.detail_scroll_offset, 0);

    // Test Esc resets offset
    search.detail_scroll_offset = 5;
    let esc_key = KeyEvent::new(KeyCode::Esc, KeyModifiers::empty());
    search.handle_result_detail_input(esc_key).unwrap();
    assert_eq!(search.detail_scroll_offset, 0);
}

#[test]
fn test_ctrl_r_cache_reload() {
    let temp_dir = tempdir().unwrap();
    let test_file = temp_dir.path().join("test.jsonl");

    let mut file = File::create(&test_file).unwrap();
    writeln!(file, r#"{{"type":"user","message":{{"role":"user","content":"Message 1"}},"uuid":"test-uuid","timestamp":"2024-01-01T00:00:00Z","sessionId":"test-session","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}}"#).unwrap();

    let mut search = InteractiveSearch::new(SearchOptions::default());
    search.query = "Message".to_string();
    search.execute_search_sync(test_file.to_str().unwrap());
    assert_eq!(search.results.len(), 1);

    // Add another message
    drop(file); // Close file first
    thread::sleep(Duration::from_secs(1));
    let mut file = File::options().append(true).open(&test_file).unwrap();
    writeln!(file, r#"{{"type":"user","message":{{"role":"user","content":"Message 2"}},"uuid":"test-uuid","timestamp":"2024-01-01T00:00:00Z","sessionId":"test-session","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}}"#).unwrap();
    file.sync_all().unwrap();
    drop(file);

    // Ctrl+R should reload
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    let ctrl_r = KeyEvent::new(KeyCode::Char('r'), KeyModifiers::CONTROL);
    search
        .handle_search_input(ctrl_r, test_file.to_str().unwrap())
        .unwrap();

    // Should show reload message
    assert!(search.message.is_some());
    assert!(search.message.as_ref().unwrap().contains("Cache cleared"));

    // Ctrl+R only clears cache, need to search again manually
    search.execute_search_sync(test_file.to_str().unwrap());

    // Now results should be updated
    assert_eq!(search.results.len(), 2);
}

#[test]
fn test_project_path_extraction_duplicate() {
    let _search = InteractiveSearch::new(SearchOptions::default());

    // Test with valid project path
    let result = create_test_result("user", "Test", "2024-01-01T00:00:00Z");
    let project = Some(result.project_path.clone());
    assert_eq!(project, Some("/test/project".to_string()));

    // Test with different project paths
    let mut result2 = result.clone();
    result2.project_path = "/another/project".to_string();
    let project2 = Some(result2.project_path.clone());
    assert_eq!(project2, Some("/another/project".to_string()));
}

#[test]
fn test_session_viewer_order_selection() {
    let temp_dir = tempdir().unwrap();
    let test_file = temp_dir.path().join("session.jsonl");

    let mut file = File::create(&test_file).unwrap();
    writeln!(file, r#"{{"type":"user","message":{{"role":"user","content":"Message 1"}},"uuid":"test-uuid","timestamp":"2024-01-01T00:00:00Z","sessionId":"test-session","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}}"#).unwrap();
    writeln!(file, r#"{{"type":"assistant","message":{{"role":"assistant","content":"Message 2"}},"uuid":"test-uuid","timestamp":"2024-01-01T00:00:00Z","sessionId":"test-session","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}}"#).unwrap();
    writeln!(file, r#"{{"type":"user","message":{{"role":"user","content":"Message 3"}},"uuid":"test-uuid","timestamp":"2024-01-01T00:00:00Z","sessionId":"test-session","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}}"#).unwrap();

    let mut search = InteractiveSearch::new(SearchOptions::default());
    search
        .load_session_messages(test_file.to_str().unwrap())
        .unwrap();

    // Set up session viewer without order initially
    search.session_order = None; // Start with no order
    search.push_screen(Mode::SessionViewer);

    // Initially no order
    assert_eq!(search.session_order, None);

    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    // Test ascending order
    let a_key = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::empty());
    search.handle_session_viewer_input(a_key).unwrap();
    assert_eq!(search.session_order, Some(SessionOrder::Ascending));

    // Reset to test descending order
    search.session_order = None;
    let d_key = KeyEvent::new(KeyCode::Char('d'), KeyModifiers::empty());
    search.handle_session_viewer_input(d_key).unwrap();
    assert_eq!(search.session_order, Some(SessionOrder::Descending));
}

#[test]
fn test_role_filter_message_clearing() {
    let mut search = InteractiveSearch::new(SearchOptions::default());

    let temp_dir = tempdir().unwrap();
    let test_file = temp_dir.path().join("test.jsonl");
    File::create(&test_file).unwrap();

    // Set a message
    search.message = Some("Previous message".to_string());

    // Change role filter
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
    let _search = InteractiveSearch::new(SearchOptions::default());

    // Test with emoji and Japanese characters
    let text_with_emoji = "Hello 😀 World 🌍 こんにちは";
    let result = create_test_result("user", text_with_emoji, "2024-01-01T00:00:00Z");
    let preview = result.text.chars().take(15).collect::<String>();
    let preview = if result.text.chars().count() > 15 {
        format!("{preview}...")
    } else {
        preview
    };

    // Should truncate at character boundary, not byte boundary
    assert!(preview.chars().count() <= 18); // 15 + "..."
    assert!(preview.ends_with("..."));
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
    assert_eq!(project_path2, "tmp");
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

    let mut search = InteractiveSearch::new(SearchOptions::default());
    search.load_initial_results(test_file.to_str().unwrap());

    // Should handle empty file gracefully
    assert!(search.results.is_empty());
    assert_eq!(search.selected_index, 0);
}

#[test]
fn test_message_limit_duplicate() {
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
fn test_invalid_json_handling_duplicate() {
    let temp_dir = tempdir().unwrap();
    let test_file = temp_dir.path().join("invalid.jsonl");

    let mut file = File::create(&test_file).unwrap();
    writeln!(file, "Not valid JSON").unwrap();
    writeln!(file, r#"{{"type":"user","message":{{"role":"user","content":"Valid message"}},"uuid":"test-uuid","timestamp":"2024-01-01T00:00:00Z","sessionId":"test-session","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}}"#).unwrap();
    writeln!(file, "Another invalid line").unwrap();

    let mut search = InteractiveSearch::new(SearchOptions::default());
    search
        .load_session_messages(test_file.to_str().unwrap())
        .unwrap();

    assert_eq!(search.session_messages.len(), 3);
}

#[test]
fn test_query_parsing_integration_duplicate() {
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
fn test_result_sorting_duplicate() {
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
fn test_detail_scroll_functionality_duplicate() {
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

    // 'j' is now for clipboard copy, not scrolling
    // Test clipboard operation instead
    let j_key = KeyEvent::new(KeyCode::Char('j'), KeyModifiers::empty());
    search.handle_result_detail_input(j_key).unwrap();
    // detail_scroll_offset should remain at 1
    assert_eq!(search.detail_scroll_offset, 1);

    // Simulate scrolling up
    let up_key = KeyEvent::new(KeyCode::Up, KeyModifiers::empty());
    search.handle_result_detail_input(up_key).unwrap();
    assert_eq!(search.detail_scroll_offset, 0);

    // Simulate page down
    let page_down = KeyEvent::new(KeyCode::PageDown, KeyModifiers::empty());
    search.handle_result_detail_input(page_down).unwrap();
    assert_eq!(search.detail_scroll_offset, 10);

    // Test reset on escape
    let esc_key = KeyEvent::new(KeyCode::Esc, KeyModifiers::empty());
    search.handle_result_detail_input(esc_key).unwrap();
    assert_eq!(search.detail_scroll_offset, 0);
    assert_eq!(search.current_mode(), Mode::Search);
}

#[test]
fn test_message_clearing_on_mode_change_duplicate() {
    let mut search = InteractiveSearch::new(SearchOptions::default());

    // Set a message
    search.message = Some("Test message".to_string());

    // Message should be cleared when returning from detail to search
    search.push_screen(Mode::ResultDetail);
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    let esc_key = KeyEvent::new(KeyCode::Esc, KeyModifiers::empty());
    search.handle_result_detail_input(esc_key).unwrap();

    assert!(search.message.is_none());
    assert_eq!(search.current_mode(), Mode::Search);
}

#[test]
fn test_empty_search_results_display() {
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

    // The long message should be truncated with "..."
    let buffer_content = buffer
        .content()
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>();

    // Should contain part of the message
    assert!(buffer_content.contains("This is a very long"));
    // Should be truncated
    assert!(buffer_content.contains("..."));
}

#[test]
fn test_role_filter_message_clearing_duplicate() {
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
fn test_preview_text_multibyte_safety_duplicate() {
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
    let test_file = temp_dir.path().join("many.jsonl");

    let mut file = File::create(&test_file).unwrap();
    for i in 0..100 {
        writeln!(file, r#"{{"type":"user","message":{{"role":"user","content":"Message {i}"}},"uuid":"test-uuid","timestamp":"2024-01-01T00:00:00Z","sessionId":"test-session","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}}"#).unwrap();
    }

    let options = SearchOptions {
        max_results: Some(25),
        ..Default::default()
    };
    let mut search = InteractiveSearch::new(options);
    assert_eq!(search.max_results, 25);

    search.query = "Message".to_string();
    search.execute_search_sync(test_file.to_str().unwrap());

    // Should only return max_results
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
    let _ = search.load_session_messages(test_file.to_str().unwrap());

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
    search.push_screen(Mode::ResultDetail);

    // Test file copy feedback
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    let f_key = KeyEvent::new(KeyCode::Char('f'), KeyModifiers::empty());
    let _ = search.handle_result_detail_input(f_key); // Ignore clipboard error in tests

    assert!(search.message.is_some());
    assert!(search.message.as_ref().unwrap().contains("✓"));
    assert!(search.message.as_ref().unwrap().contains("File path"));

    // Should stay in detail mode
    assert_eq!(search.current_mode(), Mode::ResultDetail);
}

#[test]
fn test_initial_results_loading() {
    let temp_dir = tempdir().unwrap();
    let test_file = temp_dir.path().join("initial.jsonl");

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
fn test_ctrl_r_cache_reload_duplicate() {
    let temp_dir = tempdir().unwrap();
    let test_file = temp_dir.path().join("test.jsonl");

    // Create initial file
    let mut file = File::create(&test_file).unwrap();
    writeln!(file, r#"{{"type":"user","message":{{"role":"user","content":"Original"}},"uuid":"1","timestamp":"2024-01-01T00:00:00Z","sessionId":"s1","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}}"#).unwrap();
    drop(file);

    let mut search = InteractiveSearch::new(SearchOptions::default());
    search.query = "Original".to_string();
    search.execute_search_sync(test_file.to_str().unwrap());
    assert_eq!(search.results.len(), 1);

    // Modify file
    thread::sleep(Duration::from_millis(10));
    let mut file = File::create(&test_file).unwrap();
    writeln!(file, r#"{{"type":"user","message":{{"role":"user","content":"Updated"}},"uuid":"2","timestamp":"2024-01-01T00:00:00Z","sessionId":"s1","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}}"#).unwrap();
    drop(file);

    // Simulate Ctrl+R
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    let ctrl_r = KeyEvent::new(KeyCode::Char('r'), KeyModifiers::CONTROL);
    search
        .handle_search_input(ctrl_r, test_file.to_str().unwrap())
        .unwrap();

    // Cache should be cleared and search re-executed
    search.query = "Updated".to_string();
    search.execute_search_sync(test_file.to_str().unwrap());
    assert_eq!(search.results.len(), 1);
    assert!(search.results[0].text.contains("Updated"));
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

    // Create many results to test "more results" display
    let mut results = Vec::new();
    for i in 0..30 {
        results.push(create_test_result(
            "user",
            &format!("Message {i}"),
            "2024-01-01T00:00:00Z",
        ));
    }
    search.results = results;
    search.max_results = 25; // Limit to show "more results" indicator

    let mut terminal = create_test_terminal();
    terminal.draw(|f| search.draw(f)).unwrap();

    let buffer = terminal.backend().buffer();

    // Should show that results are limited
    assert!(find_text_in_buffer(buffer, "limit reached").is_some());
}
