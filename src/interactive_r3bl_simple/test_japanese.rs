#[cfg(test)]
mod tests {
    use super::super::app::*;
    use super::super::state::*;
    use crate::{SearchOptions, SearchResult};
    use crate::query::QueryCondition;
    use std::sync::Arc;
    use tokio::sync::Mutex;

    fn create_test_app() -> (SearchApp, Arc<Mutex<AppState>>) {
        let state = Arc::new(Mutex::new(AppState::new()));
        let options = SearchOptions::default();
        let app = SearchApp::new("test.jsonl".to_string(), options, state.clone());
        (app, state)
    }

    fn create_japanese_result(text: &str) -> SearchResult {
        SearchResult {
            file: "test.jsonl".to_string(),
            uuid: "test-uuid".to_string(),
            timestamp: "2024-01-01T00:00:00Z".to_string(),
            session_id: "test-session".to_string(),
            role: "user".to_string(),
            text: text.to_string(),
            has_tools: false,
            has_thinking: false,
            message_type: "text".to_string(),
            query: QueryCondition::Literal {
                pattern: "test".to_string(),
                case_sensitive: false,
            },
            project_path: "/test".to_string(),
            raw_json: None,
        }
    }

    #[tokio::test]
    async fn test_japanese_text_truncation() {
        let (app, state) = create_test_app();
        
        let mut state_lock = state.lock().await;
        
        // This is the exact text that caused the error
        let problematic_text = r##"[{"body":"# 概要

- ブランチ命名規則に従わないブランチを pre-push フックで弾く機能を追加
- プロジェクトの品質管理とブランチ管理の統一性を向上

# 技術的変更点概要

## 追加ファイル"##;
        
        state_lock.search_results.push(create_japanese_result(problematic_text));
        
        // This should not panic
        let output = app.render(&mut state_lock).await.unwrap();
        
        // Check that it contains proper ANSI escape sequences
        assert!(!output.is_empty()); // Should render something
        assert!(output.contains("\x1b[K")); // Clear line
        
        // The render function only shows the first line of the text,
        // and truncates it based on terminal width. The first line is:
        // [{"body":"# 概要
        // which is only 16 characters, so it won't be truncated
        assert!(output.contains("[{"));
        assert!(output.contains("概要"));
        
        // "追加ファイル" is on a later line, so it won't be shown
        // in the search results list view
        assert!(!output.contains("追加ファイル"));
        
        // The important thing is that it doesn't panic with the UTF-8 boundary error
    }

    #[tokio::test]
    async fn test_various_japanese_texts() {
        let (app, state) = create_test_app();
        
        let test_cases = vec![
            "これは短いテキストです",
            "とても長い日本語のテキストで、画面の幅を超えてしまうような文章です。このような長い文章でも正しく切り詰められることを確認します。",
            "絵文字も含む😀テキスト🎌です",
            "半角ｶﾀｶﾅも混ざったﾃｷｽﾄです",
            "English and 日本語 mixed text that is very long and should be truncated properly without errors",
        ];
        
        for text in test_cases {
            let mut state_lock = state.lock().await;
            state_lock.search_results.clear();
            state_lock.search_results.push(create_japanese_result(text));
            
            // None of these should panic
            let output = app.render(&mut state_lock).await.unwrap();
            
            // Basic checks
            assert!(output.contains("CCMS Search"));
            assert!(output.contains("Results: 1 found"));
        }
    }

    #[tokio::test]
    async fn test_emoji_and_complex_unicode() {
        let (app, state) = create_test_app();
        
        let mut state_lock = state.lock().await;
        
        // Complex Unicode including zero-width joiners
        let complex_text = "👨‍👩‍👧‍👦 Family emoji with ZWJ sequences and very long text that needs to be truncated properly without breaking the emoji";
        
        state_lock.search_results.push(create_japanese_result(complex_text));
        
        // Should not panic
        let output = app.render(&mut state_lock).await.unwrap();
        
        // The family emoji should be preserved
        assert!(output.contains("👨‍👩‍👧‍👦"));
    }

    #[tokio::test]
    async fn test_edge_case_exactly_77_chars() {
        let (app, state) = create_test_app();
        
        let mut state_lock = state.lock().await;
        
        // Create a string that's exactly 77 characters
        let text_77 = "あ".repeat(77);
        assert_eq!(text_77.chars().count(), 77);
        
        state_lock.search_results.push(create_japanese_result(&text_77));
        
        let output = app.render(&mut state_lock).await.unwrap();
        
        // The text might be truncated based on terminal width, but shouldn't panic
        // We can't assume 77 chars will be shown since it depends on terminal width
        assert!(!output.is_empty()); // Should render something
        assert!(output.contains("Results: 1 found"));
        // The important test is that it doesn't panic
    }

    #[tokio::test]
    async fn test_edge_case_78_chars() {
        let (app, state) = create_test_app();
        
        let mut state_lock = state.lock().await;
        
        // Create a string that's exactly 78 characters
        let text_78 = "あ".repeat(78);
        assert_eq!(text_78.chars().count(), 78);
        
        state_lock.search_results.push(create_japanese_result(&text_78));
        
        let output = app.render(&mut state_lock).await.unwrap();
        
        // The text will be truncated based on terminal width
        // We can't assume exact truncation behavior since it's dynamic
        assert!(!output.is_empty()); // Should render something
        assert!(output.contains("Results: 1 found"));
        // If the terminal is wide enough, it might show all 78 chars
        // If not, it will be truncated with ellipsis
        // The important test is that it doesn't panic
    }

    #[tokio::test]
    async fn test_multiline_japanese_text() {
        let (app, state) = create_test_app();
        
        let mut state_lock = state.lock().await;
        
        let multiline = "第一行の日本語テキスト
第二行の日本語テキスト
第三行の日本語テキスト";
        state_lock.search_results.push(create_japanese_result(multiline));
        
        let output = app.render(&mut state_lock).await.unwrap();
        
        // Should only show the first line
        assert!(output.contains("第一行の日本語テキスト"));
        assert!(!output.contains("第二行"));
    }

    #[tokio::test]
    async fn test_japanese_input() {
        let (mut app, state) = create_test_app();
        let mut state_lock = state.lock().await;

        // Test inputting Japanese characters
        let japanese_chars = vec!['こ', 'ん', 'に', 'ち', 'は'];
        
        for ch in japanese_chars {
            app.handle_input(ch, &mut state_lock).await.unwrap();
        }

        assert_eq!(state_lock.query, "こんにちは");
        assert!(state_lock.is_searching);
        assert!(state_lock.needs_render);
    }

    #[tokio::test]
    async fn test_mixed_input() {
        let (mut app, state) = create_test_app();
        let mut state_lock = state.lock().await;

        // Test mixed English and Japanese input
        let chars = vec!['h', 'e', 'l', 'l', 'o', ' ', '世', '界'];
        
        for ch in chars {
            app.handle_input(ch, &mut state_lock).await.unwrap();
        }

        assert_eq!(state_lock.query, "hello 世界");
        assert!(state_lock.is_searching);
        assert!(state_lock.needs_render);
    }

    #[tokio::test]
    async fn test_japanese_backspace() {
        let (mut app, state) = create_test_app();
        let mut state_lock = state.lock().await;

        // Input Japanese text
        for ch in "こんにちは".chars() {
            app.handle_input(ch, &mut state_lock).await.unwrap();
        }
        
        // Test backspace
        app.handle_input('\x08', &mut state_lock).await.unwrap();
        assert_eq!(state_lock.query, "こんにち");
        
        app.handle_input('\x08', &mut state_lock).await.unwrap();
        assert_eq!(state_lock.query, "こんに");
    }
}