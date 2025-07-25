#[cfg(test)]
mod tests {
    use crate::interactive_ratatui::ui::components::{Component, result_list::ResultList};
    use crate::query::condition::SearchResult;
    use ratatui::{Terminal, backend::TestBackend, buffer::Buffer};

    #[test]
    fn test_table_detailed_column_analysis() {
        // Use a wider terminal to match user's environment
        let backend = TestBackend::new(120, 20);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|f| {
                let area = f.area();

                let mut result_list = ResultList::new();

                // Create test data similar to user's report
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
                ];
                result_list.set_results(items);
                result_list.render(f, area);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();

        // Analyze each line in detail
        println!("\nDetailed line analysis:");
        for y in 0..20 {
            let line = extract_line(buffer, y, 120);
            if !line.trim().is_empty() {
                // Find all vertical bars
                let mut bar_positions = vec![];
                for (i, ch) in line.chars().enumerate() {
                    if ch == '│' {
                        bar_positions.push(i);
                    }
                }

                // Analyze the content between bars
                if bar_positions.len() >= 2 {
                    // Convert the string to a vector of chars for safe indexing
                    let line_chars: Vec<char> = line.chars().collect();

                    for i in 0..bar_positions.len() - 1 {
                        let start = bar_positions[i];
                        let end = bar_positions[i + 1];

                        if start + 1 < end && end <= line_chars.len() {
                            let content: String = line_chars[start + 1..end].iter().collect();

                            // Check for suspicious empty columns
                            if content.trim().is_empty() && content.len() > 5 {
                                println!(
                                    "Line {}: Empty column between positions {} and {} (width: {})",
                                    y,
                                    start,
                                    end,
                                    content.len()
                                );
                                println!("  Content: '{}'", content);

                                // Check if this might be the problematic pattern
                                if content == "        " {
                                    println!("  ⚠️  Found exact '        ' pattern!");
                                }
                            }
                        }
                    }
                }

                println!("{:2}: {} (bars at: {:?})", y, line, bar_positions);

                // Check for the specific pattern
                if line.contains("│        │") {
                    println!("  ⚠️  FOUND PROBLEMATIC PATTERN: '│        │'");

                    // Find where it occurs
                    if let Some(pos) = line.find("│        │") {
                        println!("  Position: {} (starts at column {})", pos, pos);
                    }
                }
            }
        }

        // Also check specific patterns
        let mut found_issue = false;
        for y in 0..20 {
            let line = extract_line(buffer, y, 120);
            if line.contains("│        │") {
                found_issue = true;
                break;
            }
        }

        // This test is designed to help debug, not necessarily fail
        if found_issue {
            println!("\n⚠️  Found the problematic '│        │' pattern in the output!");
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
