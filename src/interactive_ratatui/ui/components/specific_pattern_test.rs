#[cfg(test)]
mod tests {
    use crate::interactive_ratatui::ui::components::{Component, result_list::ResultList};
    use crate::query::condition::SearchResult;
    use ratatui::{Terminal, backend::TestBackend, buffer::Buffer};

    #[test]
    fn test_specific_double_bar_pattern() {
        // ユーザーが報告した具体的なパターンをテスト
        let backend = TestBackend::new(130, 20); // 130文字幅でテスト
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|f| {
                let area = f.area();

                let mut result_list = ResultList::new();

                // ユーザーが報告した具体的なデータを再現
                let items = vec![
                    SearchResult {
                        file: "/test/file.jsonl".to_string(),
                        uuid: "msg_1".to_string(),
                        timestamp: "2025-07-25T19:19:00Z".to_string(),
                        session_id: "test-session".to_string(),
                        role: "assistant".to_string(),
                        text: "OK".to_string(),
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
                    SearchResult {
                        file: "/test/file.jsonl".to_string(),
                        uuid: "msg_3".to_string(),
                        timestamp: "2025-07-25T19:19:00Z".to_string(),
                        session_id: "test-session".to_string(),
                        role: "assistant".to_string(),
                        text: "".to_string(),  // 空のメッセージ
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
                        uuid: "msg_4".to_string(),
                        timestamp: "2025-07-25T19:19:00Z".to_string(),
                        session_id: "test-session".to_string(),
                        role: "system".to_string(),
                        text: "PostToolUse:Bash [ccth --debug] completed successfully: [dotenv@17.2.1] injecting env".to_string(),
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
                        uuid: "msg_5".to_string(),
                        timestamp: "2025-07-25T19:19:00Z".to_string(),
                        session_id: "test-session".to_string(),
                        role: "system".to_string(),
                        text: "Running PostToolUse:Bash...".to_string(),
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

        // 出力を詳しく分析
        println!("\nDetailed output analysis (width 130):");
        let mut found_pattern = false;

        for y in 0..20 {
            let line = extract_line(buffer, y, 130);
            if !line.trim().is_empty() {
                // データ行を探す
                if line.contains("system") || line.contains("assistant") {
                    println!("Line {}: {}", y, line);

                    // "│        │"パターンを探す
                    if let Some(pos) = line.find("│        │") {
                        found_pattern = true;
                        println!("  ⚠️ FOUND PATTERN at position {}", pos);

                        // パターンの前後を表示
                        let start = pos.saturating_sub(20);
                        let end = (pos + 30).min(line.len());
                        println!("  Context: '{}'", &line[start..end]);
                    }

                    // 行末の余分なスペースも確認
                    let trimmed = line.trim_end();
                    if trimmed != line {
                        let trailing = line.len() - trimmed.len();
                        println!("  {} trailing spaces", trailing);
                    }
                }
            }
        }

        if !found_pattern {
            // 全ての出力を表示して確認
            println!("\nFull output:");
            for y in 0..20 {
                let line = extract_line(buffer, y, 130);
                if !line.trim().is_empty() {
                    println!("{:2}: {}", y, line);
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
