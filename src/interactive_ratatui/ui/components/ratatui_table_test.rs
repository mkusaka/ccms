#[cfg(test)]
mod tests {
    use ratatui::{
        Terminal, backend::TestBackend, buffer::Buffer,
        widgets::{Table, Row, Cell, Block, Borders, TableState},
        layout::Constraint,
        style::{Style, Color},
    };

    #[test]
    fn test_ratatui_table_behavior() {
        // Ratatuiのテーブルの動作を詳しく調査
        let backend = TestBackend::new(80, 10);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|f| {
                let area = f.area();

                // 様々な制約の組み合わせを試す
                let test_cases = vec![
                    (
                        "Min(0)",
                        vec![
                            Constraint::Length(11),
                            Constraint::Length(10),
                            Constraint::Min(0),
                        ],
                    ),
                    (
                        "Percentage",
                        vec![
                            Constraint::Length(11),
                            Constraint::Length(10),
                            Constraint::Percentage(100),
                        ],
                    ),
                    (
                        "Fixed Length",
                        vec![
                            Constraint::Length(11),
                            Constraint::Length(10),
                            Constraint::Length(50),
                        ],
                    ),
                ];

                // 最初のケースだけテスト
                let (name, widths) = &test_cases[0];
                
                let rows = vec![
                    Row::new(vec![
                        Cell::from("07/25 19:32"),
                        Cell::from("system"),
                        Cell::from("Short text"),
                    ]),
                    Row::new(vec![
                        Cell::from("07/25 19:32"),
                        Cell::from("assistant"),
                        Cell::from("This is a longer text that might cause issues"),
                    ]),
                ];

                let table = Table::new(rows, widths.clone())
                    .block(Block::default().title(name.to_string()).borders(Borders::ALL))
                    .column_spacing(1);

                let mut state = TableState::default();
                state.select(Some(0));
                
                f.render_stateful_widget(table, area, &mut state);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();

        println!("\nRatatui Table behavior test:");
        for y in 0..10 {
            let line = extract_line(buffer, y, 80);
            if !line.trim().is_empty() {
                println!("Line {}: '{}'", y, line);
                
                // 行末のスペースを確認
                if line.trim_end() != line {
                    let trailing = line.len() - line.trim_end().len();
                    println!("  {} trailing spaces", trailing);
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