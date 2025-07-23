#[cfg(test)]
mod tests {

    #[test]
    fn test_multibyte_char_length_comparison() {
        // Confirm difference between byte length and character count for Japanese text
        let japanese_text = "こんにちは世界";
        let byte_len = japanese_text.len();
        let char_len = japanese_text.chars().count();

        // Japanese is 3 bytes per character, so byte length is 3 times character count
        assert_eq!(byte_len, 21); // 7 characters × 3 bytes = 21 bytes
        assert_eq!(char_len, 7); // 7 characters

        // Text containing emoji
        let emoji_text = "Hello 🌍";
        let emoji_byte_len = emoji_text.len();
        let emoji_char_len = emoji_text.chars().count();

        assert_eq!(emoji_byte_len, 10); // "Hello " = 6 + 🌍 = 4 = 10 bytes
        assert_eq!(emoji_char_len, 7); // 7 characters
    }

    #[test]
    fn test_search_view_truncation_issue() {
        // Reproduce issue from search_view.rs Line 104
        let japanese_text =
            "これは日本語のテキストです。もっと長い文章を書いてバイト長の問題を確認します。";

        // Try to truncate at 80 characters (search_view.rs logic)
        let _truncated_chars = japanese_text.chars().take(80).collect::<String>();
        let is_too_long_by_bytes = japanese_text.len() > 80; // Compare by byte length
        let is_too_long_by_chars = japanese_text.chars().count() > 80; // Compare by character count

        // Japanese 39 characters = 117 bytes
        assert_eq!(japanese_text.chars().count(), 39);
        assert_eq!(japanese_text.len(), 117);

        // When comparing by byte length, even 39 characters is judged as "too long"
        assert!(is_too_long_by_bytes); // 117 > 80
        assert!(!is_too_long_by_chars); // 39 < 80

        // This causes unnecessary "..." to be added even for short text
    }

    #[test]
    fn test_clipboard_preview_truncation() {
        // Reproduce issue from ui/mod.rs Line 470-471
        let japanese_msg = "これは日本語のメッセージです";

        // Judge by 50 bytes (current implementation)
        let should_truncate_by_bytes = japanese_msg.len() > 50;
        // Judge by 50 characters (correct implementation)
        let should_truncate_by_chars = japanese_msg.chars().count() > 50;

        // 14 characters = 42 bytes
        assert_eq!(japanese_msg.chars().count(), 14);
        assert_eq!(japanese_msg.len(), 42);

        // Should show preview since it's 14 characters, and byte length check is OK here
        assert!(!should_truncate_by_bytes); // 42 < 50
        assert!(!should_truncate_by_chars); // 14 < 50

        // However, with slightly longer Japanese text...
        let longer_japanese = "これは少し長めの日本語メッセージの例です。";
        assert_eq!(longer_japanese.chars().count(), 21);
        assert_eq!(longer_japanese.len(), 63);

        // Even 21 characters get truncated with byte length check
        assert!(longer_japanese.len() > 50); // 63 > 50
        assert!(longer_japanese.chars().count() < 50); // 21 < 50
    }

    #[test]
    fn test_text_editing_with_multibyte() {
        // Confirm push/pop works at character level
        let mut query = String::from("Hello");
        query.push('世');
        assert_eq!(query, "Hello世");

        query.pop();
        assert_eq!(query, "Hello");

        // Also confirm with emoji
        query.push('🌍');
        assert_eq!(query, "Hello🌍");

        query.pop();
        assert_eq!(query, "Hello");
    }

    #[test]
    fn test_cursor_position_issue() {
        // Issue due to lack of cursor position management
        let _text = "こんにちは世界";

        // Current implementation can only add characters at the end of text
        // No handling for inserting in the middle

        // Example: Want to input "X" at "konni[cursor]chiha sekai"
        // Current implementation: text.push('X') → "konnichiha sekaiX"
        // Expected result: "konniXchiha sekai"

        // This is difficult to reproduce in a test, but shows a missing implementation
    }
}
