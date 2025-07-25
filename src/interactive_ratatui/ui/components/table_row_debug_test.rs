#[cfg(test)]
mod tests {
    use crate::interactive_ratatui::ui::components::list_viewer::ListViewer;
    use crate::query::condition::SearchResult;
    use ratatui::{
        Terminal,
        backend::TestBackend,
        buffer::Buffer,
        layout::Constraint,
        widgets::{Block, Borders, Cell, Row, Table, TableState},
    };

    #[test]
    fn test_table_row_creation_debug() {
        // Create a simple table directly to understand the issue
        let backend = TestBackend::new(120, 20);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|f| {
                let area = f.area();

                // Create rows similar to what ListViewer creates
                let rows = vec![
                    Row::new(vec![
                        Cell::from("07/25 19:19"),
                        Cell::from("system"),
                        Cell::from("PreToolUse:Bash [ccth --debug] completed successfully: [dotenv@17.2.1] injecting env ("),
                    ]),
                    Row::new(vec![
                        Cell::from("07/25 19:19"),
                        Cell::from("assistant"),
                        Cell::from(""),
                    ]),
                ];

                // Print debug info about rows
                println!("\nDebug: Row information");
                println!("Row 0: timestamp='07/25 19:19', role='system', content='PreToolUse:Bash [ccth --debug] completed successfully: [dotenv@17.2.1] injecting env ('");
                println!("Row 1: timestamp='07/25 19:19', role='assistant', content=''");

                // Create table with same constraints as ListViewer
                let widths = [
                    Constraint::Length(11), // Timestamp
                    Constraint::Length(10), // Role
                    Constraint::Min(0),     // Content
                ];

                let table = Table::new(rows.clone(), widths)
                    .column_spacing(1)
                    .block(Block::default().title("Test Table").borders(Borders::ALL));

                // Also try without column_spacing to see if that's the issue
                let table_no_spacing = Table::new(rows, [
                    Constraint::Length(11),
                    Constraint::Length(10),
                    Constraint::Min(0),
                ])
                    .column_spacing(0)
                    .block(Block::default().title("No Spacing").borders(Borders::ALL));

                // Render both tables
                let chunks = ratatui::layout::Layout::default()
                    .direction(ratatui::layout::Direction::Vertical)
                    .constraints([
                        Constraint::Percentage(50),
                        Constraint::Percentage(50),
                    ])
                    .split(area);

                let mut state = TableState::default();
                state.select(Some(0));
                
                f.render_stateful_widget(table, chunks[0], &mut state);
                f.render_stateful_widget(table_no_spacing, chunks[1], &mut state.clone());
            })
            .unwrap();

        let buffer = terminal.backend().buffer();

        // Analyze the output
        println!("\nTable output analysis:");
        for y in 0..20 {
            let line = extract_line(buffer, y, 120);
            if !line.trim().is_empty() {
                // Look for the problematic pattern
                if line.contains("│        │") {
                    println!("Line {}: {} ⚠️ FOUND PATTERN", y, line);
                } else {
                    println!("Line {}: {}", y, line);
                }
            }
        }
    }

    #[test]
    fn test_list_viewer_row_creation() {
        // Test ListViewer directly
        let mut list_viewer = ListViewer::new("Test".to_string(), "Empty".to_string());
        list_viewer.set_with_border(true);

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
        ];

        list_viewer.set_items(items);

        let backend = TestBackend::new(120, 10);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|f| {
                list_viewer.render(f, f.area());
            })
            .unwrap();

        let buffer = terminal.backend().buffer();

        println!("\nListViewer output:");
        for y in 0..10 {
            let line = extract_line(buffer, y, 120);
            if !line.trim().is_empty() {
                if line.contains("│        │") {
                    println!("Line {}: {} ⚠️ FOUND PATTERN", y, line);
                } else {
                    println!("Line {}: {}", y, line);
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
