#[cfg(test)]
mod tests {
    use ratatui::{
        Terminal,
        backend::TestBackend,
        buffer::Buffer,
        layout::{Constraint, Rect},
        style::{Color, Style},
        widgets::{Block, Borders, Cell, Row, Table, TableState},
    };

    #[test]
    fn test_table_widget_column_behavior() {
        // Test if Table widget adds extra columns
        let backend = TestBackend::new(50, 10);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|f| {
                let area = Rect::new(0, 0, 50, 10);

                // Create a simple table with 3 columns
                let rows = vec![
                    Row::new(vec![
                        Cell::from("Col1"),
                        Cell::from("Col2"),
                        Cell::from("Col3"),
                    ]),
                    Row::new(vec![Cell::from("A"), Cell::from("B"), Cell::from("C")]),
                ];

                let widths = [
                    Constraint::Length(5),
                    Constraint::Length(5),
                    Constraint::Min(0),
                ];

                let table = Table::new(rows, widths).column_spacing(1);

                let mut state = TableState::default();
                state.select(Some(0));

                f.render_stateful_widget(table, area, &mut state);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();

        println!("Basic table rendering:");
        for y in 0..10 {
            let line = get_line(buffer, y, 50);
            println!("{}: |{}|", y, line);
        }
    }

    #[test]
    fn test_table_with_border_widget() {
        // Test Table inside a bordered block
        let backend = TestBackend::new(50, 10);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|f| {
                let area = Rect::new(0, 0, 50, 10);

                let rows = vec![Row::new(vec![
                    Cell::from("Col1"),
                    Cell::from("Col2"),
                    Cell::from("Col3"),
                ])];

                let widths = [
                    Constraint::Length(5),
                    Constraint::Length(5),
                    Constraint::Min(0),
                ];

                let table = Table::new(rows, widths)
                    .column_spacing(1)
                    .block(Block::default().borders(Borders::ALL).title("Table"));

                let mut state = TableState::default();
                f.render_stateful_widget(table, area, &mut state);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();

        println!("\nTable with borders:");
        for y in 0..10 {
            let line = get_line(buffer, y, 50);
            println!("{}: {}", y, line);

            // Check for double borders
            if line.contains("││") {
                println!("  ^ Found double border!");
            }

            // Check for the specific pattern
            if line.contains("│        │") {
                println!("  ^ Found the exact pattern!");
            }
        }
    }

    #[test]
    fn test_ratatui_table_edge_case() {
        // Try to reproduce the exact scenario
        let backend = TestBackend::new(120, 10);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|f| {
                let area = f.area();

                // Simulate the exact setup from ListViewer
                let rows = vec![Row::new(vec![
                    Cell::from("07/25 18:22").style(Style::default().fg(Color::DarkGray)),
                    Cell::from("system").style(Style::default().fg(Color::Blue)),
                    Cell::from("PostToolUse:Edit [ccth --debug] completed successfully"),
                ])];

                let widths = [
                    Constraint::Length(11),
                    Constraint::Length(10),
                    Constraint::Min(0),
                ];

                let table = Table::new(rows, widths)
                    .column_spacing(1)
                    .block(Block::default().borders(Borders::ALL));

                let mut state = TableState::default();
                state.select(Some(0));

                f.render_stateful_widget(table, area, &mut state);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();

        println!("\nExact scenario test (120 chars wide):");
        for y in 0..10 {
            let line = get_line(buffer, y, 120);
            println!("{}: {}", y, line);

            // Look for any suspicious patterns
            if line.contains("│  ") && line.chars().rev().take(10).any(|c| c == '│') {
                println!("  ^ Potential issue: extra space before end border");

                // Analyze the end of the line
                let trimmed = line.trim_end();
                let last_10: String = line
                    .chars()
                    .rev()
                    .take(10)
                    .collect::<Vec<_>>()
                    .into_iter()
                    .rev()
                    .collect();
                println!("    Last 10 chars: '{}'", last_10);
                println!("    Trimmed line ends at position: {}", trimmed.len());
            }
        }
    }

    #[test]
    fn test_minimal_reproduction() {
        // Absolute minimal test
        let backend = TestBackend::new(40, 5);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|f| {
                let rows = vec![Row::new(vec!["A", "B", "C"])];

                let widths = [
                    Constraint::Length(4),
                    Constraint::Length(4),
                    Constraint::Min(0),
                ];

                let table = Table::new(rows, widths);
                f.render_widget(table, f.area());
            })
            .unwrap();

        let buffer = terminal.backend().buffer();

        println!("\nMinimal test (40 chars):");
        for y in 0..5 {
            let line = get_line(buffer, y, 40);
            // Print with visible markers for spaces
            let visible: String = line
                .chars()
                .map(|c| if c == ' ' { '·' } else { c })
                .collect();
            println!("{}: {}", y, visible);
        }
    }

    fn get_line(buffer: &Buffer, y: u16, width: u16) -> String {
        let mut line = String::new();
        for x in 0..width {
            let cell = &buffer[(x, y)];
            line.push_str(cell.symbol());
        }
        line
    }
}
