#[cfg(test)]
mod tests {
    use ratatui::{
        Terminal,
        backend::TestBackend,
        buffer::Buffer,
        layout::{Constraint, Rect},
        style::{Color, Modifier, Style},
        widgets::{Cell, Row, Table, TableState},
    };

    #[test]
    fn test_table_widget_raw_behavior() {
        // Test the exact Table configuration we're using
        let backend = TestBackend::new(80, 10);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|f| {
                let area = Rect::new(0, 0, 80, 10);

                // Create rows exactly as ListViewer does
                let rows = vec![
                    Row::new(vec![
                        Cell::from("07/25 18:22").style(Style::default().fg(Color::DarkGray)),
                        Cell::from("system").style(Style::default().fg(Color::Blue)),
                        Cell::from("Short message"),
                    ]),
                    Row::new(vec![
                        Cell::from("07/25 18:23").style(Style::default().fg(Color::DarkGray)),
                        Cell::from("user").style(Style::default().fg(Color::Green)),
                        Cell::from("A longer message that might reveal the issue"),
                    ]),
                ];

                // Use exact same constraints as ListViewer
                let widths = [
                    Constraint::Length(11), // Timestamp
                    Constraint::Length(10), // Role
                    Constraint::Min(1),     // Content
                ];

                let table = Table::new(rows, widths)
                    .column_spacing(2)
                    .row_highlight_style(
                        Style::default()
                            .bg(Color::DarkGray)
                            .add_modifier(Modifier::BOLD),
                    );

                let mut state = TableState::default();
                state.select(Some(0));

                f.render_stateful_widget(table, area, &mut state);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();

        // Print with visual markers
        println!("\nRaw Table widget output (80 chars wide):");
        println!("{}|", "0123456789".repeat(8));
        for y in 0..10 {
            let line = extract_line_with_markers(buffer, y, 80);
            println!("{}", line);
        }

        // Check for extra columns or borders
        for y in 0..10 {
            let line = extract_raw_line(buffer, y, 80);
            if line.contains("│") {
                println!("Found border character at line {}: '{}'", y, line);
                // Check if there are border characters where they shouldn't be
                for x in 0..80 {
                    let cell = &buffer[(x, y)];
                    if cell.symbol() == "│" {
                        println!("  Border at column {}", x);
                    }
                }
            }
        }
    }

    #[test]
    fn test_table_with_block_border() {
        // Test what happens when Table has a border block
        let backend = TestBackend::new(80, 10);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|f| {
                let area = Rect::new(0, 0, 80, 10);

                let rows = vec![Row::new(vec![
                    Cell::from("07/25 18:22"),
                    Cell::from("system"),
                    Cell::from("Test message"),
                ])];

                let widths = [
                    Constraint::Length(11),
                    Constraint::Length(10),
                    Constraint::Min(1),
                ];

                // Table WITH borders
                let table = Table::new(rows, widths).column_spacing(2).block(
                    ratatui::widgets::Block::default()
                        .borders(ratatui::widgets::Borders::ALL)
                        .title("With Border"),
                );

                let mut state = TableState::default();
                state.select(Some(0));

                f.render_stateful_widget(table, area, &mut state);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();

        println!("\nTable with borders:");
        for y in 0..10 {
            let line = extract_raw_line(buffer, y, 80);
            if !line.trim().is_empty() {
                println!("{:2}: '{}'", y, line);
            }
        }
    }

    #[test]
    fn test_minimal_table_reproduction() {
        // Minimal test case
        let backend = TestBackend::new(50, 5);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|f| {
                let area = f.area();

                let rows = vec![
                    Row::new(vec!["Time", "Role", "Text"]),
                    Row::new(vec!["18:22", "sys", "OK"]),
                ];

                let widths = [
                    Constraint::Length(5),
                    Constraint::Length(4),
                    Constraint::Min(0),
                ];

                let table = Table::new(rows, widths);
                f.render_widget(table, area);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();

        println!("\nMinimal table (50 chars):");
        for y in 0..5 {
            print!("{}: ", y);
            for x in 0..50 {
                let cell = &buffer[(x, y)];
                let sym = cell.symbol();
                if sym == " " {
                    print!("·");
                } else {
                    print!("{}", sym);
                }
            }
            println!();
        }
    }

    fn extract_line_with_markers(buffer: &Buffer, y: u16, width: u16) -> String {
        let mut line = String::new();
        for x in 0..width {
            let cell = &buffer[(x, y)];
            let symbol = cell.symbol();
            if symbol == " " {
                line.push('·');
            } else if symbol == "│" {
                line.push('║'); // Make borders more visible
            } else {
                line.push_str(symbol);
            }
        }
        format!("{:2}|{}", y, line)
    }

    fn extract_raw_line(buffer: &Buffer, y: u16, width: u16) -> String {
        let mut line = String::new();
        for x in 0..width {
            let cell = &buffer[(x, y)];
            line.push_str(cell.symbol());
        }
        line
    }
}
