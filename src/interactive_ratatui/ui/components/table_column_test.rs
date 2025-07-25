#[cfg(test)]
mod tests {
    use crate::interactive_ratatui::ui::components::list_viewer::ListViewer;
    use crate::query::condition::SearchResult;
    use ratatui::{Terminal, backend::TestBackend, buffer::Buffer, layout::Rect};

    #[test]
    fn test_table_column_rendering() {
        // Create a terminal with exact width
        let backend = TestBackend::new(80, 10);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|f| {
                let area = Rect::new(0, 0, 80, 10);

                // Create a ListViewer without borders
                let mut viewer =
                    ListViewer::<SearchResult>::new("Results".to_string(), "Empty".to_string());
                viewer.set_with_border(false);

                // Add a short system message
                let items = vec![SearchResult {
                    file: "/test/path".to_string(),
                    uuid: "test-uuid".to_string(),
                    timestamp: "2024-07-25T18:22:00Z".to_string(),
                    session_id: "test-session".to_string(),
                    role: "system".to_string(),
                    text: "Short".to_string(), // Very short content
                    has_tools: false,
                    has_thinking: false,
                    message_type: "normal".to_string(),
                    query: crate::query::condition::QueryCondition::Literal {
                        pattern: "test".to_string(),
                        case_sensitive: false,
                    },
                    project_path: "/test/project".to_string(),
                    raw_json: Some("{}".to_string()),
                }];
                viewer.set_items(items);

                // Render
                viewer.render(f, area);
            })
            .unwrap();

        // Get the rendered output
        let buffer = terminal.backend().buffer();

        // Print each line with character positions
        println!("Table column rendering test:");
        for y in 0..10 {
            let line = buffer_line_with_positions(buffer, y, 80);
            println!("Line {}: {}", y, line);

            // Also print character by character for debugging
            print!("Chars: ");
            for x in 0..80 {
                let cell = &buffer[(x, y)];
                let symbol = cell.symbol();
                if symbol == " " {
                    print!("·");
                } else {
                    print!("{}", symbol);
                }
            }
            println!();
        }

        // Check if there are extra borders at the end
        for y in 0..10 {
            let line = buffer_line_to_string(buffer, y, 80);
            if line.contains("│") {
                // Find the last non-space character
                let trimmed = line.trim_end();
                println!("Line {} trimmed: '{}'", y, trimmed);

                // Check if it ends with border characters
                if trimmed.ends_with("│") && !trimmed.ends_with("Short│") {
                    panic!("Line {} has unexpected border at end: '{}'", y, trimmed);
                }
            }
        }
    }

    #[test]
    fn test_table_with_different_content_lengths() {
        let backend = TestBackend::new(100, 15);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal.draw(|f| {
            let area = Rect::new(0, 0, 100, 15);
            
            let mut viewer = ListViewer::<SearchResult>::new("Results".to_string(), "Empty".to_string());
            viewer.set_with_border(false);
            
            // Add messages with different content lengths
            let items = vec![
                SearchResult {
                    file: "/test/path".to_string(),
                    uuid: "test-uuid-1".to_string(),
                    timestamp: "2024-07-25T18:22:00Z".to_string(),
                    session_id: "test-session".to_string(),
                    role: "system".to_string(),
                    text: "Short".to_string(),
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
                    file: "/test/path".to_string(),
                    uuid: "test-uuid-2".to_string(),
                    timestamp: "2024-07-25T18:23:00Z".to_string(),
                    session_id: "test-session".to_string(),
                    role: "user".to_string(),
                    text: "This is a much longer message that should fill more of the content area".to_string(),
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
                    file: "/test/path".to_string(),
                    uuid: "test-uuid-3".to_string(),
                    timestamp: "2024-07-25T18:24:00Z".to_string(),
                    session_id: "test-session".to_string(),
                    role: "assistant".to_string(),
                    text: "Medium length content here".to_string(),
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
            viewer.set_items(items);
            
            viewer.render(f, area);
        }).unwrap();

        let buffer = terminal.backend().buffer();

        println!("\nTable with different content lengths:");
        for y in 0..15 {
            let line = buffer_line_to_string(buffer, y, 100);
            if !line.trim().is_empty() {
                println!("Line {}: '{}'", y, line);
            }
        }
    }

    fn buffer_line_to_string(buffer: &Buffer, y: u16, width: u16) -> String {
        let mut line = String::new();
        for x in 0..width {
            let cell = &buffer[(x, y)];
            line.push_str(cell.symbol());
        }
        line.trim_end().to_string()
    }

    fn buffer_line_with_positions(buffer: &Buffer, y: u16, width: u16) -> String {
        let mut line = String::new();
        for x in 0..width {
            let cell = &buffer[(x, y)];
            line.push_str(cell.symbol());
        }
        line
    }
}
