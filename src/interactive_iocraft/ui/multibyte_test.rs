#[cfg(test)]
mod tests {

    #[test]
    fn test_multibyte_char_length_comparison() {
        // 日本語テキストのバイト長と文字数の違いを確認
        let japanese_text = "こんにちは世界";
        let byte_len = japanese_text.len();
        let char_len = japanese_text.chars().count();

        // 日本語は1文字3バイトなので、バイト長は文字数の3倍
        assert_eq!(byte_len, 21); // 7文字 × 3バイト = 21バイト
        assert_eq!(char_len, 7); // 7文字

        // 絵文字を含むテキスト
        let emoji_text = "Hello 🌍";
        let emoji_byte_len = emoji_text.len();
        let emoji_char_len = emoji_text.chars().count();

        assert_eq!(emoji_byte_len, 10); // "Hello " = 6 + 🌍 = 4 = 10バイト
        assert_eq!(emoji_char_len, 7); // 7文字
    }

    #[test]
    fn test_search_view_truncation_issue() {
        // search_view.rsのLine 104の問題を再現
        let japanese_text =
            "これは日本語のテキストです。もっと長い文章を書いてバイト長の問題を確認します。";

        // 80文字で切り詰めようとする（search_view.rsのロジック）
        let _truncated_chars = japanese_text.chars().take(80).collect::<String>();
        let is_too_long_by_bytes = japanese_text.len() > 80; // バイト長で比較
        let is_too_long_by_chars = japanese_text.chars().count() > 80; // 文字数で比較

        // 日本語39文字 = 117バイト
        assert_eq!(japanese_text.chars().count(), 39);
        assert_eq!(japanese_text.len(), 117);

        // バイト長で比較すると、39文字でも"too long"と判定される
        assert!(is_too_long_by_bytes); // 117 > 80
        assert!(!is_too_long_by_chars); // 39 < 80

        // これにより、短いテキストでも不要な"..."が追加される
    }

    #[test]
    fn test_clipboard_preview_truncation() {
        // ui/mod.rs Line 470-471の問題を再現
        let japanese_msg = "これは日本語のメッセージです";

        // 50バイトで判定（現在の実装）
        let should_truncate_by_bytes = japanese_msg.len() > 50;
        // 50文字で判定（正しい実装）
        let should_truncate_by_chars = japanese_msg.chars().count() > 50;

        // 14文字 = 42バイト
        assert_eq!(japanese_msg.chars().count(), 14);
        assert_eq!(japanese_msg.len(), 42);

        // 14文字なのでプレビュー表示すべきだが、バイト長判定だと問題ない
        assert!(!should_truncate_by_bytes); // 42 < 50
        assert!(!should_truncate_by_chars); // 14 < 50

        // しかし、もう少し長い日本語だと...
        let longer_japanese = "これは少し長めの日本語メッセージの例です。";
        assert_eq!(longer_japanese.chars().count(), 21);
        assert_eq!(longer_japanese.len(), 63);

        // 21文字でもバイト長判定だと切り詰められる
        assert!(longer_japanese.len() > 50); // 63 > 50
        assert!(longer_japanese.chars().count() < 50); // 21 < 50
    }

    #[test]
    fn test_text_editing_with_multibyte() {
        // push/popが文字単位で動作することを確認
        let mut query = String::from("Hello");
        query.push('世');
        assert_eq!(query, "Hello世");

        query.pop();
        assert_eq!(query, "Hello");

        // 絵文字でも確認
        query.push('🌍');
        assert_eq!(query, "Hello🌍");

        query.pop();
        assert_eq!(query, "Hello");
    }

    #[test]
    fn test_cursor_position_issue() {
        // カーソル位置管理がないことによる問題
        let _text = "こんにちは世界";

        // 現在の実装では、テキストの最後にしか文字を追加できない
        // 途中に挿入したい場合の処理がない

        // 例：「こんに[カーソル]ちは世界」で「X」を入力したい
        // 現在の実装: text.push('X') → "こんにちは世界X"
        // 期待する結果: "こんにXちは世界"

        // これはテストで再現するのが難しいが、実装の欠落を示している
    }
}
