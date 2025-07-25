#[cfg(test)]
mod tests {
    use crate::interactive_ratatui::ui::components::{Component, result_list::ResultList};
    use crate::query::condition::SearchResult;
    use ratatui::{Terminal, backend::TestBackend, buffer::Buffer};

    #[test]
    fn test_exact_user_issue() {
        // ユーザーの画面を再現するため、特定の幅でテスト
        // "injecting env ("の長さを計算し、その位置で切れるような幅を試す
        let test_widths = vec![
            90, // 狭い画面
            95, 100, 105, 110, 115,
        ];

        for width in test_widths {
            println!("\n=== Testing with width {} ===", width);

            let backend = TestBackend::new(width, 15);
            let mut terminal = Terminal::new(backend).unwrap();

            terminal
                .draw(|f| {
                    let area = f.area();

                    let mut result_list = ResultList::new();

                    // ユーザーが報告したテキストを正確に再現
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
                    result_list.set_results(items);
                    result_list.render(f, area);
                })
                .unwrap();

            let buffer = terminal.backend().buffer();

            // 特定の行（データ行）を詳しく分析
            for y in 4..10 {
                // データ行は通常4行目から始まる
                let line = extract_line(buffer, y, width);
                if line.contains("system") && line.contains("injecting env") {
                    println!("Found data line at y={}", y);

                    // 文字ごとに分析
                    let chars: Vec<char> = line.chars().collect();

                    // "injecting env ("の位置を探す
                    if let Some(pos) = line.find("injecting env (") {
                        let end_pos = pos + "injecting env (".len();
                        println!("  'injecting env (' found at position {}-{}", pos, end_pos);

                        // その後の文字を確認
                        if end_pos < chars.len() {
                            println!("  Characters after 'injecting env (':");
                            for i in end_pos..chars.len().min(end_pos + 20) {
                                println!("    [{}]: '{}'", i, chars[i]);
                                if chars[i] == '│' {
                                    println!("      ⚠️ Found '│' at position {}", i);

                                    // その後に "        │" があるか確認
                                    if i + 9 < chars.len() {
                                        let next_chars: String =
                                            chars[i + 1..i + 10].iter().collect();
                                        if next_chars == "        │" {
                                            println!(
                                                "      🚨 FOUND THE EXACT PATTERN: '│        │'"
                                            );
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // 行全体を出力
                    println!("  Full line: '{}'", line);

                    // パターンを探す
                    if line.contains("│        │") {
                        println!("  ⚠️ CONTAINS THE PROBLEMATIC PATTERN");
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
