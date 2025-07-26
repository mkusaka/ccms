#[cfg(test)]
mod tests {
    use crate::interactive_ratatui::ui::components::{Component, result_list::ResultList};
    use crate::query::condition::SearchResult;
    use ratatui::{Terminal, backend::TestBackend, buffer::Buffer};

    #[test]
    fn test_list_widget_rendering() {
        // Test List widget rendering without trailing spaces
        let backend = TestBackend::new(80, 20);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|f| {
                let area = f.area();

                let mut result_list = ResultList::new();

                // Create test data with various content lengths
                let items = vec![
                    SearchResult {
                        file: "/test/file.jsonl".to_string(),
                        uuid: "msg_1".to_string(),
                        timestamp: "2025-07-25T19:32:00Z".to_string(),
                        session_id: "test-session".to_string(),
                        role: "system".to_string(),
                        text: "Short text".to_string(),
                        has_tools: false,
                        has_thinking: false,
                        message_type: "normal".to_string(),
                        query: crate::query::condition::QueryCondition::Literal {
                            pattern: "test".to_string(),
                            case_sensitive: false,
                        },
                        project_path: "/test/project".to_string(),
                        raw_json: Some("{}".to_string()),
                    },
                    SearchResult {
                        file: "/test/file.jsonl".to_string(),
                        uuid: "msg_2".to_string(),
                        timestamp: "2025-07-25T19:32:00Z".to_string(),
                        session_id: "test-session".to_string(),
                        role: "assistant".to_string(),
                        text: "".to_string(), // Empty content
                        has_tools: false,
                        has_thinking: false,
                        message_type: "normal".to_string(),
                        query: crate::query::condition::QueryCondition::Literal {
                            pattern: "test".to_string(),
                            case_sensitive: false,
                        },
                        project_path: "/test/project".to_string(),
                        raw_json: Some("{}".to_string()),
                    },
                    SearchResult {
                        file: "/test/file.jsonl".to_string(),
                        uuid: "msg_3".to_string(),
                        timestamp: "2025-07-25T19:32:00Z".to_string(),
                        session_id: "test-session".to_string(),
                        role: "user".to_string(),
                        text: "This is a longer message that should not have trailing spaces when rendered in List widget".to_string(),
                        has_tools: false,
                        has_thinking: false,
                        message_type: "normal".to_string(),
                        query: crate::query::condition::QueryCondition::Literal {
                            pattern: "test".to_string(),
                            case_sensitive: false,
                        },
                        project_path: "/test/project".to_string(),
                        raw_json: Some("{}".to_string()),
                    },
                ];
                
                result_list.set_results(items);
                result_list.render(f, area);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();

        println!("\nList widget rendering test:");
        
        // Analyze each line
        for y in 0..20 {
            let line = extract_line(buffer, y, 80);
            if line.contains("│") && !line.contains("─") && !line.contains("┌") && !line.contains("└") {
                println!("Data line {}: '{}'", y, line);
                
                // Check for trailing spaces
                let trimmed = line.trim_end();
                if line.len() > trimmed.len() {
                    let trailing = line.len() - trimmed.len();
                    println!("  → {} trailing spaces (line length: {}, trimmed: {})", 
                             trailing, line.len(), trimmed.len());
                    
                    // Show the last few characters
                    let end_chars: Vec<char> = line.chars().rev().take(10).collect();
                    println!("  → Last 10 chars: {:?}", end_chars);
                    
                    // Check if it ends with the problematic pattern
                    if line.ends_with("│        │") {
                        println!("  → WARNING: Found the '│        │' pattern!");
                    }
                } else {
                    println!("  → No trailing spaces!");
                }
            }
        }

        // Specifically check for the "│        │" pattern
        println!("\nChecking for '│        │' pattern:");
        for y in 0..20 {
            let line = extract_line(buffer, y, 80);
            if line.contains("│        │") {
                println!("Found pattern at line {}: '{}'", y, line);
            }
        }
    }

    fn extract_line(buffer: &Buffer, y: u16, width: u16) -> String {
        let mut line = String::new();
        for x in 0..width {
            let cell = &buffer[(x, y)];
            line.push_str(cell.symbol());
        }
        line
    }
}