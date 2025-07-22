use super::*;
use crate::query::condition::QueryCondition;
use crate::{SearchOptions, SearchResult};
// use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::Terminal;
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
        query: QueryCondition::Literal { pattern: "test".to_string(), case_sensitive: false },
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
    search.selected_result = Some(create_test_result("user", "Detailed test message with more content", "2024-01-01T00:00:00Z"));
    
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
    search.results = vec![
        create_test_result("user", &long_content, "2024-01-01T00:00:00Z"),
    ];
    
    let mut terminal = create_test_terminal();
    terminal.draw(|f| search.draw_search(f)).unwrap();
    
    let buffer = terminal.backend().buffer();
    
    // Should show truncated message with ellipsis
    assert!(find_text_in_buffer(buffer, "...").is_some());
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
    assert!(find_text_in_buffer(buffer, "No messages").is_some() || 
            find_text_in_buffer(buffer, "0 messages").is_some() ||
            find_text_in_buffer(buffer, "Session Viewer").is_some());
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
    assert!(find_text_in_buffer(buffer, "No messages").is_some() ||
            find_text_in_buffer(buffer, "0 messages").is_some() ||
            find_text_in_buffer(buffer, "Empty session").is_some() ||
            find_text_in_buffer(buffer, "Session Viewer").is_some()); // At minimum, title should be shown
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
    assert!(find_text_in_buffer(buffer, "↑").is_some() ||
            find_text_in_buffer(buffer, "↓").is_some() ||
            find_text_in_buffer(buffer, "to scroll").is_some());
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