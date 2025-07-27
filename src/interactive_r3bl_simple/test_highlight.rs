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

    fn create_test_result(text: &str) -> SearchResult {
        SearchResult {
            file: "test.jsonl".to_string(),
            uuid: "test-uuid".to_string(),
            timestamp: "2024-01-01T00:00:00Z".to_string(),
            session_id: "test-session".to_string(),
            role: "user".to_string(),
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
    async fn test_search_result_highlighting() {
        let (app, state) = create_test_app();
        let mut state_lock = state.lock().await;
        
        // Set search query
        state_lock.query = "hello".to_string();
        
        // Add results containing the search term
        state_lock.search_results.push(create_test_result("Hello world, this is a test"));
        state_lock.search_results.push(create_test_result("Another message with hello in it"));
        state_lock.search_results.push(create_test_result("No match here"));
        
        let output = app.render(&mut state_lock).await.unwrap();
        
        // Check that "Hello" and "hello" are highlighted with yellow background
        assert!(output.contains("\x1b[43;30mHello\x1b[0m"), "First 'Hello' should be highlighted");
        assert!(output.contains("\x1b[43;30mhello\x1b[0m"), "Second 'hello' should be highlighted");
        
        // The third result should not have any highlighting
        // Look for the line with " 3 [user] No match here"
        // The output contains ANSI escape codes, so we need to find the specific pattern
        let has_third_result_with_highlight = output.contains(" 3 [user] ") && 
            output.split(" 3 [user] ").nth(1)
                .map(|rest| rest.contains("\x1b[43;30m") && rest.starts_with("No match"))
                .unwrap_or(false);
        
        assert!(!has_third_result_with_highlight, "Third result 'No match here' should not be highlighted");
    }

    #[tokio::test]
    async fn test_japanese_highlighting() {
        let (app, state) = create_test_app();
        let mut state_lock = state.lock().await;
        
        // Set Japanese search query
        state_lock.query = "世界".to_string();
        
        // Add results with Japanese text
        state_lock.search_results.push(create_test_result("こんにちは世界、今日は良い天気です"));
        state_lock.search_results.push(create_test_result("世界平和を願っています"));
        
        let output = app.render(&mut state_lock).await.unwrap();
        
        // Check that "世界" is highlighted
        assert!(output.contains("\x1b[43;30m世界\x1b[0m"), "Japanese text should be highlighted");
        
        // Count occurrences
        let highlight_count = output.matches("\x1b[43;30m世界\x1b[0m").count();
        assert_eq!(highlight_count, 2, "Should highlight both occurrences of 世界");
    }

    #[tokio::test]
    async fn test_highlighting_with_truncation() {
        let (app, state) = create_test_app();
        let mut state_lock = state.lock().await;
        
        // Set search query
        state_lock.query = "important".to_string();
        
        // Add a long result that will be truncated
        let long_text = "This is a very long message with the word important somewhere in the middle that will definitely be truncated because it's too long to fit on one line";
        state_lock.search_results.push(create_test_result(long_text));
        
        let output = app.render(&mut state_lock).await.unwrap();
        
        // Check that highlighting is preserved even with truncation
        assert!(output.contains("\x1b[43;30mimportant\x1b[0m"), "Highlight should be preserved during truncation");
        assert!(output.contains("..."), "Truncated text should have ellipsis");
    }

    #[tokio::test]
    async fn test_empty_query_no_highlighting() {
        let (app, state) = create_test_app();
        let mut state_lock = state.lock().await;
        
        // Empty search query
        state_lock.query = "".to_string();
        
        // Add results
        state_lock.search_results.push(create_test_result("Hello world"));
        state_lock.search_results.push(create_test_result("Test message"));
        
        let output = app.render(&mut state_lock).await.unwrap();
        
        // No highlighting should be applied
        assert!(!output.contains("\x1b[43;30m"), "No highlighting with empty query");
    }
}