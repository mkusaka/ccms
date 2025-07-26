#[cfg(test)]
mod tests {
    use super::super::app::*;
    use super::super::state::*;
    use crate::{SearchOptions, SearchResult};
    use crate::query::QueryCondition;
    use std::sync::Arc;
    use tokio::sync::Mutex;

    fn create_test_app() -> (SearchApp, Arc<Mutex<AppState>>) {
        let state = Arc::new(Mutex::new(AppState::new()));
        let options = SearchOptions::default();
        let app = SearchApp::new("test.jsonl".to_string(), options, state.clone());
        (app, state)
    }

    fn create_test_result(id: u32, role: &str, text: &str) -> SearchResult {
        SearchResult {
            file: format!("test{}.jsonl", id),
            uuid: format!("uuid-{}", id),
            timestamp: "2024-01-01T00:00:00Z".to_string(),
            session_id: format!("session-{}", id),
            role: role.to_string(),
            text: text.to_string(),
            has_tools: false,
            has_thinking: false,
            message_type: "text".to_string(),
            query: QueryCondition::Literal {
                pattern: "test".to_string(),
                case_sensitive: false,
            },
            project_path: "/test".to_string(),
            raw_json: None,
        }
    }

    #[tokio::test]
    async fn test_render_search_view() {
        let (app, state) = create_test_app();
        
        let mut state_lock = state.lock().await;
        state_lock.query = "test query".to_string();
        state_lock.search_results.push(create_test_result(1, "user", "Hello world"));
        state_lock.search_results.push(create_test_result(2, "assistant", "Response"));
        
        let output = app.render(&mut state_lock).await.unwrap();
        
        // Check that output contains expected elements with proper ANSI codes
        assert!(output.contains("\x1b[2J\x1b[H")); // Clear screen
        assert!(output.contains("\x1b[1;1H")); // Position cursor at line 1
        assert!(output.contains("\x1b[K")); // Clear line
        assert!(output.contains("CCMS Search (R3BL TUI)"));
        assert!(output.contains("Search:"));
        assert!(output.contains("test query"));
        assert!(output.contains("Results: 2 found"));
        assert!(output.contains("[user]"));
        assert!(output.contains("Hello world"));
        assert!(output.contains("[assistant]"));
        assert!(output.contains("Response"));
        
        // Check for proper cursor positioning
        assert!(output.contains("\x1b[3;1H")); // Search bar position
        assert!(output.contains("\x1b[5;1H")); // Results count position
    }

    #[tokio::test]
    async fn test_render_help_view() {
        let (app, state) = create_test_app();
        
        let mut state_lock = state.lock().await;
        state_lock.current_mode = ViewMode::Help;
        
        let output = app.render(&mut state_lock).await.unwrap();
        
        // Check help content
        assert!(output.contains("CCMS Help"));
        assert!(output.contains("Navigation:"));
        assert!(output.contains("↑/k      Move up"));
        assert!(output.contains("↓/j      Move down"));
        assert!(output.contains("?        Show this help"));
        assert!(output.contains("q        Quit application"));
    }

    #[tokio::test]
    async fn test_render_detail_view() {
        let (app, state) = create_test_app();
        
        let mut state_lock = state.lock().await;
        state_lock.search_results.push(create_test_result(1, "user", "Detailed message content\nWith multiple lines\nOf text"));
        state_lock.current_mode = ViewMode::ResultDetail;
        state_lock.selected_index = 0;
        
        let output = app.render(&mut state_lock).await.unwrap();
        
        // Check detail view content
        assert!(output.contains("Message Detail"));
        assert!(output.contains("Role: user"));
        assert!(output.contains("Timestamp: 2024-01-01T00:00:00Z"));
        assert!(output.contains("Session: session-1"));
        assert!(output.contains("Detailed message content"));
        assert!(output.contains("With multiple lines"));
    }

    #[tokio::test]
    async fn test_handle_input_quit() {
        let (mut app, state) = create_test_app();
        
        let mut state_lock = state.lock().await;
        let should_exit = app.handle_input('q', &mut state_lock).await.unwrap();
        
        assert!(should_exit);
    }

    #[tokio::test]
    async fn test_handle_input_help() {
        let (mut app, state) = create_test_app();
        
        let mut state_lock = state.lock().await;
        let should_exit = app.handle_input('?', &mut state_lock).await.unwrap();
        
        assert!(!should_exit);
        assert_eq!(state_lock.current_mode, ViewMode::Help);
    }

    #[tokio::test]
    async fn test_handle_input_navigation() {
        let (mut app, state) = create_test_app();
        
        let mut state_lock = state.lock().await;
        state_lock.search_results.push(create_test_result(1, "user", "First"));
        state_lock.search_results.push(create_test_result(2, "assistant", "Second"));
        
        // Test 'j' (down)
        app.handle_input('j', &mut state_lock).await.unwrap();
        assert_eq!(state_lock.selected_index, 1);
        
        // Test 'k' (up)
        app.handle_input('k', &mut state_lock).await.unwrap();
        assert_eq!(state_lock.selected_index, 0);
    }

    #[tokio::test]
    async fn test_handle_input_enter() {
        let (mut app, state) = create_test_app();
        
        let mut state_lock = state.lock().await;
        state_lock.search_results.push(create_test_result(1, "user", "Message"));
        
        // Press Enter to view detail
        app.handle_input('\n', &mut state_lock).await.unwrap();
        assert_eq!(state_lock.current_mode, ViewMode::ResultDetail);
    }

    #[tokio::test]
    async fn test_handle_input_escape() {
        let (mut app, state) = create_test_app();
        
        let mut state_lock = state.lock().await;
        
        // From help view
        state_lock.current_mode = ViewMode::Help;
        app.handle_input('\x1b', &mut state_lock).await.unwrap();
        assert_eq!(state_lock.current_mode, ViewMode::Search);
        
        // From detail view
        state_lock.current_mode = ViewMode::ResultDetail;
        app.handle_input('\x1b', &mut state_lock).await.unwrap();
        assert_eq!(state_lock.current_mode, ViewMode::Search);
    }

    #[tokio::test]
    async fn test_handle_input_search() {
        let (mut app, state) = create_test_app();
        
        let mut state_lock = state.lock().await;
        
        // Type 'h'
        app.handle_input('h', &mut state_lock).await.unwrap();
        assert_eq!(state_lock.query, "h");
        assert!(state_lock.is_searching);
        
        // Type 'e'
        app.handle_input('e', &mut state_lock).await.unwrap();
        assert_eq!(state_lock.query, "he");
        
        // Backspace
        app.handle_input('\x08', &mut state_lock).await.unwrap();
        assert_eq!(state_lock.query, "h");
        
        // Ctrl+U (clear)
        app.handle_input('\x15', &mut state_lock).await.unwrap();
        assert_eq!(state_lock.query, "");
    }

    #[tokio::test]
    async fn test_process_signals() {
        let (mut app, state) = create_test_app();
        
        // Send search completed signal
        let results = vec![
            create_test_result(1, "user", "Result 1"),
            create_test_result(2, "assistant", "Result 2"),
        ];
        app.search_tx.send(SearchSignal::SearchCompleted(results)).await.unwrap();
        
        let mut state_lock = state.lock().await;
        app.process_signals(&mut state_lock).await.unwrap();
        
        assert_eq!(state_lock.search_results.len(), 2);
        assert!(!state_lock.is_searching);
        assert_eq!(state_lock.selected_index, 0);
        assert_eq!(state_lock.status_message.as_ref().unwrap(), "Found 2 results");
    }

    #[tokio::test]
    async fn test_process_error_signal() {
        let (mut app, state) = create_test_app();
        
        // Send error signal
        app.search_tx.send(SearchSignal::SearchError("Test error".to_string())).await.unwrap();
        
        let mut state_lock = state.lock().await;
        state_lock.is_searching = true;
        app.process_signals(&mut state_lock).await.unwrap();
        
        assert!(!state_lock.is_searching);
        assert_eq!(state_lock.status_message.as_ref().unwrap(), "Error: Test error");
    }

    #[tokio::test]
    async fn test_render_long_text_truncation() {
        let (app, state) = create_test_app();
        
        let mut state_lock = state.lock().await;
        let long_text = "This is a very long message that should be truncated because it exceeds the maximum width limit of 80 characters per line";
        state_lock.search_results.push(create_test_result(1, "user", long_text));
        
        let output = app.render(&mut state_lock).await.unwrap();
        
        // Check that the render completes without panic
        assert!(output.contains("\x1b[2J\x1b[H")); // Clear screen
        assert!(output.contains("Results: 1 found"));
        
        // The exact truncation depends on terminal width, but it should contain at least the beginning
        assert!(output.contains("This is a very long message"));
        
        // If truncated, it should have ellipsis
        if !output.contains(long_text) {
            assert!(output.contains("..."));
        }
    }

    #[tokio::test]
    async fn test_render_searching_state() {
        let (app, state) = create_test_app();
        
        let mut state_lock = state.lock().await;
        state_lock.query = "test".to_string();
        state_lock.is_searching = true;
        
        let output = app.render(&mut state_lock).await.unwrap();
        
        // Check that searching indicator is shown
        assert!(output.contains("[searching...]"));
    }
}