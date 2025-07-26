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
    async fn test_render_layout_structure() {
        let (app, state) = create_test_app();
        
        let mut state_lock = state.lock().await;
        state_lock.query = "test".to_string();
        
        // Add 5 results
        for i in 1..=5 {
            state_lock.search_results.push(create_test_result(
                i,
                if i % 2 == 0 { "assistant" } else { "user" },
                &format!("Message content {} with some text", i)
            ));
        }
        
        let output = app.render(&mut state_lock).await.unwrap();
        
        // Should start with clear screen
        assert!(output.starts_with("\x1b[2J\x1b[H"));
        
        // Count cursor positioning commands
        let position_count = output.matches(";1H").count();
        
        // We should have many positioning commands for proper layout
        assert!(position_count > 10, "Not enough cursor positioning commands: {}", position_count);
        
        // Check that each line is properly cleared
        let clear_count = output.matches("\x1b[K").count();
        
        assert!(clear_count > 5, "Not enough line clear commands: {}", clear_count);
        
        // Verify content appears after proper positioning
        assert!(output.contains("CCMS Search (R3BL TUI)"));
        assert!(output.contains("Search:"));
        assert!(output.contains("Results: 5 found"));
        
        // Print output for debugging
        eprintln!("=== Rendered Output ===");
        eprintln!("{}", output.replace('\x1b', "ESC"));
        eprintln!("=== End Output ===");
    }

    #[tokio::test]
    async fn test_no_overlapping_lines() {
        let (app, state) = create_test_app();
        
        let mut state_lock = state.lock().await;
        state_lock.query = "async".to_string();
        
        // Add many results to test scrolling
        for i in 1..=50 {
            state_lock.search_results.push(create_test_result(
                i,
                "user",
                &format!("Test message {}: This is a longer message that should be truncated properly without overlapping", i)
            ));
        }
        
        let output = app.render(&mut state_lock).await.unwrap();
        
        // Extract line positions
        let mut line_positions = Vec::new();
        let parts: Vec<&str> = output.split("\x1b[").collect();
        
        for part in parts {
            if let Some(pos) = part.find(";1H") {
                if let Ok(line_num) = part[..pos].parse::<usize>() {
                    line_positions.push(line_num);
                }
            }
        }
        
        // Verify lines are in order and not overlapping
        for i in 1..line_positions.len() {
            assert!(
                line_positions[i] >= line_positions[i-1],
                "Line {} comes before line {}", 
                line_positions[i], 
                line_positions[i-1]
            );
        }
    }
}