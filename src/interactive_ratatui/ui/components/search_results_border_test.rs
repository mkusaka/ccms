#[cfg(test)]
mod tests {
    use crate::interactive_ratatui::ui::components::{Component, result_list::ResultList};
    use crate::query::condition::SearchResult;
    use ratatui::{Terminal, backend::TestBackend, buffer::Buffer};

    #[test]
    fn test_search_results_exact_issue() {
        // Test with various terminal widths to reproduce the issue
        let test_cases = vec![
            (80, "narrow terminal"),
            (120, "standard terminal"),
            (160, "wide terminal"),
        ];

        for (width, desc) in test_cases {
            println!("\n=== Testing {} (width: {}) ===", desc, width);

            let backend = TestBackend::new(width, 30);
            let mut terminal = Terminal::new(backend).unwrap();

            terminal.draw(|f| {
                let area = f.area();
                
                let mut result_list = ResultList::new();
                
                // Create exactly the kind of system messages the user showed
                let items = vec![
                    SearchResult {
                        file: "/Users/masatomokusaka/.claude/e9b0702d-5dc8-42ed-901c-2a5d6c31c362_20250725_183419.jsonl".to_string(),
                        uuid: "msg_01JtMvKqx5hZyoEnxPTCJGsw".to_string(),
                        timestamp: "2025-07-25T09:41:06.758091Z".to_string(),
                        session_id: "e9b0702d-5dc8-42ed-901c-2a5d6c31c362".to_string(),
                        role: "system".to_string(),
                        text: "PostToolUse:Edit [ccth --debug] completed successfully: [dotenv@17.2.1] injecting env (0) from .env".to_string(),
                        has_tools: false,
                        has_thinking: false,
                        message_type: "normal".to_string(),
                        query: crate::query::condition::QueryCondition::Literal { 
                            pattern: "test".to_string(), 
                            case_sensitive: false 
                        },
                        project_path: "/Users/masatomokusaka/src/github.com/mkusaka/ccms/.git/tmp_worktrees/20250726_000402_refactorrrr".to_string(),
                        raw_json: Some("{}".to_string()),
                    },
                    SearchResult {
                        file: "/Users/masatomokusaka/.claude/e9b0702d-5dc8-42ed-901c-2a5d6c31c362_20250725_183419.jsonl".to_string(),
                        uuid: "msg_01TJGEuvzEfcBfJdFXxXP63h".to_string(),
                        timestamp: "2025-07-25T09:41:06.757851Z".to_string(),
                        session_id: "e9b0702d-5dc8-42ed-901c-2a5d6c31c362".to_string(),
                        role: "system".to_string(),
                        text: "Running PostToolUse:Edit...".to_string(),
                        has_tools: false,
                        has_thinking: false,
                        message_type: "normal".to_string(),
                        query: crate::query::condition::QueryCondition::Literal { 
                            pattern: "test".to_string(), 
                            case_sensitive: false 
                        },
                        project_path: "/Users/masatomokusaka/src/github.com/mkusaka/ccms/.git/tmp_worktrees/20250726_000402_refactorrrr".to_string(),
                        raw_json: Some("{}".to_string()),
                    },
                ];
                result_list.set_results(items);
                result_list.render(f, area);
            }).unwrap();

            let buffer = terminal.backend().buffer();

            // Look for the exact pattern "│        │"
            let mut found_pattern = false;
            let mut pattern_lines = Vec::new();

            for y in 0..30 {
                let line = extract_line(buffer, y, width);
                if line.contains("│        │") {
                    found_pattern = true;
                    pattern_lines.push((y, line.clone()));
                    println!("FOUND PATTERN at line {}: {}", y, line);
                }
            }

            if found_pattern {
                println!("\nFull output showing the issue:");
                for y in 0..30 {
                    let line = extract_line(buffer, y, width);
                    if !line.trim().is_empty() {
                        println!("{:2}: {}", y, line);
                        // Show character positions for lines with the pattern
                        if pattern_lines.iter().any(|(ly, _)| *ly == y) {
                            print!("    ");
                            for (_i, c) in line.chars().enumerate() {
                                if c == '│' {
                                    print!("^");
                                } else {
                                    print!(" ");
                                }
                            }
                            println!(" (positions of │)");
                        }
                    }
                }

                // Try to understand what's causing the extra columns
                println!("\nAnalyzing the pattern:");
                for (y, line) in &pattern_lines {
                    println!("Line {}: {}", y, line);

                    // Count the number of │ characters
                    let pipe_count = line.chars().filter(|&c| c == '│').count();
                    println!("  Number of │ characters: {}", pipe_count);

                    // Find positions of │ characters
                    let positions: Vec<usize> = line
                        .chars()
                        .enumerate()
                        .filter(|(_, c)| *c == '│')
                        .map(|(i, _)| i)
                        .collect();
                    println!("  Positions: {:?}", positions);

                    // Check if it's at the end of content
                    if let Some(last_content_pos) = line.rfind(|c: char| c != ' ' && c != '│') {
                        println!("  Last content at position: {}", last_content_pos);
                        println!(
                            "  Extra space after content: {} chars",
                            line.len() - last_content_pos - 1
                        );
                    }
                }

                panic!(
                    "Found the '│        │' pattern in {} at lines: {:?}",
                    desc,
                    pattern_lines.iter().map(|(y, _)| y).collect::<Vec<_>>()
                );
            }
        }
    }

    #[test]
    fn test_table_column_calculation_issue() {
        // Focus on the specific column width issue
        let backend = TestBackend::new(120, 20);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal.draw(|f| {
            let area = f.area();
            
            let mut result_list = ResultList::new();
            
            // Create messages with varying content lengths to expose the issue
            let items = vec![
                SearchResult {
                    file: "/test".to_string(),
                    uuid: "1".to_string(),
                    timestamp: "2025-07-25T09:41:06Z".to_string(),
                    session_id: "test".to_string(),
                    role: "system".to_string(),
                    text: "A".to_string(), // Very short
                    has_tools: false,
                    has_thinking: false,
                    message_type: "normal".to_string(),
                    query: crate::query::condition::QueryCondition::Literal { 
                        pattern: "test".to_string(), 
                        case_sensitive: false 
                    },
                    project_path: "/test".to_string(),
                    raw_json: Some("{}".to_string()),
                },
                SearchResult {
                    file: "/test".to_string(),
                    uuid: "2".to_string(),
                    timestamp: "2025-07-25T09:41:07Z".to_string(),
                    session_id: "test".to_string(),
                    role: "system".to_string(),
                    text: "Medium length message here for testing".to_string(),
                    has_tools: false,
                    has_thinking: false,
                    message_type: "normal".to_string(),
                    query: crate::query::condition::QueryCondition::Literal { 
                        pattern: "test".to_string(), 
                        case_sensitive: false 
                    },
                    project_path: "/test".to_string(),
                    raw_json: Some("{}".to_string()),
                },
                SearchResult {
                    file: "/test".to_string(),
                    uuid: "3".to_string(),
                    timestamp: "2025-07-25T09:41:08Z".to_string(),
                    session_id: "test".to_string(),
                    role: "system".to_string(),
                    text: "This is a much longer message that should fill up more of the available width and help us see if the table is calculating column widths correctly".to_string(),
                    has_tools: false,
                    has_thinking: false,
                    message_type: "normal".to_string(),
                    query: crate::query::condition::QueryCondition::Literal { 
                        pattern: "test".to_string(), 
                        case_sensitive: false 
                    },
                    project_path: "/test".to_string(),
                    raw_json: Some("{}".to_string()),
                },
            ];
            result_list.set_results(items);
            result_list.render(f, area);
        }).unwrap();

        let buffer = terminal.backend().buffer();

        println!("\nTable column calculation test:");

        // Find the content area (after the header)
        let mut content_start = 0;
        for y in 0..20 {
            let line = extract_line(buffer, y, 120);
            if line.contains("──────") {
                content_start = y + 1;
                break;
            }
        }

        // Analyze the content rows
        for y in content_start..(content_start + 5) {
            let line = extract_line(buffer, y, 120);
            if line.trim().is_empty() {
                continue;
            }

            println!("\nRow {}: {}", y, line);

            // Try to identify column boundaries
            let chars: Vec<char> = line.chars().collect();

            // Find timestamp column (should be around position 0-11)
            let timestamp_end = 11;
            let timestamp: String = chars[..timestamp_end.min(chars.len())].iter().collect();
            println!("  Timestamp column (0-10): '{}'", timestamp.trim());

            // Find role column (should be around position 13-23)
            if chars.len() > 13 {
                let role_start = 13;
                let role_end = 23.min(chars.len());
                let role: String = chars[role_start..role_end].iter().collect();
                println!("  Role column (13-22): '{}'", role.trim());
            }

            // Find content column (should start around position 25)
            if chars.len() > 25 {
                let content_start = 25;
                let content: String = chars[content_start..].iter().collect();
                println!("  Content column (25-): '{}'", content.trim_end());

                // Check if there are unexpected characters at the end
                if content.trim_end() != content {
                    let trailing = content.len() - content.trim_end().len();
                    println!("  TRAILING WHITESPACE: {} characters", trailing);
                }
            }

            // Look for any │ characters that shouldn't be there
            for (i, c) in chars.iter().enumerate() {
                if *c == '│' && i > 0 && i < chars.len() - 1 {
                    println!("  WARNING: Found │ at position {} (not at edge)", i);
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
