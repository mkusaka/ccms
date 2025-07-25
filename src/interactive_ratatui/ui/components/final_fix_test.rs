#[cfg(test)]
mod tests {
    use crate::interactive_ratatui::ui::components::{Component, result_list::ResultList};
    use crate::query::condition::SearchResult;
    use ratatui::{Terminal, backend::TestBackend, buffer::Buffer};

    #[test]
    fn test_result_list_has_border_without_double_pattern() {
        let backend = TestBackend::new(120, 20);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|f| {
                let area = f.area();

                let mut result_list = ResultList::new();

                // ユーザーが報告した問題と同じ種類のメッセージを作成
                let items = vec![
                    SearchResult {
                        file: "/test/file.jsonl".to_string(),
                        uuid: "msg_1".to_string(),
                        timestamp: "2025-07-25T19:11:00Z".to_string(),
                        session_id: "test-session".to_string(),
                        role: "system".to_string(),
                        text: "PostToolUse:Edit [ccth --debug] completed successfully: [dotenv@17.2.1] injecting env (0)".to_string(),
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
                    SearchResult {
                        file: "/test/file.jsonl".to_string(),
                        uuid: "msg_2".to_string(),
                        timestamp: "2025-07-25T19:11:00Z".to_string(),
                        session_id: "test-session".to_string(),
                        role: "system".to_string(),
                        text: "Running PostToolUse:Edit...".to_string(),
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
                result_list.set_results(items);
                result_list.render(f, area);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();

        // 枠線が存在することを確認
        let mut has_border = false;
        let mut has_double_pattern = false;

        for y in 0..20 {
            let line = extract_line(buffer, y, 120);

            // 枠線の存在を確認（┌, │, └などの文字）
            if line.contains("┌") || line.contains("│") || line.contains("└") {
                has_border = true;
            }

            // 元の問題のパターンを確認
            if line.contains("│        │") {
                has_double_pattern = true;
                println!("Found problematic pattern at line {}: {}", y, line);
            }

            // 行末の余分なスペースも確認
            if line.trim_end() != line {
                let trailing = line.len() - line.trim_end().len();
                if trailing > 10 && line.contains("│") {
                    println!(
                        "Line {} has {} trailing spaces: '{}'",
                        y,
                        trailing,
                        &line[line.len() - 20..]
                    );
                }
            }
        }

        assert!(has_border, "ResultList should have borders");
        assert!(
            !has_double_pattern,
            "Should not have the problematic '│        │' pattern"
        );

        // デバッグ用の出力
        println!("\nResultList with borders (final fix):");
        for y in 0..20 {
            let line = extract_line(buffer, y, 120);
            if !line.trim().is_empty() {
                println!("{:2}: {}", y, line);
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
