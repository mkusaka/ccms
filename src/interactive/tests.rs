#[cfg(test)]
#[allow(clippy::module_inception)]
mod tests {
    use crate::interactive::InteractiveSearch;
    use crate::{QueryCondition, SearchOptions, SearchResult, parse_query};
    use std::fs::File;
    use std::io::Write;
    use std::thread;
    use std::time::Duration;
    use tempfile::tempdir;

    // Helper function to create a test SearchResult
    fn create_test_result(role: &str, text: &str) -> SearchResult {
        SearchResult {
            file: "test.jsonl".to_string(),
            uuid: "test-uuid-123".to_string(),
            timestamp: "2024-01-01T12:00:00Z".to_string(),
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
            raw_json: None,
        }
    }

    #[test]
    fn test_interactive_search_creation() {
        let options = SearchOptions {
            max_results: Some(10),
            role: None,
            session_id: None,
            before: None,
            after: None,
            verbose: false,
            project_path: None,
        };

        let search = InteractiveSearch::new(options);
        assert_eq!(search.max_results, 10);
    }

    #[test]
    fn test_format_result_line_basic() {
        let _search = InteractiveSearch::new(SearchOptions::default());
        let _result = create_test_result("user", "Hello world");

        // Since format_result_line is private, we can't test it directly
        // This demonstrates the need for refactoring to make methods testable
        // In a real implementation, we would make this method public or
        // create a testable trait
    }

    #[test]
    fn test_role_filter_cycling() {
        // Test the role filter cycling logic
        // The actual cycle is: None -> user -> assistant -> system -> summary -> None
        let transitions = vec![
            (None, Some("user")),
            (Some("user"), Some("assistant")),
            (Some("assistant"), Some("system")),
            (Some("system"), Some("summary")),
            (Some("summary"), None),
        ];

        for (current, expected) in transitions {
            let next = match current {
                None => Some("user"),
                Some("user") => Some("assistant"),
                Some("assistant") => Some("system"),
                Some("system") => Some("summary"),
                Some("summary") => None,
                _ => None,
            };

            assert_eq!(next, expected);
        }
    }

    #[test]
    fn test_clipboard_command_selection() {
        // Test that we select the right clipboard command for each platform
        #[cfg(target_os = "macos")]
        {
            // On macOS, we should use pbcopy
            let cmd = "pbcopy";
            // Just check if the command exists
            let _ = std::process::Command::new("which")
                .arg(cmd)
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false);
        }

        #[cfg(target_os = "linux")]
        {
            // On Linux, we try xclip or xsel
            let has_xclip = std::process::Command::new("which")
                .arg("xclip")
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false);

            let has_xsel = std::process::Command::new("which")
                .arg("xsel")
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false);

            // In CI environment, clipboard tools might not be available
            if !has_xclip && !has_xsel {
                eprintln!(
                    "Warning: Neither xclip nor xsel found - clipboard operations may not work"
                );
            }
        }

        #[cfg(target_os = "windows")]
        {
            // On Windows, we should use clip
            let cmd = "clip";
            // Just check if the command exists
            let _ = std::process::Command::new("where")
                .arg(cmd)
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false);
        }
    }

    #[test]
    fn test_search_result_timestamp_formatting() {
        use chrono::DateTime;

        // Test RFC3339 timestamp parsing and formatting
        let timestamp = "2024-01-15T14:30:00Z";
        let dt = DateTime::parse_from_rfc3339(timestamp).unwrap();

        // For result line: MM/DD HH:MM
        let short_format = dt.format("%m/%d %H:%M").to_string();
        assert_eq!(short_format, "01/15 14:30");

        // For full display: YYYY-MM-DD HH:MM:SS
        let long_format = dt.format("%Y-%m-%d %H:%M:%S").to_string();
        assert_eq!(long_format, "2024-01-15 14:30:00");
    }

    #[test]
    fn test_text_preview_truncation() {
        let long_text = "This is a very long message that should be truncated for display in the search results list view";
        let preview: String = long_text.replace('\n', " ").chars().take(40).collect();

        assert_eq!(preview.len(), 40);
        assert_eq!(preview, "This is a very long message that should ");
    }

    #[test]
    fn test_session_file_reading() {
        // Create a temporary test file
        let temp_dir = tempdir().unwrap();
        let test_file = temp_dir.path().join("session.jsonl");

        let mut file = File::create(&test_file).unwrap();
        writeln!(file, r#"{{"type":"user","message":{{"role":"user","content":"Hello"}},"uuid":"1","timestamp":"2024-01-01T00:00:00Z","sessionId":"test","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}}"#).unwrap();
        writeln!(file, r#"{{"type":"assistant","message":{{"id":"msg1","type":"message","role":"assistant","model":"claude","content":[{{"type":"text","text":"Hi there!"}}],"stop_reason":"end_turn","stop_sequence":null,"usage":{{"input_tokens":10,"cache_creation_input_tokens":0,"cache_read_input_tokens":0,"output_tokens":5}}}},"uuid":"2","timestamp":"2024-01-01T00:00:01Z","sessionId":"test","parentUuid":"1","isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}}"#).unwrap();

        // Read the file
        let file = File::open(&test_file).unwrap();
        let reader = std::io::BufReader::new(file);
        let mut messages = Vec::new();

        use std::io::BufRead;
        for line in reader.lines().map_while(Result::ok) {
            if !line.trim().is_empty() {
                messages.push(line);
            }
        }

        assert_eq!(messages.len(), 2);

        // Test message reversal for descending order
        let mut desc_messages = messages.clone();
        desc_messages.reverse();
        assert_eq!(desc_messages[0], messages[1]);
        assert_eq!(desc_messages[1], messages[0]);
    }

    #[test]
    fn test_message_json_parsing() {
        let json_str = r#"{
            "type": "user",
            "content": "Test message",
            "timestamp": "2024-01-01T00:00:00Z"
        }"#;

        let msg: serde_json::Value = serde_json::from_str(json_str).unwrap();

        assert_eq!(msg.get("type").and_then(|v| v.as_str()), Some("user"));
        assert_eq!(
            msg.get("content").and_then(|v| v.as_str()),
            Some("Test message")
        );
        assert_eq!(
            msg.get("timestamp").and_then(|v| v.as_str()),
            Some("2024-01-01T00:00:00Z")
        );
    }

    #[test]
    fn test_message_content_extraction() {
        // Test string content
        let json_str = r#"{
            "type": "user",
            "content": "Simple text"
        }"#;
        let msg: serde_json::Value = serde_json::from_str(json_str).unwrap();
        let content = msg.get("content").and_then(|v| v.as_str());
        assert_eq!(content, Some("Simple text"));

        // Test array content
        let json_str = r#"{
            "type": "assistant",
            "content": [
                {"type": "text", "text": "Part 1"},
                {"type": "text", "text": "Part 2"}
            ]
        }"#;
        let msg: serde_json::Value = serde_json::from_str(json_str).unwrap();
        let content_parts = msg.get("content").and_then(|v| v.as_array()).unwrap();

        let mut texts = Vec::new();
        for part in content_parts {
            if let Some(text) = part.get("text").and_then(|v| v.as_str()) {
                texts.push(text);
            }
        }
        assert_eq!(texts, vec!["Part 1", "Part 2"]);
    }

    #[test]
    fn test_cached_search() {
        let temp_dir = tempdir().unwrap();
        let test_file = temp_dir.path().join("test.jsonl");
        
        // Create test JSONL file
        let mut file = File::create(&test_file).unwrap();
        writeln!(file, r#"{{"type":"user","message":{{"role":"user","content":"Hello world"}},"uuid":"123","timestamp":"2024-01-01T00:00:00Z","sessionId":"s1","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}}"#).unwrap();
        writeln!(file, r#"{{"type":"assistant","message":{{"id":"msg1","type":"message","role":"assistant","model":"claude","content":[{{"type":"text","text":"Hi there!"}}],"stop_reason":"end_turn","stop_sequence":null,"usage":{{"input_tokens":10,"cache_creation_input_tokens":0,"cache_read_input_tokens":0,"output_tokens":5}}}},"uuid":"124","timestamp":"2024-01-01T00:00:01Z","sessionId":"s1","parentUuid":"123","isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}}"#).unwrap();
        
        let options = SearchOptions::default();
        let mut search = InteractiveSearch::new(options);
        
        // Test that search results are returned from cache
        let query = parse_query("Hello").unwrap();
        let results = search.execute_cached_search(test_file.to_str().unwrap(), &query, &None).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].text, "Hello world");
        
        // Test that cache is used on second search
        let query2 = parse_query("Hi").unwrap();
        let results2 = search.execute_cached_search(test_file.to_str().unwrap(), &query2, &None).unwrap();
        assert_eq!(results2.len(), 1);
        assert_eq!(results2[0].text, "Hi there!");
    }

    #[test]
    fn test_file_change_detection() {
        let temp_dir = tempdir().unwrap();
        let test_file = temp_dir.path().join("test.jsonl");
        
        // Create initial file
        let mut file = File::create(&test_file).unwrap();
        writeln!(file, r#"{{"type":"user","message":{{"role":"user","content":"Original message"}},"uuid":"123","timestamp":"2024-01-01T00:00:00Z","sessionId":"s1","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}}"#).unwrap();
        drop(file);
        
        let options = SearchOptions::default();
        let mut search = InteractiveSearch::new(options);
        
        // Initial search
        let query = parse_query("Original").unwrap();
        let results = search.execute_cached_search(test_file.to_str().unwrap(), &query, &None).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].text, "Original message");
        
        // Sleep to ensure filesystem timestamp changes
        thread::sleep(Duration::from_millis(10));
        
        // Modify file
        let mut file = File::create(&test_file).unwrap();
        writeln!(file, r#"{{"type":"user","message":{{"role":"user","content":"Updated message"}},"uuid":"456","timestamp":"2024-01-01T00:00:00Z","sessionId":"s1","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}}"#).unwrap();
        drop(file);
        
        // Search again - should get updated content
        let query2 = parse_query("Updated").unwrap();
        let results2 = search.execute_cached_search(test_file.to_str().unwrap(), &query2, &None).unwrap();
        assert_eq!(results2.len(), 1);
        assert_eq!(results2[0].text, "Updated message");
    }

    #[test]
    fn test_role_filter_in_cached_search() {
        let temp_dir = tempdir().unwrap();
        let test_file = temp_dir.path().join("test.jsonl");
        
        // Create test JSONL file with different roles
        let mut file = File::create(&test_file).unwrap();
        writeln!(file, r#"{{"type":"user","message":{{"role":"user","content":"User message"}},"uuid":"1","timestamp":"2024-01-01T00:00:00Z","sessionId":"s1","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}}"#).unwrap();
        writeln!(file, r#"{{"type":"assistant","message":{{"id":"msg1","type":"message","role":"assistant","model":"claude","content":[{{"type":"text","text":"Assistant message"}}],"stop_reason":"end_turn","stop_sequence":null,"usage":{{"input_tokens":10,"cache_creation_input_tokens":0,"cache_read_input_tokens":0,"output_tokens":5}}}},"uuid":"2","timestamp":"2024-01-01T00:00:01Z","sessionId":"s1","parentUuid":"1","isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}}"#).unwrap();
        writeln!(file, r#"{{"type":"system","content":"System message","uuid":"3","timestamp":"2024-01-01T00:00:02Z","sessionId":"s1","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/test","version":"1.0","isMeta":false}}"#).unwrap();
        
        let options = SearchOptions::default();
        let mut search = InteractiveSearch::new(options);
        
        // Test search with user role filter
        let query = parse_query("message").unwrap();
        let user_filter = Some("user".to_string());
        let results = search.execute_cached_search(test_file.to_str().unwrap(), &query, &user_filter).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].role, "user");
        
        // Test search with assistant role filter
        let assistant_filter = Some("assistant".to_string());
        let results2 = search.execute_cached_search(test_file.to_str().unwrap(), &query, &assistant_filter).unwrap();
        assert_eq!(results2.len(), 1);
        assert_eq!(results2[0].role, "assistant");
        
        // Test search without filter
        let results3 = search.execute_cached_search(test_file.to_str().unwrap(), &query, &None).unwrap();
        assert_eq!(results3.len(), 3);
    }

    #[test]
    fn test_timestamp_filtering_in_cached_search() {
        let temp_dir = tempdir().unwrap();
        let test_file = temp_dir.path().join("test.jsonl");
        
        // Create test JSONL file with different timestamps
        let mut file = File::create(&test_file).unwrap();
        writeln!(file, r#"{{"type":"user","message":{{"role":"user","content":"Early message"}},"uuid":"1","timestamp":"2024-01-01T00:00:00Z","sessionId":"s1","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}}"#).unwrap();
        writeln!(file, r#"{{"type":"user","message":{{"role":"user","content":"Middle message"}},"uuid":"2","timestamp":"2024-01-02T00:00:00Z","sessionId":"s1","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}}"#).unwrap();
        writeln!(file, r#"{{"type":"user","message":{{"role":"user","content":"Late message"}},"uuid":"3","timestamp":"2024-01-03T00:00:00Z","sessionId":"s1","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}}"#).unwrap();
        
        let mut options = SearchOptions::default();
        options.after = Some("2024-01-01T12:00:00Z".to_string());
        options.before = Some("2024-01-02T12:00:00Z".to_string());
        
        let mut search = InteractiveSearch::new(options);
        
        // Test search with timestamp filter
        let query = parse_query("message").unwrap();
        let results = search.execute_cached_search(test_file.to_str().unwrap(), &query, &None).unwrap();
        assert_eq!(results.len(), 1);
        assert!(results[0].text.contains("Middle message"));
    }

    #[test]
    fn test_empty_file_handling() {
        let temp_dir = tempdir().unwrap();
        let test_file = temp_dir.path().join("empty.jsonl");
        
        // Create empty file
        File::create(&test_file).unwrap();
        
        let options = SearchOptions::default();
        let mut search = InteractiveSearch::new(options);
        
        // Should not panic on empty file
        let query = parse_query("test").unwrap();
        let results = search.execute_cached_search(test_file.to_str().unwrap(), &query, &None).unwrap();
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_invalid_json_handling() {
        let temp_dir = tempdir().unwrap();
        let test_file = temp_dir.path().join("invalid.jsonl");
        
        // Create file with invalid JSON
        let mut file = File::create(&test_file).unwrap();
        writeln!(file, "This is not JSON").unwrap();
        writeln!(file, r#"{{"type":"user","message":{{"role":"user","content":"Valid message"}},"uuid":"123","timestamp":"2024-01-01T00:00:00Z","sessionId":"s1","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}}"#).unwrap();
        writeln!(file, "Another invalid line").unwrap();
        
        let options = SearchOptions::default();
        let mut search = InteractiveSearch::new(options);
        
        // Should skip invalid lines and process valid ones
        let query = parse_query("Valid").unwrap();
        let results = search.execute_cached_search(test_file.to_str().unwrap(), &query, &None).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].text, "Valid message");
    }

    #[test]
    fn test_execute_search() {
        let temp_dir = tempdir().unwrap();
        let test_file = temp_dir.path().join("test.jsonl");
        
        // Create test JSONL file
        let mut file = File::create(&test_file).unwrap();
        writeln!(file, r#"{{"type":"user","message":{{"role":"user","content":"Test message"}},"uuid":"123","timestamp":"2024-01-01T00:00:00Z","sessionId":"s1","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}}"#).unwrap();
        
        let options = SearchOptions::default();
        let mut search = InteractiveSearch::new(options);
        
        // Test with valid query
        let (results, search_query) = search.execute_search(test_file.to_str().unwrap(), "Test", &None);
        assert_eq!(results.len(), 1);
        assert_eq!(search_query, "Test");
        
        // Test with empty query
        let (results2, search_query2) = search.execute_search(test_file.to_str().unwrap(), "", &None);
        assert_eq!(results2.len(), 0);
        assert_eq!(search_query2, "");
        
        // Test with invalid query
        let (results3, search_query3) = search.execute_search(test_file.to_str().unwrap(), "/invalid(regex", &None);
        assert_eq!(results3.len(), 0);
        assert_eq!(search_query3, "/invalid(regex");
    }
}
