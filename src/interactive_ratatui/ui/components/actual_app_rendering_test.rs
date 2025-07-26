#[cfg(test)]
mod tests {
    use crate::interactive_ratatui::ui::components::{Component, result_list::ResultList};
    use crate::query::condition::SearchResult;
    use ratatui::{Terminal, backend::TestBackend, buffer::Buffer};

    #[test]
    fn test_actual_app_rendering_issue() {
        // Simulate the actual application rendering with realistic dimensions
        let backend = TestBackend::new(130, 30); // Wider terminal to see the issue
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|f| {
                let area = f.area();
                println!("Terminal area: width={}, height={}", area.width, area.height);

                let mut result_list = ResultList::new();

                // Create test data that matches the actual app
                let items = vec![
                    SearchResult {
                        file: "/test/file.jsonl".to_string(),
                        uuid: "msg_1".to_string(),
                        timestamp: "2025-07-26T06:41:00Z".to_string(),
                        session_id: "test-session".to_string(),
                        role: "system".to_string(),
                        text: "Stop [ccth --debug] completed successfully: [dotenv@17.2.1] injecting env (0) from ...".to_string(),
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
                        timestamp: "2025-07-26T06:41:00Z".to_string(),
                        session_id: "test-session".to_string(),
                        role: "assistant".to_string(),
                        text: "".to_string(), // Empty content - this shows the pattern clearly
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

        println!("\nActual app rendering issue test:");
        
        // Look for lines that might show the issue
        for y in 0..30 {
            let line = extract_line(buffer, y, 130);
            
            // Look for data lines (contain timestamp pattern)
            if line.contains("07/26") {
                println!("\nData line {}: '{}'", y, line);
                
                // Analyze the structure
                let parts: Vec<&str> = line.split('│').collect();
                println!("  Parts split by '│': {} parts", parts.len());
                for (i, part) in parts.iter().enumerate() {
                    println!("    Part {}: '{}' (len: {})", i, part, part.len());
                }
                
                // Check for the specific pattern
                if line.contains("│        │") {
                    println!("  → WARNING: Found '│        │' pattern!");
                    
                    // Find where it occurs
                    if let Some(pos) = line.find("│        │") {
                        println!("  → Pattern found at position: {}", pos);
                        println!("  → Context: '{}'", &line[pos.saturating_sub(10)..pos.saturating_add(20).min(line.len())]);
                    }
                }
            }
        }

        // Look specifically for empty rows
        println!("\nChecking empty assistant message:");
        for y in 0..30 {
            let line = extract_line(buffer, y, 130);
            if line.contains("assistant") && line.contains("│        │") {
                println!("Empty assistant line {}: '{}'", y, line);
                
                // Count the borders
                let border_count = line.matches('│').count();
                println!("  Number of '│' characters: {}", border_count);
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