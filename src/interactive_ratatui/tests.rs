#[cfg(test)]
use super::{InteractiveSearch, MessageCache, Mode, SessionOrder};
use crate::{QueryCondition, SearchOptions, SearchResult};
use ratatui::{
    Terminal,
    backend::TestBackend,
    buffer::Buffer,
    style::{Color, Modifier},
};
use std::fs::File;
use std::io::Write;
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
    assert_eq!(search.current_mode(), Mode::Search);
    assert!(search.query.is_empty());
    assert_eq!(search.selected_index, 0);
    assert!(search.results.is_empty());
    assert!(search.role_filter.is_none());
}

#[test]
fn test_mode_transitions() {
    let mut search = InteractiveSearch::new(SearchOptions::default());

    // Start in Search mode
    assert_eq!(search.current_mode(), Mode::Search);

    // Transition to Help
    search.set_mode(Mode::Help);
    assert_eq!(search.current_mode(), Mode::Help);

    // Transition to ResultDetail
    search.set_mode(Mode::ResultDetail);
    assert_eq!(search.current_mode(), Mode::ResultDetail);

    // Transition to SessionViewer
    search.set_mode(Mode::SessionViewer);
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
        r#"{{"type":"user","content":"Test message"}}"#
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

    // Modify file
    thread::sleep(Duration::from_millis(10));
    writeln!(
        file,
        r#"{{"type":"assistant","content":"Response"}}"#
    )
    .unwrap();

    // Should reload due to modification time change
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
        r#"{{"type":"user","content":"Message 1"}}"#
    )
    .unwrap();

    let mut search = InteractiveSearch::new(SearchOptions::default());
    search.query = "Message".to_string();
    search.execute_search(test_file.to_str().unwrap());
    assert_eq!(search.results.len(), 1);

    // Add another message
    thread::sleep(Duration::from_millis(10));
    writeln!(
        file,
        r#"{{"type":"user","content":"Message 2"}}"#
    )
    .unwrap();

    search.execute_search(test_file.to_str().unwrap());
    assert_eq!(search.results.len(), 2);
}

#[test]
fn test_help_screen_rendering() {
    let mut terminal = create_test_terminal();
    let mut search = InteractiveSearch::new(SearchOptions::default());
    search.set_mode(Mode::Help);

    terminal.draw(|f| search.draw(f)).unwrap();
    let buffer = terminal.backend().buffer();

    // Check that help title is present
    let help_title = "Help";
    let mut found = false;
    for y in 0..buffer.area.height {
        for x in 0..buffer.area.width {
            let cell = &buffer[(x, y)];
            if cell.symbol.starts_with('H') {
                let mut title = String::new();
                for i in 0..help_title.len() {
                    if x + i as u16 >= buffer.area.width {
                        break;
                    }
                    title.push_str(&buffer[(x + i as u16, y)].symbol);
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
    search.set_mode(Mode::ResultDetail);

    terminal.draw(|f| search.draw(f)).unwrap();
    let buffer = terminal.backend().buffer();

    // Check that result detail title is present
    let detail_title = "Result Detail";
    let mut found = false;
    for y in 0..buffer.area.height {
        for x in 0..buffer.area.width {
            let cell = &buffer[(x, y)];
            if cell.symbol.starts_with('R') {
                let mut title = String::new();
                for i in 0..detail_title.len() {
                    if x + i as u16 >= buffer.area.width {
                        break;
                    }
                    title.push_str(&buffer[(x + i as u16, y)].symbol);
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

    search.handle_search_input(tab_key, test_file.to_str().unwrap()).unwrap();
    assert_eq!(search.role_filter, Some("user".to_string()));

    search.handle_search_input(tab_key, test_file.to_str().unwrap()).unwrap();
    assert_eq!(search.role_filter, Some("assistant".to_string()));

    search.handle_search_input(tab_key, test_file.to_str().unwrap()).unwrap();
    assert_eq!(search.role_filter, Some("system".to_string()));

    search.handle_search_input(tab_key, test_file.to_str().unwrap()).unwrap();
    assert_eq!(search.role_filter, Some("summary".to_string()));

    search.handle_search_input(tab_key, test_file.to_str().unwrap()).unwrap();
    assert_eq!(search.role_filter, None);
}

#[test]
fn test_query_parsing_integration() {
    let temp_dir = tempdir().unwrap();
    let test_file = temp_dir.path().join("test.jsonl");

    let mut file = File::create(&test_file).unwrap();
    writeln!(file, r#"{{"type":"user","content":"Hello world"}}"#).unwrap();
    writeln!(file, r#"{{"type":"assistant","content":"Goodbye world"}}"#).unwrap();
    writeln!(file, r#"{{"type":"system","content":"System message"}}"#).unwrap();

    let mut search = InteractiveSearch::new(SearchOptions::default());

    // Test AND query
    search.query = "Hello AND world".to_string();
    search.execute_search(test_file.to_str().unwrap());
    assert_eq!(search.results.len(), 1);
    assert_eq!(search.results[0].text, "Hello world");

    // Test OR query
    search.query = "Hello OR Goodbye".to_string();
    search.execute_search(test_file.to_str().unwrap());
    assert_eq!(search.results.len(), 2);

    // Test NOT query
    search.query = "world NOT Hello".to_string();
    search.execute_search(test_file.to_str().unwrap());
    assert_eq!(search.results.len(), 1);
    assert_eq!(search.results[0].text, "Goodbye world");
}

#[test]
fn test_execute_search_with_filters() {
    let temp_dir = tempdir().unwrap();
    let test_file = temp_dir.path().join("test.jsonl");

    let mut file = File::create(&test_file).unwrap();
    writeln!(file, r#"{{"type":"user","message":{{"role":"user","content":"User message"}},"uuid":"1","timestamp":"2024-01-01T00:00:00Z","sessionId":"s1","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}}"#).unwrap();
    writeln!(file, r#"{{"type":"assistant","message":{{"id":"msg1","type":"message","role":"assistant","model":"claude","content":[{{"type":"text","text":"Assistant response"}}],"stop_reason":"end_turn","stop_sequence":null,"usage":{{"input_tokens":10,"cache_creation_input_tokens":0,"cache_read_input_tokens":0,"output_tokens":5}}}},"uuid":"2","timestamp":"2024-01-01T00:00:01Z","sessionId":"s1","parentUuid":"1","isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}}"#).unwrap();
    writeln!(file, r#"{{"type":"system","content":"System message"}}"#).unwrap();

    let mut search = InteractiveSearch::new(SearchOptions::default());

    // Test role filter
    search.query = "message".to_string();
    search.role_filter = Some("user".to_string());
    search.execute_search(test_file.to_str().unwrap());
    assert_eq!(search.results.len(), 1);
    assert_eq!(search.results[0].role, "user");

    // Test assistant filter
    search.role_filter = Some("assistant".to_string());
    search.execute_search(test_file.to_str().unwrap());
    assert_eq!(search.results.len(), 1);
    assert_eq!(search.results[0].role, "assistant");

    // Test system filter
    search.role_filter = Some("system".to_string());
    search.execute_search(test_file.to_str().unwrap());
    assert_eq!(search.results.len(), 1);
    assert_eq!(search.results[0].role, "system");
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
    search.execute_search(test_file.to_str().unwrap());

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
    let (cursor_x, _cursor_y) = terminal.get_cursor().unwrap();
    // Expected position: starting position + query length
    assert!(cursor_x > 0);
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
            result.push_str(&buffer[(x, y)].symbol);
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
    let long_text = (0..100).map(|i| format!("Line {}", i)).collect::<Vec<_>>().join("\n");
    let result = create_test_result("user", &long_text, "2024-01-01T00:00:00Z");
    search.selected_result = Some(result);
    search.set_mode(Mode::ResultDetail);

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
    writeln!(file, r#"{{"type":"user","content":"Message 1"}}"#).unwrap();

    let mut search = InteractiveSearch::new(SearchOptions::default());
    search.query = "Message".to_string();
    search.execute_search(test_file.to_str().unwrap());
    assert_eq!(search.results.len(), 1);

    // Add another message
    thread::sleep(Duration::from_millis(10));
    writeln!(file, r#"{{"type":"user","content":"Message 2"}}"#).unwrap();

    // Ctrl+R should reload
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    let ctrl_r = KeyEvent::new(KeyCode::Char('r'), KeyModifiers::CONTROL);
    search.handle_search_input(ctrl_r, test_file.to_str().unwrap()).unwrap();

    // Should show reload message
    assert!(search.message.is_some());
    assert!(search.message.as_ref().unwrap().contains("Cache cleared"));
    
    // Results should be updated
    assert_eq!(search.results.len(), 2);
}

#[test]
fn test_project_path_extraction() {
    let mut search = InteractiveSearch::new(SearchOptions::default());

    // Test with valid project path
    let result = create_test_result("user", "Test", "2024-01-01T00:00:00Z");
    let project = search.extract_project_path(&result);
    assert_eq!(project, Some("/test/project".to_string()));

    // Test with different project paths
    let mut result2 = result.clone();
    result2.project_path = "/another/project".to_string();
    let project2 = search.extract_project_path(&result2);
    assert_eq!(project2, Some("/another/project".to_string()));
}

#[test]
fn test_session_viewer_order_selection() {
    let temp_dir = tempdir().unwrap();
    let test_file = temp_dir.path().join("session.jsonl");

    let mut file = File::create(&test_file).unwrap();
    writeln!(file, r#"{{"type":"user","content":"Message 1"}}"#).unwrap();
    writeln!(file, r#"{{"type":"assistant","content":"Message 2"}}"#).unwrap();
    writeln!(file, r#"{{"type":"user","content":"Message 3"}}"#).unwrap();

    let mut search = InteractiveSearch::new(SearchOptions::default());
    search.load_session_messages(test_file.to_str().unwrap()).unwrap();

    // Set up session viewer  
    search.session_order = Some(SessionOrder::Ascending);
    search.set_mode(Mode::SessionViewer);

    // Initially not ordered
    assert!(search.session_order.is_some());

    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    // Test ascending order
    let a_key = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::empty());
    search.handle_session_viewer_input(a_key).unwrap();
    assert_eq!(search.session_order, Some(SessionOrder::Ascending));

    // Test descending order
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
    search.handle_search_input(tab_key, test_file.to_str().unwrap()).unwrap();

    // Message should be cleared
    assert!(search.message.is_none());
    assert_eq!(search.role_filter, Some("user".to_string()));
}

#[test]
fn test_preview_text_multibyte_safety() {
    let mut search = InteractiveSearch::new(SearchOptions::default());

    // Test with emoji and Japanese characters
    let text_with_emoji = "Hello 😀 World 🌍 こんにちは";
    let result = create_test_result("user", text_with_emoji, "2024-01-01T00:00:00Z");
    let preview = search.get_preview_text(&result, 15);
    
    // Should truncate at character boundary, not byte boundary
    assert!(preview.chars().count() <= 18); // 15 + "..."
    assert!(preview.ends_with("..."));

    // Test with very short text
    let short_text = "Hi";
    let short_result = create_test_result("user", short_text, "2024-01-01T00:00:00Z");
    let short_preview = search.get_preview_text(&short_result, 20);
    assert_eq!(short_preview, "Hi");
    assert!(!short_preview.contains("..."));
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

    // Test empty query behavior
    search.query = "".to_string();
    search.execute_search(test_file.to_str().unwrap());
    assert!(search.results.is_empty());
}

#[test]
fn test_invalid_json_handling() {
    let temp_dir = tempdir().unwrap();
    let test_file = temp_dir.path().join("invalid.jsonl");

    let mut file = File::create(&test_file).unwrap();
    writeln!(file, "Not valid JSON").unwrap();
    writeln!(file, r#"{{"type":"user","content":"Valid message"}}"#).unwrap();
    writeln!(file, "Another invalid line").unwrap();

    let mut search = InteractiveSearch::new(SearchOptions::default());
    search.query = "Valid".to_string();
    search.execute_search(test_file.to_str().unwrap());

    // Should only find the valid message
    assert_eq!(search.results.len(), 1);
    assert_eq!(search.results[0].text, "Valid message");
}

#[test]
fn test_max_results_limit() {
    let temp_dir = tempdir().unwrap();
    let test_file = temp_dir.path().join("many.jsonl");

    let mut file = File::create(&test_file).unwrap();
    for i in 0..100 {
        writeln!(file, r#"{{"type":"user","content":"Message {}"}}"#, i).unwrap();
    }

    let options = SearchOptions {
        max_results: Some(25),
        ..Default::default()
    };
    let mut search = InteractiveSearch::new(options);
    assert_eq!(search.max_results, 25);

    search.query = "Message".to_string();
    search.execute_search(test_file.to_str().unwrap());

    // Should only return max_results
    assert_eq!(search.results.len(), 25);
}

#[test]
fn test_initial_results_loading() {
    let temp_dir = tempdir().unwrap();
    let test_file = temp_dir.path().join("initial.jsonl");

    let mut file = File::create(&test_file).unwrap();
    for i in 0..10 {
        writeln!(file, r#"{{"type":"user","message":{{"role":"user","content":"Message {}"}},"uuid":"{}","timestamp":"2024-01-01T00:00:{:02}Z","sessionId":"s1","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}}"#, i, i, i).unwrap();
    }

    let mut search = InteractiveSearch::new(SearchOptions::default());
    search.load_initial_results(test_file.to_str().unwrap());

    // Should have loaded results without any query
    assert!(!search.results.is_empty());
    // Should be sorted by timestamp (newest first)
    assert!(search.results[0].timestamp > search.results[1].timestamp);
}

#[test]
fn test_session_viewer_pagination() {
    let temp_dir = tempdir().unwrap();
    let test_file = temp_dir.path().join("session.jsonl");

    // Create session with many messages
    let mut file = File::create(&test_file).unwrap();
    for i in 0..10 {
        writeln!(file, r#"{{"type":"user","content":"Message {}"}}"#, i).unwrap();
    }

    let mut search = InteractiveSearch::new(SearchOptions::default());
    search.load_session_messages(test_file.to_str().unwrap()).unwrap();

    // Set up session viewer
    search.session_order = Some(SessionOrder::Ascending);
    search.set_mode(Mode::SessionViewer);

    // Test that we have multiple messages
    assert_eq!(search.session_messages.len(), 10);

    // Test page navigation
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    
    // Test down navigation
    let down_key = KeyEvent::new(KeyCode::Down, KeyModifiers::empty());
    search.handle_session_viewer_input(down_key).unwrap();
    assert_eq!(search.session_index, 1);

    // Test up navigation
    let up_key = KeyEvent::new(KeyCode::Up, KeyModifiers::empty());
    search.handle_session_viewer_input(up_key).unwrap();
    assert_eq!(search.session_index, 0);

    // Test page down
    let page_down = KeyEvent::new(KeyCode::PageDown, KeyModifiers::empty());
    search.handle_session_viewer_input(page_down).unwrap();
    assert!(search.session_index > 1);

    // Test page up
    let page_up = KeyEvent::new(KeyCode::PageUp, KeyModifiers::empty());
    search.handle_session_viewer_input(page_up).unwrap();
    assert_eq!(search.session_index, 0);
}

#[test]
fn test_result_limit_indicator() {
    let temp_dir = tempdir().unwrap();
    let test_file = temp_dir.path().join("many.jsonl");

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
    search.execute_search(test_file.to_str().unwrap());

    assert_eq!(search.results.len(), 50);

    // Draw and check for limit indicator
    let mut terminal = create_test_terminal();
    terminal.draw(|f| search.draw(f)).unwrap();
    
    let buffer = terminal.backend().buffer();
    let buffer_content = buffer_to_string(buffer);
    
    // Should show the limit indicator
    assert!(buffer_content.contains("50 results") || buffer_content.contains("limited"));
}

#[test]
fn test_more_results_display() {
    let temp_dir = tempdir().unwrap();
    let test_file = temp_dir.path().join("many.jsonl");

    // Create exactly max_results + 1 messages
    let mut file = File::create(&test_file).unwrap();
    for i in 0..51 {
        writeln!(file, r#"{{"type":"user","message":{{"role":"user","content":"Message {}"}},"uuid":"{}","timestamp":"2024-01-01T00:00:{:02}Z","sessionId":"s1","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}}"#, i, i, i % 60).unwrap();
    }

    let options = SearchOptions {
        max_results: Some(50),
        ..Default::default()
    };
    let mut search = InteractiveSearch::new(options);
    search.query = "Message".to_string();
    search.execute_search(test_file.to_str().unwrap());

    assert_eq!(search.results.len(), 50);

    // Draw and check for "more results" message
    let mut terminal = create_test_terminal();
    terminal.draw(|f| search.draw(f)).unwrap();
    
    let buffer = terminal.backend().buffer();
    let buffer_content = buffer_to_string(buffer);
    
    // Should indicate there are more results
    assert!(buffer_content.contains("50") && (buffer_content.contains("more") || buffer_content.contains("limited")));
}

#[test]
fn test_search_navigation_keys() {
    let temp_dir = tempdir().unwrap();
    let test_file = temp_dir.path().join("nav.jsonl");

    let mut file = File::create(&test_file).unwrap();
    for i in 0..30 {
        writeln!(file, r#"{{"type":"user","message":{{"role":"user","content":"Message {}"}},"uuid":"{}","timestamp":"2024-01-01T00:00:{:02}Z","sessionId":"s1","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}}"#, i, i, i).unwrap();
    }

    let mut search = InteractiveSearch::new(SearchOptions::default());
    search.query = "Message".to_string();
    search.execute_search(test_file.to_str().unwrap());

    // Test basic navigation
    assert_eq!(search.selected_index, 0);
    assert_eq!(search.scroll_offset, 0);

    // Navigate down within visible range (no scrolling yet)
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    let down_key = KeyEvent::new(KeyCode::Down, KeyModifiers::empty());
    for _ in 0..10 {
        search.handle_search_input(down_key, test_file.to_str().unwrap()).unwrap();
    }
    assert_eq!(search.selected_index, 10);
    assert_eq!(search.scroll_offset, 0); // Should not scroll yet

    // Continue navigating to trigger scroll
    for _ in 0..8 {
        search.handle_search_input(down_key, test_file.to_str().unwrap()).unwrap();
    }
    assert_eq!(search.selected_index, 18);

    // Navigate back up
    let up_key = KeyEvent::new(KeyCode::Up, KeyModifiers::empty());
    for _ in 0..5 {
        search.handle_search_input(up_key, test_file.to_str().unwrap()).unwrap();
    }
    assert_eq!(search.selected_index, 13);

    // Test Home key
    let home_key = KeyEvent::new(KeyCode::Home, KeyModifiers::empty());
    search.handle_search_input(home_key, test_file.to_str().unwrap()).unwrap();
    assert_eq!(search.selected_index, 0);
    assert_eq!(search.scroll_offset, 0);

    // Test End key
    let end_key = KeyEvent::new(KeyCode::End, KeyModifiers::empty());
    search.handle_search_input(end_key, test_file.to_str().unwrap()).unwrap();
    assert_eq!(search.selected_index, 29);
    assert!(search.scroll_offset > 0);
}

#[test]
fn test_empty_query_shows_initial_screen() {
    let mut terminal = create_test_terminal();
    let mut search = InteractiveSearch::new(SearchOptions::default());

    // With empty query, should show initial screen
    assert!(search.query.is_empty());
    terminal.draw(|f| search.draw(f)).unwrap();

    let buffer = terminal.backend().buffer();
    let buffer_content = buffer_to_string(buffer);

    // Should show the welcome message or instructions
    assert!(buffer_content.contains("Type to search") || buffer_content.contains("Enter query"));
}

#[test]
fn test_search_results_scrolling() {
    let temp_dir = tempdir().unwrap();
    let test_file = temp_dir.path().join("scroll.jsonl");

    let mut file = File::create(&test_file).unwrap();
    for i in 0..50 {
        writeln!(file, r#"{{"type":"user","message":{{"role":"user","content":"Message {}"}},"uuid":"{}","timestamp":"2024-01-01T00:00:{:02}Z","sessionId":"s1","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}}"#, i, i, i).unwrap();
    }

    let mut search = InteractiveSearch::new(SearchOptions::default());
    search.query = "Message".to_string();
    search.execute_search(test_file.to_str().unwrap());

    // Navigate to trigger scrolling
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    let down_key = KeyEvent::new(KeyCode::Down, KeyModifiers::empty());

    // Navigate down past visible area
    for i in 0..20 {
        search.handle_search_input(down_key, test_file.to_str().unwrap()).unwrap();
        
        // With terminal height 24, available height is 17 (24-7)
        // With scroll indicator, visible items is 16 (17-1)
        // So items 0-15 should be visible without scrolling
        if i < 15 {
            assert_eq!(search.scroll_offset, 0, "Should not scroll yet at index {}", i + 1);
        }
    }

    // Now should be scrolling
    assert!(search.scroll_offset > 0);

    // Test page down
    let page_down = KeyEvent::new(KeyCode::PageDown, KeyModifiers::empty());
    let prev_offset = search.scroll_offset;
    let prev_index = search.selected_index;
    search.handle_search_input(page_down, test_file.to_str().unwrap()).unwrap();
    assert!(search.selected_index > prev_index);
    assert!(search.scroll_offset >= prev_offset);

    // Test page up
    let page_up = KeyEvent::new(KeyCode::PageUp, KeyModifiers::empty());
    search.handle_search_input(page_up, test_file.to_str().unwrap()).unwrap();
    assert!(search.selected_index < 20);
}

#[test]
fn test_actual_scrolling_behavior() {
    let mut terminal = create_test_terminal();
    let mut search = InteractiveSearch::new(SearchOptions::default());

    let temp_dir = tempdir().unwrap();
    let test_file = temp_dir.path().join("many_results.jsonl");

    // Create many results to test scrolling
    let mut file = File::create(&test_file).unwrap();
    for i in 0..30 {
        writeln!(file, r#"{{"type":"user","message":{{"role":"user","content":"Message {}"}},"uuid":"{}","timestamp":"2024-01-01T00:00:{:02}Z","sessionId":"s1","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}}"#, i, i, i).unwrap();
    }

    search.query = "Message".to_string();
    search.execute_search(test_file.to_str().unwrap());
    assert_eq!(search.results.len(), 30);

    // Draw initial state
    terminal.draw(|f| search.draw(f)).unwrap();

    // Test scrolling behavior with actual terminal
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    let down_key = KeyEvent::new(KeyCode::Down, KeyModifiers::empty());

    // Navigate down and verify UI updates
    for i in 0..20 {
        search.handle_search_input(down_key, test_file.to_str().unwrap()).unwrap();
        terminal.draw(|f| search.draw(f)).unwrap();
        
        // The terminal should show the selected item
        assert_eq!(search.selected_index, i + 1);
    }

    // Verify scrolling happened
    assert!(search.scroll_offset > 0);
}

#[test]
fn test_non_blocking_input() {
    let temp_dir = tempdir().unwrap();
    let test_file = temp_dir.path().join("test.jsonl");

    let mut file = File::create(&test_file).unwrap();
    for i in 0..10 {
        writeln!(file, r#"{{"type":"user","message":{{"role":"user","content":"Message {}"}},"uuid":"{}","timestamp":"2024-01-01T00:00:{:02}Z","sessionId":"s1","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}}"#, i, i, i).unwrap();
    }

    let mut search = InteractiveSearch::new(SearchOptions::default());

    // Simulate typing
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    // Test immediate search execution on input
    let m_key = KeyEvent::new(KeyCode::Char('M'), KeyModifiers::empty());
    search.handle_search_input(m_key, test_file.to_str().unwrap()).unwrap();
    assert_eq!(search.query, "M");
    assert_eq!(search.results.len(), 10); // All messages match "M"

    // Test that each character triggers immediate search
    let e_key = KeyEvent::new(KeyCode::Char('e'), KeyModifiers::empty());
    search.handle_search_input(e_key, test_file.to_str().unwrap()).unwrap();
    assert_eq!(search.query, "Me");
    assert_eq!(search.results.len(), 10); // All messages still match "Me"

    // Test backspace also triggers immediate search
    let backspace = KeyEvent::new(KeyCode::Backspace, KeyModifiers::empty());
    search.handle_search_input(backspace, test_file.to_str().unwrap()).unwrap();
    assert_eq!(search.query, "M");
    assert_eq!(search.results.len(), 10); // Back to "M"

    // Clear query
    search.handle_search_input(backspace, test_file.to_str().unwrap()).unwrap();
    assert_eq!(search.query, "");
    // Empty query shows initial results
    assert!(!search.results.is_empty());
}

#[test]
fn test_message_truncation_with_multibyte() {
    let mut search = InteractiveSearch::new(SearchOptions::default());

    // Set a long message with multibyte characters
    let long_message = format!("これは非常に長いメッセージです。{}", "あ".repeat(100));
    search.message = Some(long_message);

    let mut terminal = create_test_terminal();
    terminal.draw(|f| search.draw(f)).unwrap();

    // Should not panic with multibyte truncation
    let buffer = terminal.backend().buffer();
    let buffer_content = buffer_to_string(buffer);
    
    // Message should be displayed (truncated if necessary)
    assert!(buffer_content.contains("これは"));
}

#[test]
fn test_dynamic_message_truncation() {
    let mut search = InteractiveSearch::new(SearchOptions::default());
    
    // Test short message
    search.message = Some("Short".to_string());
    assert_eq!(search.truncate_message("Short", 10), "Short");

    // Test exact length
    search.message = Some("Exact fit!".to_string());
    assert_eq!(search.truncate_message("Exact fit!", 10), "Exact fit!");

    // Test truncation
    search.message = Some("This is a very long message".to_string());
    let truncated = search.truncate_message("This is a very long message", 10);
    assert_eq!(truncated, "This is...");
    assert_eq!(truncated.chars().count(), 10);

    // Test with emoji
    let emoji_msg = "Hello 😀 World 🌍";
    let truncated_emoji = search.truncate_message(emoji_msg, 10);
    assert!(truncated_emoji.ends_with("..."));
    assert!(truncated_emoji.chars().count() <= 10);
}

#[test]
fn test_exit_goodbye_message() {
    let mut search = InteractiveSearch::new(SearchOptions::default());

    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    let q_key = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::empty());

    let temp_dir = tempdir().unwrap();
    let test_file = temp_dir.path().join("test.jsonl");
    File::create(&test_file).unwrap();

    let continue_running = search.handle_search_input(q_key, test_file.to_str().unwrap()).unwrap();
    assert!(!continue_running);

    // Should show goodbye message
    assert!(search.message.is_some());
    assert!(search.message.as_ref().unwrap().contains("Goodbye") || 
            search.message.as_ref().unwrap().contains("goodbye"));
}

#[test]
fn test_centered_rect_calculation() {
    let search = InteractiveSearch::new(SearchOptions::default());

    let area = ratatui::layout::Rect {
        x: 0,
        y: 0,
        width: 80,
        height: 24,
    };

    let centered = search.centered_rect(50, 50, area);
    
    // Should be centered
    assert_eq!(centered.width, 40); // 50% of 80
    assert_eq!(centered.height, 12); // 50% of 24
    assert_eq!(centered.x, 20); // (80 - 40) / 2
    assert_eq!(centered.y, 6); // (24 - 12) / 2
}

#[test]
fn test_escape_key_behaviors() {
    let mut search = InteractiveSearch::new(SearchOptions::default());

    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    let esc_key = KeyEvent::new(KeyCode::Esc, KeyModifiers::empty());

    // Test plain Esc exits from search mode
    search.set_mode(Mode::Search);
    let temp_dir = tempdir().unwrap();
    let temp_file = temp_dir.path().join("test.jsonl");
    File::create(&temp_file).unwrap();
    let continue_running = search.handle_search_input(esc_key, temp_file.to_str().unwrap()).unwrap();
    assert!(!continue_running); // false means exit

    // From detail mode - Esc should return to search
    search.push_screen(Mode::ResultDetail);
    search.detail_scroll_offset = 10;
    search.message = Some("Test message".to_string());
    search.handle_result_detail_input(esc_key).unwrap();
    assert_eq!(search.current_mode(), Mode::Search);
    assert_eq!(search.detail_scroll_offset, 0);
    assert!(search.message.is_none());
}

#[test]
fn test_copy_feedback_messages() {
    let mut search = InteractiveSearch::new(SearchOptions::default());

    let result = create_test_result("user", "Test message", "2024-01-01T00:00:00Z");
    search.selected_result = Some(result);
    search.set_mode(Mode::ResultDetail);

    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    // Test F - Copy file path
    search.message = None;
    let f_key = KeyEvent::new(KeyCode::Char('f'), KeyModifiers::empty());
    search.handle_result_detail_input(f_key).unwrap();
    assert!(search.message.is_some());
    assert!(search.message.as_ref().unwrap().contains("File path copied"));

    // Test I - Copy session ID
    search.message = None;
    let i_key = KeyEvent::new(KeyCode::Char('i'), KeyModifiers::empty());
    search.handle_result_detail_input(i_key).unwrap();
    assert!(search.message.is_some());
    assert!(search.message.as_ref().unwrap().contains("Session ID copied"));

    // Test P - Copy project path
    search.message = None;
    let p_key = KeyEvent::new(KeyCode::Char('p'), KeyModifiers::empty());
    search.handle_result_detail_input(p_key).unwrap();
    assert!(search.message.is_some());
    assert!(search.message.as_ref().unwrap().contains("Project path copied"));

    // Test M - Copy message text
    search.message = None;
    let m_key = KeyEvent::new(KeyCode::Char('m'), KeyModifiers::empty());
    search.handle_result_detail_input(m_key).unwrap();
    assert!(search.message.is_some());
    assert!(search.message.as_ref().unwrap().contains("Message copied"));

    // Test J - Copy JSON
    search.message = None;
    let j_key = KeyEvent::new(KeyCode::Char('j'), KeyModifiers::empty());
    search.handle_result_detail_input(j_key).unwrap();
    assert!(search.message.is_some());
    assert!(search.message.as_ref().unwrap().contains("JSON copied"));
}

#[test]
fn test_clipboard_error_handling() {
    let mut search = InteractiveSearch::new(SearchOptions::default());

    // Create result without raw_json
    let mut result = create_test_result("user", "Test", "2024-01-01T00:00:00Z");
    result.raw_json = None;
    search.selected_result = Some(result);
    search.set_mode(Mode::ResultDetail);

    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    // Test J - Should show error when no JSON available
    let j_key = KeyEvent::new(KeyCode::Char('j'), KeyModifiers::empty());
    search.handle_result_detail_input(j_key).unwrap();
    assert!(search.message.is_some());
    assert!(search.message.as_ref().unwrap().contains("No JSON"));
}

#[test]
fn test_scrolling_with_ratatui_terminal() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    let mut search = InteractiveSearch::new(SearchOptions::default());

    // Create test data
    let temp_dir = tempdir().unwrap();
    let test_file = temp_dir.path().join("scroll_test.jsonl");
    let mut file = File::create(&test_file).unwrap();
    
    for i in 0..30 {
        writeln!(file, r#"{{"type":"user","message":{{"role":"user","content":"Message {}"}},"uuid":"{}","timestamp":"2024-01-01T00:00:{:02}Z","sessionId":"s1","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}}"#, i, i, i).unwrap();
    }

    search.query = "Message".to_string();
    search.execute_search(test_file.to_str().unwrap());

    // Initial draw
    terminal.draw(|f| search.draw(f)).unwrap();

    // Navigate and redraw
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    let down_key = KeyEvent::new(KeyCode::Down, KeyModifiers::empty());

    for _ in 0..20 {
        search.handle_search_input(down_key, test_file.to_str().unwrap()).unwrap();
        terminal.draw(|f| search.draw(f)).unwrap();
    }

    // Check final state
    assert_eq!(search.selected_index, 20);
    assert!(search.scroll_offset > 0);
}

#[test]
fn test_help_mode_key_binding() {
    let mut search = InteractiveSearch::new(SearchOptions::default());

    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    // Test '?' key opens help
    let question_key = KeyEvent::new(KeyCode::Char('?'), KeyModifiers::empty());
    
    let temp_dir = tempdir().unwrap();
    let test_file = temp_dir.path().join("test.jsonl");
    File::create(&test_file).unwrap();

    search.handle_search_input(question_key, test_file.to_str().unwrap()).unwrap();
    assert_eq!(search.current_mode(), Mode::Help);

    // Any key should return from help
    let any_key = KeyEvent::new(KeyCode::Char('x'), KeyModifiers::empty());
    search.handle_search_input(any_key, test_file.to_str().unwrap()).unwrap();
    assert_eq!(search.current_mode(), Mode::Search);
}

#[test]
fn test_session_detail_navigation() {
    let temp_dir = tempdir().unwrap();
    let test_file = temp_dir.path().join("test.jsonl");

    let mut file = File::create(&test_file).unwrap();
    writeln!(file, r#"{{"type":"user","message":{{"role":"user","content":"Test message"}},"uuid":"1","timestamp":"2024-01-01T00:00:00Z","sessionId":"test-session","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}}"#).unwrap();

    let mut search = InteractiveSearch::new(SearchOptions::default());
    search.query = "Test".to_string();
    search.execute_search(test_file.to_str().unwrap());

    // Navigate to result detail
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    let enter_key = KeyEvent::new(KeyCode::Enter, KeyModifiers::empty());
    search.handle_search_input(enter_key, test_file.to_str().unwrap()).unwrap();
    assert_eq!(search.current_mode(), Mode::ResultDetail);
    assert!(search.selected_result.is_some());

    // Test 's' key opens session viewer
    let s_key = KeyEvent::new(KeyCode::Char('s'), KeyModifiers::empty());
    search.handle_result_detail_input(s_key).unwrap();
    
    // Should show message about loading session
    assert!(search.message.is_some());
}

#[test]
fn test_session_viewer_quit() {
    let mut search = InteractiveSearch::new(SearchOptions::default());
    
    // Simulate navigating from Search to SessionViewer
    search.push_screen(Mode::SessionViewer);
    
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    
    // Test 'q' returns to previous screen
    let q_key = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::empty());
    search.handle_session_viewer_input(q_key).unwrap();
    assert_eq!(search.current_mode(), Mode::Search);
    
    // Test 'Q' also works
    search.push_screen(Mode::SessionViewer);
    let big_q_key = KeyEvent::new(KeyCode::Char('Q'), KeyModifiers::empty());
    search.handle_session_viewer_input(big_q_key).unwrap();
    assert_eq!(search.current_mode(), Mode::Search);
    
    // Test Esc also works
    search.push_screen(Mode::SessionViewer);
    let esc_key = KeyEvent::new(KeyCode::Esc, KeyModifiers::empty());
    search.handle_session_viewer_input(esc_key).unwrap();
    assert_eq!(search.current_mode(), Mode::Search);
}

#[test]
fn test_navigation_stack_behavior() {
    let mut search = InteractiveSearch::new(SearchOptions::default());
    
    // Start in Search
    assert_eq!(search.current_mode(), Mode::Search);
    
    // Navigate to ResultDetail
    search.push_screen(Mode::ResultDetail);
    assert_eq!(search.current_mode(), Mode::ResultDetail);
    
    // Navigate to SessionViewer
    search.push_screen(Mode::SessionViewer);
    assert_eq!(search.current_mode(), Mode::SessionViewer);
    
    // Pop back to ResultDetail
    search.pop_screen();
    assert_eq!(search.current_mode(), Mode::ResultDetail);
    
    // Pop back to Search
    search.pop_screen();
    assert_eq!(search.current_mode(), Mode::Search);
    
    // Popping from Search should stay in Search
    search.pop_screen();
    assert_eq!(search.current_mode(), Mode::Search);
}

#[test]
fn test_error_message_color() {
    let mut terminal = create_test_terminal();
    let mut search = InteractiveSearch::new(SearchOptions::default());
    
    // Set an error message
    search.message = Some("⚠ Error: Something went wrong".to_string());
    
    terminal.draw(|f| search.draw(f)).unwrap();
    let buffer = terminal.backend().buffer();
    
    // Find the error message in the buffer
    let error_marker = "⚠";
    for y in 0..buffer.area.height {
        for x in 0..buffer.area.width {
            let cell = &buffer[(x, y)];
            if cell.symbol == error_marker {
                // Error messages should be displayed in red
                assert_eq!(cell.fg, Color::Red);
                break;
            }
        }
    }
}

#[test]
fn test_session_order_display() {
    let mut search = InteractiveSearch::new(SearchOptions::default());
    
    // Create test session messages
    search.session_messages = vec![
        r#"{"type":"user","message":{"role":"user","content":"First message"},"uuid":"1","timestamp":"2024-01-01T00:00:00Z","sessionId":"test-session","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}"#.to_string(),
        r#"{"type":"assistant","message":{"id":"msg1","type":"message","role":"assistant","model":"claude","content":[{"type":"text","text":"Response"}],"stop_reason":"end_turn","stop_sequence":null,"usage":{"input_tokens":10,"cache_creation_input_tokens":0,"cache_read_input_tokens":0,"output_tokens":5}}},"uuid":"2","timestamp":"2024-01-01T00:00:01Z","sessionId":"test-session","parentUuid":"1","isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}"#.to_string(),
        r#"{"type":"user","message":{"role":"user","content":"Second message"},"uuid":"3","timestamp":"2024-01-01T00:00:02Z","sessionId":"test-session","parentUuid":"2","isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}"#.to_string(),
        r#"{"type":"assistant","message":{"id":"msg2","type":"message","role":"assistant","model":"claude","content":[{"type":"text","text":"Another response"}],"stop_reason":"end_turn","stop_sequence":null,"usage":{"input_tokens":10,"cache_creation_input_tokens":0,"cache_read_input_tokens":0,"output_tokens":5}}},"uuid":"4","timestamp":"2024-01-01T00:00:03Z","sessionId":"test-session","parentUuid":"3","isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}"#.to_string(),
    ];
    search.session_index = 0;
    search.session_order = None;
    search.set_mode(Mode::SessionViewer);
    assert!(search.session_order.is_none()); // Not selected yet

    // Select ascending order
    let a_key = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::empty());
    search.handle_session_viewer_input(a_key).unwrap();
    assert_eq!(search.session_order, Some(SessionOrder::Ascending));

    // Test descending order
    let d_key = KeyEvent::new(KeyCode::Char('d'), KeyModifiers::empty());
    search.handle_session_viewer_input(d_key).unwrap();
    assert_eq!(search.session_order, Some(SessionOrder::Descending));

    // Messages should be reversed
    assert!(search.session_messages[0].contains("Another response"));
    assert!(search.session_messages[3].contains("First message"));
}

#[test]
fn test_color_and_formatting() {
    let mut terminal = create_test_terminal();
    let mut search = InteractiveSearch::new(SearchOptions::default());

    // Create a test result with specific role
    let result = create_test_result("user", "Test message content", "2024-01-01T12:34:56Z");
    search.results = vec![result];
    search.query = "test".to_string();

    terminal.draw(|f| search.draw(f)).unwrap();
    let buffer = terminal.backend().buffer();

    // Helper function to find text in buffer
    fn find_text_in_buffer(buffer: &Buffer, text: &str) -> Option<(u16, u16)> {
        for y in 0..buffer.area.height {
            for x in 0..buffer.area.width {
                if x + text.len() as u16 <= buffer.area.width {
                    let mut found = true;
                    for (i, ch) in text.chars().enumerate() {
                        if buffer[(x + i as u16, y)].symbol != ch.to_string() {
                            found = false;
                            break;
                        }
                    }
                    if found {
                        return Some((x, y));
                    }
                }
            }
        }
        None
    }

    // Test title color (should be cyan and bold)
    let title_text = "Claude Session Search";
    if let Some(pos) = find_text_in_buffer(buffer, title_text) {
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
    search.set_mode(Mode::Help);
    let mut terminal = create_test_terminal();

    terminal.draw(|f| search.draw(f)).unwrap();
    let buffer = terminal.backend().buffer();

    // Check for help title styling
    let help_title = "Help";
    let mut found_title = false;
    
    for y in 0..buffer.area.height {
        for x in 0..buffer.area.width {
            if x + help_title.len() as u16 <= buffer.area.width {
                let mut is_title = true;
                for (i, ch) in help_title.chars().enumerate() {
                    if buffer[(x + i as u16, y)].symbol != ch.to_string() {
                        is_title = false;
                        break;
                    }
                }
                if is_title {
                    // Check that the title has proper styling
                    let cell = &buffer[(x, y)];
                    assert_eq!(cell.fg, Color::Cyan);
                    assert!(cell.modifier.contains(Modifier::BOLD));
                    found_title = true;
                    break;
                }
            }
        }
        if found_title {
            break;
        }
    }
    
    assert!(found_title, "Help title not found with proper styling");

    // Check for keyboard shortcut highlighting
    // Look for patterns like "q" or "Esc" that should be highlighted
    let mut found_shortcut = false;
    for y in 0..buffer.area.height {
        for x in 0..buffer.area.width {
            let cell = &buffer[(x, y)];
            // Single letter shortcuts like 'q', 'j', 'k' should be highlighted
            if cell.symbol.len() == 1 && cell.symbol.chars().all(|c| c.is_alphabetic() || c == '?') {
                // Check if it's styled as a shortcut (usually different color)
                if cell.fg == Color::Green || cell.fg == Color::Yellow {
                    found_shortcut = true;
                    break;
                }
            }
        }
        if found_shortcut {
            break;
        }
    }
    
    assert!(found_shortcut, "No highlighted keyboard shortcuts found");
}

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

#[test]
fn test_help_page() {
    let mut search = InteractiveSearch::new(SearchOptions::default());
    assert_eq!(search.current_mode(), Mode::Search);
    
    // Open help
    search.push_screen(Mode::Help);
    assert_eq!(search.current_mode(), Mode::Help);
    
    // Close help should return to Search
    search.pop_screen();
    assert_eq!(search.current_mode(), Mode::Search);
}

#[test]
fn test_message_truncation() {
    let search = InteractiveSearch::new(SearchOptions::default());
    
    // Test short message
    assert_eq!(search.truncate_message("Hello", 10), "Hello");
    
    // Test exact length
    assert_eq!(search.truncate_message("1234567890", 10), "1234567890");
    
    // Test truncation
    assert_eq!(search.truncate_message("Hello World!", 10), "Hello W...");
    
    // Test with multibyte characters
    let japanese = "こんにちは世界です";
    let truncated = search.truncate_message(japanese, 8);
    assert!(truncated.ends_with("..."));
    assert!(truncated.chars().count() <= 8);
}
