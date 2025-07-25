#[cfg(test)]
mod tests {
    use crate::interactive_ratatui::ui::components::{Component, result_list::ResultList};
    use crate::query::condition::SearchResult;
    use ratatui::{Terminal, backend::TestBackend, buffer::Buffer};

    #[test]
    fn test_no_double_borders_in_result_list() {
        // Test that ResultList doesn't create double borders
        let backend = TestBackend::new(120, 20);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal.draw(|f| {
            let area = f.area();
            
            let mut result_list = ResultList::new();
            
            // Add system messages like the user showed
            let items = vec![
                SearchResult {
                    file: "/test/file.jsonl".to_string(),
                    uuid: "msg_1".to_string(),
                    timestamp: "2024-07-25T18:22:00Z".to_string(),
                    session_id: "test-session".to_string(),
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
                    file: "/test/file.jsonl".to_string(),
                    uuid: "msg_2".to_string(),
                    timestamp: "2024-07-25T18:23:00Z".to_string(),
                    session_id: "test-session".to_string(),
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
            ];
            result_list.set_results(items);
            result_list.render(f, area);
        }).unwrap();

        let buffer = terminal.backend().buffer();

        // Check that there are no double border patterns
        let mut has_issue = false;
        for y in 0..20 {
            let line = extract_line(buffer, y, 120);

            // Check for the specific pattern the user reported
            if line.contains("│        │") {
                has_issue = true;
                println!("Found double border pattern at line {}: {}", y, line);
            }

            // Check for other double border patterns
            if line.contains("││") || line.contains("┤│") || line.contains("│├") {
                has_issue = true;
                println!("Found double border at line {}: {}", y, line);
            }

            // Check for excessive trailing spaces before borders
            if line.trim_end() != line && line.trim_end().ends_with("│") {
                let trailing = line.len() - line.trim_end().len();
                if trailing > 5 {
                    // Allow some padding but not excessive
                    println!("Line {} has {} trailing spaces before border", y, trailing);
                }
            }
        }

        assert!(
            !has_issue,
            "ResultList should not have double borders or the '│        │' pattern"
        );

        // Print the output for visual verification
        println!("\nResultList output (no double borders expected):");
        for y in 0..20 {
            let line = extract_line(buffer, y, 120);
            if !line.trim().is_empty() {
                println!("{:2}: {}", y, line);
            }
        }
    }

    #[test]
    fn test_list_viewer_default_no_border() {
        // Verify that ListViewer defaults to no border
        use crate::interactive_ratatui::ui::components::list_viewer::ListViewer;

        let viewer: ListViewer<SearchResult> = Default::default();
        assert!(
            !viewer.with_border,
            "ListViewer should default to no border"
        );

        let viewer = ListViewer::<SearchResult>::new("Test".to_string(), "Empty".to_string());
        assert!(
            !viewer.with_border,
            "ListViewer::new should default to no border"
        );
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
