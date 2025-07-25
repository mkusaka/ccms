#[cfg(test)]
mod tests {
    use crate::interactive_ratatui::ui::components::{Component, result_list::ResultList};
    use crate::query::condition::SearchResult;
    use ratatui::{Terminal, backend::TestBackend, buffer::Buffer};

    #[test]
    fn test_exact_border_pattern_reproduction() {
        // Test various terminal widths to see if the issue appears
        let widths = vec![80, 100, 120, 140, 160];

        for width in widths {
            println!("\n=== Testing with width {} ===", width);

            let backend = TestBackend::new(width, 20);
            let mut terminal = Terminal::new(backend).unwrap();

            terminal.draw(|f| {
                let area = f.area();
                
                let mut result_list = ResultList::new();
                
                // Create system messages that might trigger the issue
                let items = vec![
                    SearchResult {
                        file: "/Users/foo/projects/test/file.jsonl".to_string(),
                        uuid: "msg_01234567890".to_string(),
                        timestamp: "2024-07-25T18:22:45.123Z".to_string(),
                        session_id: "session-123".to_string(),
                        role: "system".to_string(),
                        text: "PostToolUse:Edit [ccth --debug] completed successfully: [dotenv@17.2.1] injecting env (0) from .env".to_string(),
                        has_tools: false,
                        has_thinking: false,
                        message_type: "normal".to_string(),
                        query: crate::query::condition::QueryCondition::Literal { 
                            pattern: "test".to_string(), 
                            case_sensitive: false 
                        },
                        project_path: "/test/project".to_string(),
                        raw_json: Some("{}".to_string()),
                    },
                    SearchResult {
                        file: "/Users/foo/projects/test/file.jsonl".to_string(),
                        uuid: "msg_98765432100".to_string(),
                        timestamp: "2024-07-25T18:22:46.456Z".to_string(),
                        session_id: "session-123".to_string(),
                        role: "system".to_string(),
                        text: "Running PostToolUse:Edit...".to_string(),
                        has_tools: false,
                        has_thinking: false,
                        message_type: "normal".to_string(),
                        query: crate::query::condition::QueryCondition::Literal { 
                            pattern: "test".to_string(), 
                            case_sensitive: false 
                        },
                        project_path: "/test/project".to_string(),
                        raw_json: Some("{}".to_string()),
                    },
                    SearchResult {
                        file: "/Users/foo/projects/test/file.jsonl".to_string(),
                        uuid: "msg_11111111111".to_string(),
                        timestamp: "2024-07-25T18:22:47.789Z".to_string(),
                        session_id: "session-123".to_string(),
                        role: "system".to_string(),
                        text: "Preparing command execution...".to_string(),
                        has_tools: false,
                        has_thinking: false,
                        message_type: "normal".to_string(),
                        query: crate::query::condition::QueryCondition::Literal { 
                            pattern: "test".to_string(), 
                            case_sensitive: false 
                        },
                        project_path: "/test/project".to_string(),
                        raw_json: Some("{}".to_string()),
                    },
                ];
                result_list.set_results(items);
                result_list.render(f, area);
            }).unwrap();

            let buffer = terminal.backend().buffer();

            // Look for the specific pattern
            let mut found_issue = false;
            for y in 0..20 {
                let line = extract_full_line(buffer, y, width);

                // Check for the exact pattern the user reported
                if line.contains("│        │") {
                    found_issue = true;
                    println!("FOUND ISSUE at line {}: {}", y, line);

                    // Print surrounding lines for context
                    if y > 0 {
                        println!("  prev: {}", extract_full_line(buffer, y - 1, width));
                    }
                    println!("  curr: {}", line);
                    if y < 19 {
                        println!("  next: {}", extract_full_line(buffer, y + 1, width));
                    }
                }

                // Also check for other potential double border patterns
                if line.contains("││") || line.contains("┤│") || line.contains("│├") {
                    println!("Found other border issue at line {}: {}", y, line);
                }
            }

            if found_issue {
                // Print the full render for debugging
                println!("\nFull render output:");
                for y in 0..20 {
                    let line = extract_full_line(buffer, y, width);
                    println!("{:2}: {}", y, line);
                }

                panic!("Found the exact '│        │' pattern at width {}", width);
            }
        }
    }

    #[test]
    fn test_table_widget_edge_cases() {
        // Test edge cases that might cause the issue
        let backend = TestBackend::new(120, 25);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|f| {
                let area = f.area();

                let mut result_list = ResultList::new();

                // Create messages with various content lengths
                let mut items = Vec::new();

                // Very short content
                items.push(create_test_result("1", "system", "OK"));

                // Empty content
                items.push(create_test_result("2", "system", ""));

                // Content with special characters
                items.push(create_test_result("3", "system", "Test │ with │ pipes"));

                // Very long content that needs truncation
                items.push(create_test_result("4", "system", &"x".repeat(200)));

                // Content with unicode
                items.push(create_test_result("5", "system", "日本語 テスト 🎌"));

                // Whitespace-only content
                items.push(create_test_result("6", "system", "     "));

                result_list.set_results(items);
                result_list.render(f, area);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();

        println!("\nEdge case test output:");
        for y in 0..25 {
            let line = extract_full_line(buffer, y, 120);
            if !line.trim().is_empty() {
                println!("{:2}: {}", y, line);
            }
        }

        // Check for issues
        for y in 0..25 {
            let line = extract_full_line(buffer, y, 120);
            assert!(
                !line.contains("│        │"),
                "Found double border pattern at line {}",
                y
            );
        }
    }

    fn create_test_result(id: &str, role: &str, text: &str) -> SearchResult {
        SearchResult {
            file: format!("/test/file{}.jsonl", id),
            uuid: format!("msg_{}", id),
            timestamp: "2024-07-25T18:22:00Z".to_string(),
            session_id: "test-session".to_string(),
            role: role.to_string(),
            text: text.to_string(),
            has_tools: false,
            has_thinking: false,
            message_type: "normal".to_string(),
            query: crate::query::condition::QueryCondition::Literal {
                pattern: "test".to_string(),
                case_sensitive: false,
            },
            project_path: "/test/project".to_string(),
            raw_json: Some("{}".to_string()),
        }
    }

    fn extract_full_line(buffer: &Buffer, y: u16, width: u16) -> String {
        let mut line = String::new();
        for x in 0..width {
            let cell = &buffer[(x, y)];
            line.push_str(cell.symbol());
        }
        line
    }
}
