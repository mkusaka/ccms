#[cfg(test)]
mod tests {
    #[test]
    fn test_unicode_width_calculation() {
        // 日本語テキストの幅計算を確認
        let test_strings = vec![
            "Short text",
            "This is a longer text that might cause issues",
            "検索結果から、RatatuiのTableウィジェットには余分なスペースの配分に関する問題があることがわかりました。",
            "PreToolUse:Bash [ccth --debug] completed successfully: [dotenv@17.2.1] injecting env (",
        ];
        
        println!("\nUnicode width test:");
        for s in test_strings {
            let char_count = s.chars().count();
            let byte_count = s.len();
            println!("Text: '{}'", s);
            println!("  Byte count: {}, Char count: {}", byte_count, char_count);
            
            // 日本語文字を含む場合の詳細
            if s.contains(|c: char| c > '\u{007F}') {
                println!("  Contains non-ASCII characters");
                
                // 各文字の詳細を確認
                let mut display_width = 0;
                for (i, c) in s.chars().take(10).enumerate() {
                    // 簡易的な幅計算（CJK文字は2、それ以外は1）
                    let c_width = if c > '\u{007F}' && c < '\u{10000}' {
                        // CJK文字の範囲（簡易判定）
                        if c >= '\u{3000}' && c <= '\u{9FFF}' || // CJK統合漢字
                           c >= '\u{3040}' && c <= '\u{309F}' || // ひらがな
                           c >= '\u{30A0}' && c <= '\u{30FF}'    // カタカナ
                        {
                            2
                        } else {
                            1
                        }
                    } else {
                        1
                    };
                    display_width += c_width;
                    println!("    [{}] '{}' = width {} (unicode: U+{:04X})", i, c, c_width, c as u32);
                }
                println!("  Estimated display width for first 10 chars: {}", display_width);
            }
        }
    }
}