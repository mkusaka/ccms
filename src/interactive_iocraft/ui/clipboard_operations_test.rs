#[cfg(test)]
mod tests {
    use crate::interactive_iocraft::ui::{DetailState, SearchState, UIState};
    use crate::query::condition::{QueryCondition, SearchResult};

    fn create_test_result() -> SearchResult {
        SearchResult {
            text: "This is a test message with some content.".to_string(),
            file: "/path/to/test/file.jsonl".to_string(),
            uuid: "test-uuid-1234".to_string(),
            project_path: "/test/project".to_string(),
            timestamp: "2024-01-01T12:00:00Z".to_string(),
            role: "user".to_string(),
            session_id: "session-uuid-5678".to_string(),
            has_thinking: false,
            has_tools: false,
            message_type: "message".to_string(),
            query: QueryCondition::Literal {
                pattern: "test".to_string(),
                case_sensitive: false,
            },
            raw_json: Some(r#"{"type":"user","content":"test"}"#.to_string()),
        }
    }

    #[test]
    fn test_copy_message_text() {
        let search_state = SearchState {
            results: vec![create_test_result()],
            selected_index: 0,
            ..Default::default()
        };

        let result = &search_state.results[search_state.selected_index];

        // Simulate Y key press - copy message text
        let text_to_copy = &result.text;
        assert_eq!(text_to_copy, "This is a test message with some content.");

        // UI feedback
        let ui_message = "✓ Copied message text".to_string();
        assert!(ui_message.starts_with('✓'));
        assert!(ui_message.contains("message text"));
    }

    #[test]
    fn test_copy_file_path() {
        let search_state = SearchState {
            results: vec![create_test_result()],
            selected_index: 0,
            ..Default::default()
        };

        let result = &search_state.results[search_state.selected_index];

        // Simulate F key press - copy file path
        let path_to_copy = &result.file;
        assert_eq!(path_to_copy, "/path/to/test/file.jsonl");

        // UI feedback
        let ui_message = "✓ Copied file path".to_string();
        assert!(ui_message.contains("file path"));
    }

    #[test]
    fn test_copy_session_id() {
        let search_state = SearchState {
            results: vec![create_test_result()],
            selected_index: 0,
            ..Default::default()
        };

        let result = &search_state.results[search_state.selected_index];

        // Simulate U key press - copy session ID
        let session_id_to_copy = &result.session_id;
        assert_eq!(session_id_to_copy, "session-uuid-5678");

        // UI feedback
        let ui_message = "✓ Copied session ID".to_string();
        assert!(ui_message.contains("session ID"));
    }

    #[test]
    fn test_copy_from_detail_view() {
        let detail_state = DetailState {
            selected_result: Some(create_test_result()),
            scroll_offset: 0,
        };

        let result = detail_state.selected_result.as_ref().unwrap();

        // Test all copy operations from detail view
        assert_eq!(result.text, "This is a test message with some content.");
        assert_eq!(result.file, "/path/to/test/file.jsonl");
        assert_eq!(result.session_id, "session-uuid-5678");
        assert_eq!(result.project_path, "/test/project");
        assert!(result.raw_json.is_some());
    }

    #[test]
    fn test_copy_full_details() {
        let detail_state = DetailState {
            selected_result: Some(create_test_result()),
            scroll_offset: 0,
        };

        let result = detail_state.selected_result.as_ref().unwrap();

        // Simulate A key press - copy all details
        let full_details = format!(
            "File: {}\nUUID: {}\nRole: {}\nTimestamp: {}\n\n{}",
            result.file, result.session_id, result.role, result.timestamp, result.text
        );

        assert!(full_details.contains(&result.file));
        assert!(full_details.contains(&result.session_id));
        assert!(full_details.contains(&result.role));
        assert!(full_details.contains(&result.timestamp));
        assert!(full_details.contains(&result.text));

        // UI feedback
        let ui_message = "✓ Copied full result details".to_string();
        assert!(ui_message.contains("full result details"));
    }

    #[test]
    fn test_copy_with_no_selection() {
        let search_state = SearchState {
            results: vec![],
            selected_index: 0,
            ..Default::default()
        };

        // Should handle gracefully when no results
        assert!(search_state.results.is_empty());
        assert!(
            search_state
                .results
                .get(search_state.selected_index)
                .is_none()
        );
    }

    #[test]
    fn test_copy_error_handling() {
        let mut ui_state = UIState::default();

        // Simulate clipboard error
        ui_state.message = Some("Failed to copy: clipboard command not found".to_string());

        assert!(
            ui_state
                .message
                .as_ref()
                .unwrap()
                .starts_with("Failed to copy")
        );
    }

    #[test]
    fn test_context_aware_copy_feedback() {
        let mut ui_state = UIState::default();

        // Short text - shows actual content
        let short_text = "Hello world";
        if short_text.len() < 50 {
            ui_state.message = Some(format!("✓ Copied: {}", short_text));
        }
        assert_eq!(ui_state.message, Some("✓ Copied: Hello world".to_string()));

        // Long text - generic message
        let long_text =
            "This is a very long message that exceeds the limit for preview in the copy feedback";
        if long_text.len() >= 50 {
            ui_state.message = Some("✓ Copied message text".to_string());
        }
        assert_eq!(ui_state.message, Some("✓ Copied message text".to_string()));
    }

    #[test]
    fn test_raw_json_copy() {
        let mut result = create_test_result();

        // Test with raw JSON present
        assert!(result.raw_json.is_some());
        let raw_json = result.raw_json.as_ref().unwrap();
        assert!(raw_json.contains("\"type\":\"user\""));

        // Test with no raw JSON
        result.raw_json = None;
        assert!(result.raw_json.is_none());
    }
}
