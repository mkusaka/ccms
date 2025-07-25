#[cfg(test)]
mod tests {
    use crate::interactive_ratatui::ui::components::{
        list_viewer::ListViewer, view_layout::ViewLayout,
    };
    use crate::query::condition::SearchResult;
    use ratatui::{Terminal, backend::TestBackend, buffer::Buffer};

    #[test]
    fn test_no_double_borders_with_view_layout() {
        // Create a terminal
        let backend = TestBackend::new(120, 20);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal.draw(|f| {
            let area = f.area();
            
            // Create ViewLayout (which adds its own borders)
            let layout = ViewLayout::new("Test Title".to_string())
                .with_subtitle("Test Subtitle".to_string())
                .with_status_text("Test Status".to_string());
                
            // Create a ListViewer with borders disabled
            let mut viewer = ListViewer::<SearchResult>::new("Results".to_string(), "Empty".to_string());
            viewer.set_with_border(false);
            
            // Add test data
            let items = vec![
                SearchResult {
                    file: "/test/path".to_string(),
                    uuid: "test-uuid".to_string(),
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
                    file: "/test/path".to_string(),
                    uuid: "test-uuid-2".to_string(),
                    timestamp: "2024-07-25T18:22:00Z".to_string(),
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
            viewer.set_items(items);
            
            // Render using ViewLayout
            layout.render(f, area, |f, content_area| {
                viewer.render(f, content_area);
            });
        }).unwrap();

        // Get the rendered output
        let buffer = terminal.backend().buffer();

        // Print the output for debugging
        println!("Rendered output:");
        for y in 0..20 {
            let line = buffer_line_to_string(buffer, y, 120);
            println!("{:2}: {:?}", y, line);
        }

        // Check for double borders
        for y in 0..20 {
            let line = buffer_line_to_string(buffer, y, 120);
            assert!(
                !line.contains("│        │"),
                "Line {} contains double border pattern '│        │': {:?}",
                y,
                line
            );
            assert!(
                !line.contains("││"),
                "Line {} contains consecutive vertical bars '││': {:?}",
                y,
                line
            );
        }
    }

    #[test]
    fn test_double_borders_with_nested_blocks() {
        // Create a terminal
        let backend = TestBackend::new(120, 20);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|f| {
                let area = f.area();

                // Create ViewLayout (which doesn't add borders, only title and horizontal line)
                let layout = ViewLayout::new("Test Title".to_string())
                    .with_subtitle("Test Subtitle".to_string())
                    .with_status_text("Test Status".to_string());

                // Create a ListViewer with borders ENABLED
                let mut viewer =
                    ListViewer::<SearchResult>::new("Results".to_string(), "Empty".to_string());
                viewer.set_with_border(true); // This now works correctly without double borders

                // Add test data
                let items = vec![SearchResult {
                    file: "/test/path".to_string(),
                    uuid: "test-uuid".to_string(),
                    timestamp: "2024-07-25T18:22:00Z".to_string(),
                    session_id: "test-session".to_string(),
                    role: "system".to_string(),
                    text: "This should NOT show double borders".to_string(),
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

                // Render using ViewLayout
                layout.render(f, area, |f, content_area| {
                    viewer.render(f, content_area);
                });
            })
            .unwrap();

        // Get the rendered output
        let buffer = terminal.backend().buffer();

        // Print the output for debugging
        println!("\nDouble border test output:");
        for y in 0..20 {
            let line = buffer_line_to_string(buffer, y, 120);
            println!("{:2}: {:?}", y, line);
        }

        // Since ViewLayout doesn't add borders, we should NOT see nested border patterns
        let mut found_nested_border = false;
        for y in 0..20 {
            let line = buffer_line_to_string(buffer, y, 120);
            if line.contains("│┌")
                || line.contains("└│")
                || line.contains("│└")
                || line.contains("┌│")
            {
                found_nested_border = true;
                break;
            }
        }
        assert!(
            !found_nested_border,
            "Should NOT have found nested border patterns as ViewLayout doesn't add borders"
        );
    }

    fn buffer_line_to_string(buffer: &Buffer, y: u16, width: u16) -> String {
        let mut line = String::new();
        for x in 0..width {
            let cell = &buffer[(x, y)];
            line.push_str(cell.symbol());
        }
        line.trim_end().to_string()
    }
}
