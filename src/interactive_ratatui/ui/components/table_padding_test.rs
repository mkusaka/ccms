#[cfg(test)]
mod tests {
    use ratatui::{
        Terminal, backend::TestBackend, buffer::Buffer,
        widgets::{Table, Row, Cell, Block, Borders, TableState},
        layout::Constraint,
        style::{Style, Color, Modifier},
    };

    #[test]
    fn test_table_padding_behavior() {
        // Test different approaches to prevent table padding
        let backend = TestBackend::new(80, 20);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|f| {
                let area = f.area();

                // Test 1: Basic table with different constraints
                let rows = vec![
                    Row::new(vec![
                        Cell::from("Time"),
                        Cell::from("Role"),
                        Cell::from("Short"),
                    ]),
                    Row::new(vec![
                        Cell::from("Time"),
                        Cell::from("Role"),
                        Cell::from("This is a longer message that should not have trailing spaces"),
                    ]),
                    Row::new(vec![
                        Cell::from("Time"),
                        Cell::from("Role"),
                        Cell::from(""), // Empty content
                    ]),
                ];

                // Try different constraint combinations
                let test_cases = vec![
                    ("Min(0)", vec![
                        Constraint::Length(10),
                        Constraint::Length(10),
                        Constraint::Min(0),
                    ]),
                    ("Percentage", vec![
                        Constraint::Length(10),
                        Constraint::Length(10),
                        Constraint::Percentage(100),
                    ]),
                    ("Ratio", vec![
                        Constraint::Length(10),
                        Constraint::Length(10),
                        Constraint::Ratio(1, 1),
                    ]),
                ];

                // Test the first case
                let widths = test_cases[0].1.clone();
                
                let table = Table::new(rows.clone(), widths)
                    .block(Block::default().title("Table Padding Test").borders(Borders::ALL))
                    .column_spacing(1)
                    .row_highlight_style(Style::default().bg(Color::DarkGray));

                let mut state = TableState::default();
                state.select(Some(0));
                
                f.render_stateful_widget(table, area, &mut state);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();

        println!("\nTable padding behavior test:");
        
        // Analyze each line
        for y in 0..20 {
            let line = extract_line(buffer, y, 80);
            if line.contains("│") && !line.contains("─") && !line.contains("┌") && !line.contains("└") {
                println!("Data line {}: '{}'", y, line);
                
                // Find content between borders
                if let (Some(first), Some(last)) = (line.find('│'), line.rfind('│')) {
                    if first != last {
                        // Get the substring properly considering multibyte characters
                        let chars: Vec<char> = line.chars().collect();
                        let first_char_idx = line[..first].chars().count();
                        let last_char_idx = line[..last].chars().count();
                        
                        let content_chars: Vec<char> = chars[(first_char_idx + 1)..last_char_idx].to_vec();
                        let content: String = content_chars.iter().collect();
                        let trimmed = content.trim_end();
                        let padding = content.len() - trimmed.len();
                        
                        if padding > 0 {
                            println!("  → Content: '{}' (length: {})", trimmed, trimmed.len());
                            println!("  → Padding: {} spaces", padding);
                            
                            // Check if padding creates the pattern
                            if content.ends_with("        ") {
                                println!("  → WARNING: Found '        ' pattern at end!");
                            }
                        }
                    }
                }
            }
        }

        // Test 2: Try without any borders
        terminal
            .draw(|f| {
                let area = f.area();
                
                let rows = vec![
                    Row::new(vec![
                        Cell::from("Time"),
                        Cell::from("Role"),
                        Cell::from("Content"),
                    ]),
                ];

                let widths = vec![
                    Constraint::Length(10),
                    Constraint::Length(10),
                    Constraint::Length(20), // Fixed width
                ];
                
                let table = Table::new(rows, widths)
                    .column_spacing(1);

                f.render_widget(table, area);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        
        println!("\n\nTable without borders:");
        for y in 0..5 {
            let line = extract_line(buffer, y, 80);
            if !line.trim().is_empty() {
                println!("Line {}: '{}'", y, line);
                let trimmed = line.trim_end();
                if line.len() > trimmed.len() {
                    println!("  → {} trailing spaces", line.len() - trimmed.len());
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