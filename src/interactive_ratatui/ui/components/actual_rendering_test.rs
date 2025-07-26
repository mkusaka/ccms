#[cfg(test)]
mod tests {
    use crate::interactive_ratatui::ui::components::{Component, result_list::ResultList};
    use crate::query::condition::SearchResult;
    use ratatui::{Terminal, backend::TestBackend, buffer::Buffer};

    #[test]
    fn test_actual_user_data_rendering() {
        // ユーザーの実際のデータを再現
        let backend = TestBackend::new(120, 30);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|f| {
                let area = f.area();

                let mut result_list = ResultList::new();

                // ユーザーの画面に表示されているデータを再現
                let items = vec![
                    SearchResult {
                        file: "/test/file.jsonl".to_string(),
                        uuid: "msg_1".to_string(),
                        timestamp: "2025-07-25T19:32:00Z".to_string(),
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
                        uuid: "msg_2".to_string(),
                        timestamp: "2025-07-25T19:32:00Z".to_string(),
                        session_id: "test-session".to_string(),
                        role: "assistant".to_string(),
                        text: "".to_string(),
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
                        timestamp: "2025-07-25T19:32:00Z".to_string(),
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
                    SearchResult {
                        file: "/test/file.jsonl".to_string(),
                        uuid: "msg_4".to_string(),
                        timestamp: "2025-07-25T19:32:00Z".to_string(),
                        session_id: "test-session".to_string(),
                        role: "assistant".to_string(),
                        text: "検索結果から、RatatuiのTableウィジェットには余分なスペースの配分に関する問題があることがわかりました。".to_string(),
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
                
                // 選択位置を142に設定
                result_list.set_results(items);
                result_list.set_selected_index(141); // 0-indexed
                
                result_list.render(f, area);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();

        // 出力を詳しく分析
        println!("\nActual data rendering analysis:");
        
        // 各行を詳しく調べる
        for y in 0..30 {
            let line = extract_line(buffer, y, 120);
            if !line.trim().is_empty() {
                println!("Line {}: {}", y, line);
                
                // データ行を探す
                if line.contains("system") || line.contains("assistant") {
                    // 行の文字を分析
                    let chars: Vec<char> = line.chars().collect();
                    
                    // 表示可能な範囲を確認
                    if line.contains("│") {
                        let first_bar = line.find("│").unwrap();
                        let last_bar = line.rfind("│").unwrap();
                        let content_width = last_bar - first_bar - 1;
                        
                        println!("  Content area: {} to {} (width: {})", first_bar, last_bar, content_width);
                        
                        // 3つの列の区切りを探す
                        let content_chars: Vec<char> = chars.iter()
                            .skip(chars.iter().position(|&c| c == '│').unwrap() + 1)
                            .take_while(|&&c| c != '│')
                            .cloned()
                            .collect();
                        let content: String = content_chars.iter().collect();
                        
                        println!("  Content between bars (length={}): '{}'", content.len(), content);
                        
                        // 右端の余分なスペースを確認
                        if content.trim_end() != content {
                            let trailing = content.len() - content.trim_end().len();
                            println!("  WARNING: {} trailing spaces before right border", trailing);
                        }
                        
                        // 固定幅の列を確認
                        if content_chars.len() >= 11 {
                            let timestamp: String = content_chars[0..11].iter().collect();
                            println!("  Timestamp (11 chars): '{}'", timestamp);
                        }
                        
                        if content_chars.len() >= 22 {
                            let role: String = content_chars[11..22].iter().collect();
                            println!("  Role (11 chars): '{}'", role.trim());
                        }
                        
                        if content_chars.len() > 22 {
                            let text: String = content_chars[22..].iter().collect();
                            println!("  Text content: '{}'", text);
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