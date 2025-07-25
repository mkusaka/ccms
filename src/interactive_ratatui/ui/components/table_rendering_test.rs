#[cfg(test)]
mod tests {
    use crate::interactive_ratatui::ui::components::list_item::ListItem;
    use crate::interactive_ratatui::ui::components::list_viewer::ListViewer;
    use ratatui::{Terminal, backend::TestBackend, buffer::Buffer};

    #[derive(Clone)]
    struct TestItem {
        role: String,
        timestamp: String,
        content: String,
    }

    impl ListItem for TestItem {
        fn get_role(&self) -> &str {
            &self.role
        }

        fn get_timestamp(&self) -> &str {
            &self.timestamp
        }

        fn get_content(&self) -> &str {
            &self.content
        }
    }

    #[test]
    fn test_table_rendering_no_extra_columns() {
        let mut viewer = ListViewer::<TestItem>::new("Test".to_string(), "Empty".to_string());

        // Create test data that matches the problematic output
        let items = vec![
            TestItem {
                role: "system".to_string(),
                timestamp: "2024-07-25T18:22:00Z".to_string(),
                content: "PostToolUse:Edit [ccth --debug] completed successfully: [dotenv@17.2.1] injecting env (0) from .env".to_string(),
            },
            TestItem {
                role: "system".to_string(),
                timestamp: "2024-07-25T18:22:00Z".to_string(),
                content: "Running PostToolUse:Edit...".to_string(),
            },
        ];

        viewer.set_items(items);

        // Create a test terminal
        let backend = TestBackend::new(120, 10);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|f| {
                let area = f.area();
                viewer.render(f, area);
            })
            .unwrap();

        // Get the rendered output
        let buffer = terminal.backend().buffer();

        // Print for debugging
        println!("Rendered output:");
        for y in 0..10 {
            let line = buffer_line_to_string(buffer, y, 120);
            println!("{:2}: {:?}", y, line);
        }

        // Check for double borders - the line should not contain "│        │"
        for y in 0..10 {
            let line = buffer_line_to_string(buffer, y, 120);
            assert!(
                !line.contains("│        │"),
                "Line {} contains double border: {:?}",
                y,
                line
            );
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
}
