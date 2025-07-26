#[cfg(test)]
mod tests {
    use crate::interactive_ratatui::ui::components::{Component, result_list::ResultList};
    use crate::query::condition::SearchResult;
    use ratatui::{Terminal, backend::TestBackend, buffer::Buffer};

    #[test]
    fn test_list_rendering_exact_issue() {
        // Test with exact terminal width where issue occurs
        let backend = TestBackend::new(122, 15); // Matching user's terminal width
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|f| {
                let area = f.area();
                println!("\nTerminal area: width={}, height={}", area.width, area.height);

                let mut result_list = ResultList::new();

                // Exact data from user's screen
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
                        text: "".to_string(),
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

        println!("\nChecking for rendering issue:");
        
        // Look for the exact pattern
        for y in 0..15 {
            let line = extract_line(buffer, y, 122);
            
            // Check each line character by character
            if line.contains("│") {
                println!("\nLine {}: '{}'", y, line);
                
                // Look for the specific pattern at the end
                if line.len() >= 10 {
                    let end_part = &line[line.len()-10..];
                    println!("  Last 10 chars: '{}'", end_part);
                    
                    if end_part.contains("│        │") {
                        println!("  → FOUND THE ISSUE: '│        │' pattern at end!");
                    }
                }
                
                // Count vertical bars
                let bar_count = line.matches('│').count();
                println!("  Number of '│' characters: {}", bar_count);
                
                // Check if line ends with extra content
                if line.ends_with("│        │") {
                    println!("  → ERROR: Line ends with '│        │'");
                } else if line.trim_end() != line {
                    println!("  → Line has {} trailing spaces", line.len() - line.trim_end().len());
                }
            }
        }

        // Check buffer content beyond the expected width
        println!("\nChecking content beyond expected width:");
        for y in 4..8 {
            let mut beyond_content = String::new();
            for x in 110..122 {
                let cell = &buffer[(x, y)];
                beyond_content.push_str(cell.symbol());
            }
            if !beyond_content.trim().is_empty() {
                println!("Line {} beyond x=110: '{}'", y, beyond_content);
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