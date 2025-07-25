#[cfg(test)]
mod tests {
    use crate::interactive_ratatui::ui::components::{Component, result_list::ResultList};
    use crate::query::condition::SearchResult;
    use ratatui::{Terminal, backend::TestBackend, buffer::Buffer};

    #[test]
    fn test_user_reported_double_bar_pattern() {
        // Test with different terminal widths to reproduce the issue
        let widths = vec![80, 100, 120, 130];

        for width in widths {
            println!("\n=== Testing with width {} ===", width);

            let backend = TestBackend::new(width, 20);
            let mut terminal = Terminal::new(backend).unwrap();

            terminal
                .draw(|f| {
                    let area = f.area();

                    let mut result_list = ResultList::new();

                    // Create test data matching user's exact report
                    let items = vec![
                        SearchResult {
                            file: "/test/file.jsonl".to_string(),
                            uuid: "msg_1".to_string(),
                            timestamp: "2025-07-25T19:19:00Z".to_string(),
                            session_id: "test-session".to_string(),
                            role: "system".to_string(),
                            text: "PreToolUse:Bash [ccth --debug] completed successfully: [dotenv@17.2.1] injecting env (".to_string(),
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
                            timestamp: "2025-07-25T19:19:00Z".to_string(),
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
                        SearchResult {
                            file: "/test/file.jsonl".to_string(),
                            uuid: "msg_3".to_string(),
                            timestamp: "2025-07-25T19:19:00Z".to_string(),
                            session_id: "test-session".to_string(),
                            role: "system".to_string(),
                            text: "PostToolUse:Bash [ccth --debug] completed successfully: [dotenv@17.2.1] injecting env".to_string(),
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
                            uuid: "msg_4".to_string(),
                            timestamp: "2025-07-25T19:19:00Z".to_string(),
                            session_id: "test-session".to_string(),
                            role: "system".to_string(),
                            text: "Running PostToolUse:Bash...".to_string(),
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

            // Look for the specific pattern in the output
            let mut found_pattern = false;
            for y in 0..20 {
                let line = extract_line(buffer, y, width);

                // Check for the exact pattern reported by user
                if line.contains("│        │") {
                    found_pattern = true;
                    println!("Found '│        │' pattern at line {} (width {})", y, width);
                    println!("Line content: '{}'", line);

                    // Find all occurrences
                    let mut pos = 0;
                    while let Some(found) = line[pos..].find("│        │") {
                        let actual_pos = pos + found;
                        println!("  Pattern found at position {}", actual_pos);

                        // Show context around the pattern
                        let start = actual_pos.saturating_sub(10);
                        let end = (actual_pos + 20).min(line.len());
                        println!("  Context: '{}'", &line[start..end]);

                        pos = actual_pos + 1;
                    }
                }

                // Also check for other double bar patterns
                if line.contains("││") {
                    println!("Found '││' pattern at line {} (width {})", y, width);
                }
            }

            if found_pattern {
                println!("\n⚠️  REPRODUCED THE ISSUE with width {}!", width);

                // Print the full output for this width
                println!("\nFull output:");
                for y in 0..20 {
                    let line = extract_line(buffer, y, width);
                    if !line.trim().is_empty() {
                        println!("{:2}: {}", y, line);
                    }
                }
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
