#[cfg(test)]
mod tests {
    use crate::interactive_ratatui::ui::components::{
        Component, result_list::ResultList, search_bar::SearchBar,
    };
    use crate::query::condition::SearchResult;
    use ratatui::{
        Terminal,
        backend::TestBackend,
        buffer::Buffer,
        layout::{Constraint, Direction, Layout},
        style::{Color, Style},
        widgets::Paragraph,
    };

    #[test]
    fn test_full_search_mode_layout() {
        // Create a terminal that matches typical terminal width
        let backend = TestBackend::new(120, 30);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal.draw(|f| {
            // Simulate the exact layout from renderer.rs
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3), // Search bar
                    Constraint::Min(0),    // Results
                    Constraint::Length(1), // Status bar
                ])
                .split(f.area());
            
            // Render search bar
            let mut search_bar = SearchBar::new();
            search_bar.render(f, chunks[0]);
            
            // Render result list with test data
            let mut result_list = ResultList::new();
            let items = vec![
                SearchResult {
                    file: "/test/path".to_string(),
                    uuid: "test-uuid-1".to_string(),
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
            result_list.render(f, chunks[1]);
            
            // Render status bar (from renderer.rs)
            let status_text = "Tab: Filter | ↑/↓: Navigate | Enter: Detail | s: Session | Ctrl+T: Toggle [Truncated] | ?: Help | Esc: Exit";
            let status_bar = Paragraph::new(status_text).style(Style::default().fg(Color::DarkGray));
            f.render_widget(status_bar, chunks[2]);
        }).unwrap();

        // Get the rendered output
        let buffer = terminal.backend().buffer();

        // Print the full layout
        println!("Full search mode layout:");
        println!("{}", "=".repeat(120));
        for y in 0..30 {
            let line = buffer_line_to_string(buffer, y, 120);
            println!("{:2}: {}", y, line);
        }
        println!("{}", "=".repeat(120));

        // Check for the specific double border pattern
        let mut found_double_border = false;
        for y in 0..30 {
            let line = buffer_line_to_string(buffer, y, 120);
            if line.contains("│        │") {
                found_double_border = true;
                println!("Found double border pattern at line {}: {}", y, line);
            }
        }

        assert!(
            !found_double_border,
            "Should not have double border pattern '│        │'"
        );
    }

    #[test]
    fn test_result_list_width_calculation() {
        // Test with exact widths to understand the calculation
        let backend = TestBackend::new(80, 20);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|f| {
                let area = f.area();

                let mut result_list = ResultList::new();
                let items = vec![SearchResult {
                    file: "/test/path".to_string(),
                    uuid: "test-uuid".to_string(),
                    timestamp: "2024-07-25T18:22:00Z".to_string(),
                    session_id: "test-session".to_string(),
                    role: "system".to_string(),
                    text: "Short".to_string(),
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
                result_list.set_results(items);
                result_list.render(f, area);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();

        println!("\nResult list width calculation (80 chars wide):");
        for y in 0..20 {
            let line = buffer_line_with_char_positions(buffer, y, 80);
            if !line.trim().is_empty() {
                println!("{:2}: {}", y, line);
                // Print character positions
                print!("    ");
                for x in 0..80 {
                    if x % 10 == 0 {
                        print!("{}", x / 10);
                    } else {
                        print!(" ");
                    }
                }
                println!();
                print!("    ");
                for x in 0..80 {
                    print!("{}", x % 10);
                }
                println!();
            }
        }
    }

    fn buffer_line_to_string(buffer: &Buffer, y: u16, width: u16) -> String {
        let mut line = String::new();
        for x in 0..width {
            let cell = &buffer[(x, y)];
            line.push_str(cell.symbol());
        }
        line
    }

    fn buffer_line_with_char_positions(buffer: &Buffer, y: u16, width: u16) -> String {
        let mut line = String::new();
        for x in 0..width {
            let cell = &buffer[(x, y)];
            let symbol = cell.symbol();
            if symbol == " " {
                line.push('·');
            } else {
                line.push_str(symbol);
            }
        }
        line
    }
}
