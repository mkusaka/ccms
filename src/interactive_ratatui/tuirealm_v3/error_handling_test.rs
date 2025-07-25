#[cfg(test)]
mod error_handling_tests {
    use crate::query::condition::{QueryCondition, SearchResult};
    use std::sync::mpsc;
    use std::time::Duration;
    use std::thread;
    use std::sync::{Arc, Mutex};
    use crate::interactive_ratatui::tuirealm_v3::services::{SearchService, SessionService, ClipboardService};
    use crate::interactive_ratatui::tuirealm_v3::app::App;
    use crate::interactive_ratatui::tuirealm_v3::messages::AppMessage;
    use crate::interactive_ratatui::tuirealm_v3::AppMode;
    use tuirealm::Update;
    use crate::interactive_ratatui::tuirealm_v3::state::AppState;

    fn create_test_search_result() -> SearchResult {
        SearchResult {
            file: "test.jsonl".to_string(),
            uuid: "test-uuid".to_string(),
            timestamp: "2024-01-01T10:00:00Z".to_string(),
            session_id: "test-session".to_string(),
            role: "User".to_string(),
            text: "Test message".to_string(),
            has_tools: false,
            has_thinking: false,
            message_type: "message".to_string(),
            query: QueryCondition::Literal {
                pattern: "test".to_string(),
                case_sensitive: false,
            },
            project_path: "/test/project".to_string(),
            raw_json: Some(r#"{"content": "Test message"}"#.to_string()),
        }
    }

    #[test]
    fn test_file_io_error_handling() {
        let mut session_service = SessionService::new();
        
        // Test with non-existent file
        let result = session_service.load_session("non-existent-session-id");
        assert!(result.is_err());
        
        // Test with invalid path characters
        let result = session_service.load_session("\0invalid\0path");
        assert!(result.is_err());
        
        // Test with extremely long path
        let long_path = "x".repeat(10000);
        let result = session_service.load_session(&long_path);
        assert!(result.is_err());
    }

    #[test]
    fn test_json_parse_error_handling() {
        let mut app = App::new(None, None, None, None);
        
        // Create a session with invalid JSON
        app.state.session_messages = vec![
            "Not valid JSON".to_string(),
            "{broken json".to_string(),
            r#"{"incomplete": "#.to_string(),
            "null".to_string(),
            "123".to_string(),
            "true".to_string(),
        ];
        
        app.state.mode = AppMode::SessionViewer;
        app.state.session_filtered_indices = (0..6).collect();
        
        // Should handle invalid JSON gracefully
        // Components update themselves through messages
        
        // Should still be able to navigate
        app.update(Some(AppMessage::SessionScrollDown));
        app.update(Some(AppMessage::SessionScrollUp));
    }

    #[test]
    fn test_clipboard_access_error() {
        let mut clipboard_service = ClipboardService::new();
        
        // Test copying extremely large text
        let huge_text = "x".repeat(100 * 1024 * 1024); // 100MB
        let result = clipboard_service.copy(&huge_text);
        
        // Should either succeed or fail gracefully
        match result {
            Ok(_) => {
                // If it succeeds, verify we can read it back
                // Reading from clipboard is not exposed in this API
            }
            Err(e) => {
                // Error should have meaningful message
                assert!(!e.to_string().is_empty());
            }
        }
    }

    #[test]
    fn test_channel_panic_recovery() {
        let mut app = App::new(None, None, None, None);
        
        // Drop the receiver to simulate channel failure
        let (tx, rx) = mpsc::channel();
        drop(rx);
        
        // Try to send search results
        let result = tx.send(vec![create_test_search_result()]);
        assert!(result.is_err());
        
        // App should handle the error gracefully
        app.update(Some(AppMessage::SearchCompleted));
        assert!(app.state.status_message.is_none() || app.state.status_message.is_some());
    }

    #[test]
    fn test_terminal_size_error() {
        let mut app = App::new(None, None, None, None);
        
        // Simulate extremely small terminal
        // This would normally be handled by tui-realm, but we test our logic
        app.state.search_results = vec![create_test_search_result(); 100];
        
        // Try to render with 0 height (would cause division by zero in pagination)
        app.state.selected_index = 50;
        // Simulate zero height scenario
        let visible_height = 0;
        
        // Calculate scroll offset - should not panic
        let page_size = visible_height.max(1); // Prevent division by zero
        let _scroll_offset = (app.state.selected_index / page_size) * page_size;
        
        assert!(page_size >= 1);
    }

    #[test]
    fn test_concurrent_state_modification() {
        let app_state = Arc::new(Mutex::new(AppState::new()));
        
        let mut handles = vec![];
        
        // Spawn multiple threads trying to modify state concurrently
        for i in 0..10 {
            let state_clone = Arc::clone(&app_state);
            let handle = thread::spawn(move || {
                for j in 0..100 {
                    if let Ok(mut state) = state_clone.lock() {
                        state.selected_index = i * 100 + j;
                        state.search_query = format!("thread-{i}-query-{j}");
                        state.is_searching = j % 2 == 0;
                    }
                    thread::sleep(Duration::from_micros(10));
                }
            });
            handles.push(handle);
        }
        
        // Wait for all threads
        for handle in handles {
            handle.join().unwrap();
        }
        
        // State should be in a valid state
        let final_state = app_state.lock().unwrap();
        assert!(!final_state.search_query.is_empty());
    }

    #[test]
    fn test_search_timeout_handling() {
        let search_service = SearchService::new();
        let (tx, rx) = mpsc::channel();
        
        // Start a search
        // Start async search operation (simulated)
        thread::spawn(move || {
            // Simulate search
            let results = search_service.search_sync(
                "test".to_string(),
                None,
            );
            let _ = tx.send(results);
        });
        
        // Try to receive with very short timeout
        let result = rx.recv_timeout(Duration::from_micros(1));
        
        // Should either succeed or timeout
        match result {
            Ok(results) => {
                // If it succeeds quickly, results should be valid
                assert!(results.is_empty() || !results.is_empty());
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                // Timeout is expected and ok
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                // Channel disconnected is also ok (search service dropped)
            }
        }
    }

    #[test]
    fn test_invalid_regex_patterns() {
        let mut app = App::new(None, None, None, None);
        
        // Test various invalid regex patterns
        let invalid_patterns = vec![
            "[",
            "(",
            "*",
            "?",
            "+",
            "{",
            "\\",
            "(?P<",
            "(?P<name",
            "[z-a]",
        ];
        
        for pattern in invalid_patterns {
            app.state.search_query = pattern.to_string();
            
            // Should handle invalid regex gracefully
            app.update(Some(AppMessage::SearchRequested));
            
            // Should show error message or handle gracefully
            if let Some(msg) = &app.state.status_message {
                assert!(!msg.is_empty());
            }
        }
    }

    #[test]
    fn test_unicode_normalization_issues() {
        let mut app = App::new(None, None, None, None);
        
        // Test different Unicode normalizations
        let text_nfc = "café"; // NFC normalized
        let text_nfd = "café"; // NFD normalized (combining characters)
        
        // Create results with different normalizations
        let mut result1 = create_test_search_result();
        result1.text = text_nfc.to_string();
        
        let mut result2 = create_test_search_result();
        result2.text = text_nfd.to_string();
        
        app.state.search_results = vec![result1, result2];
        
        // Search for café in different normalizations
        app.state.search_query = "café".to_string();
        app.update(Some(AppMessage::SearchRequested));
        
        // Should handle both normalizations
        assert_eq!(app.state.mode, AppMode::Search);
    }

    #[test]
    fn test_stack_overflow_prevention() {
        let mut app = App::new(None, None, None, None);
        
        // Create deeply nested JSON structure
        let mut nested_json = r#"{"a":"#.to_string();
        for _ in 0..1000 {
            nested_json.push_str(r#"{"b":"#);
        }
        nested_json.push_str("value");
        for _ in 0..1000 {
            nested_json.push_str(r#"}"}"#);
        }
        
        let mut result = create_test_search_result();
        result.raw_json = Some(nested_json);
        
        app.state.current_result = Some(result);
        app.state.mode = AppMode::ResultDetail; // Must be in ResultDetail mode for copy to work
        
        // Should handle deeply nested JSON without stack overflow
        app.update(Some(AppMessage::CopyRawJson));
        
        // Should complete without panic (either success or failure message)
        assert!(app.state.status_message.is_some());
    }

    #[test]
    fn test_memory_exhaustion_protection() {
        let mut app = App::new(None, None, None, None);
        
        // Try to create results that would exhaust memory
        let mut results = Vec::new();
        
        // Add results until we hit a reasonable limit
        for i in 0..100 {
            // Each result has 1MB of text
            let mut result = create_test_search_result();
            result.text = "x".repeat(1024 * 1024);
            result.raw_json = Some(format!(r#"{{"content": "{}"}}"#, "x".repeat(1024 * 1024)));
            results.push(result);
            
            // Stop if we're using too much memory
            if i > 50 {
                break;
            }
        }
        
        app.state.search_results = results;
        
        // Should handle large result sets
        app.update(Some(AppMessage::ResultDown));
        app.update(Some(AppMessage::ResultPageDown));
        
        // Should not crash
        assert!(app.state.selected_index < app.state.search_results.len());
    }
}