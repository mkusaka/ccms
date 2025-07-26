#[cfg(test)]
mod tests {
    use crate::interactive_ratatui::ui::components::list_viewer::ListViewer;
    use crate::query::condition::SearchResult;
    use ratatui::{Terminal, backend::TestBackend, buffer::Buffer};
    use ratatui::widgets::{Table, Row, Cell, Block, Borders, TableState};
    use ratatui::layout::Constraint;

    #[test] 
    fn test_table_column_width_calculation() {
        // Ratatuiのテーブルウィジェットの列幅計算を確認
        let backend = TestBackend::new(120, 10);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|f| {
                let area = f.area();

                // 実際のListViewerと同じ設定でTableを作成
                let rows = vec![
                    Row::new(vec![
                        Cell::from("07/25 19:32"),
                        Cell::from("system"),
                        Cell::from("PreToolUse:Bash [ccth --debug] completed successfully: [dotenv@17.2.1] injecting env ("),
                    ]),
                    Row::new(vec![
                        Cell::from("07/25 19:32"), 
                        Cell::from("assistant"),
                        Cell::from(""),
                    ]),
                ];

                // ListViewerと同じ制約を使用
                let widths = [
                    Constraint::Length(11),      // Timestamp
                    Constraint::Length(10),      // Role  
                    Constraint::Percentage(100), // Content
                ];

                let table = Table::new(rows, widths)
                    .column_spacing(1)
                    .block(Block::default().title("Test").borders(Borders::ALL));

                let mut state = TableState::default();
                state.select(Some(0));
                
                f.render_stateful_widget(table, area, &mut state);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();

        println!("\nTable column width test:");
        for y in 0..10 {
            let line = extract_line(buffer, y, 120);
            if !line.trim().is_empty() {
                println!("Line {}: {}", y, line);
                
                // データ行の内容を確認
                if line.contains("system") || line.contains("assistant") {
                    // 右端の余分なスペースを確認
                    let trimmed = line.trim_end();
                    if trimmed != line {
                        let trailing = line.len() - trimmed.len();
                        println!("  WARNING: {} trailing spaces", trailing);
                    }
                    
                    // テキストコンテンツの長さを確認
                    if line.contains("PreToolUse:Bash") {
                        if let Some(start) = line.find("PreToolUse:Bash") {
                            let end = line.rfind('│').unwrap_or(line.len());
                            let content = &line[start..end];
                            println!("  Text content length: {} chars", content.len());
                            println!("  Content: '{}'", content);
                        }
                    }
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