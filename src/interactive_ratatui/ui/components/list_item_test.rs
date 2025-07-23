#[cfg(test)]
mod tests {
    use super::super::list_item::*;

    #[test]
    fn test_truncate_message() {
        // Test message truncation
        let short = "Hello";
        let truncated = truncate_message(short, 10);
        assert_eq!(truncated, "Hello");

        let long = "This is a very long message that should be truncated";
        let truncated = truncate_message(long, 20);
        assert_eq!(truncated, "This is a very lo...");

        // Test with newlines
        let multiline = "Line 1\nLine 2";
        let truncated = truncate_message(multiline, 20);
        assert_eq!(truncated, "Line 1 Line 2");
    }

    #[test]
    fn test_wrap_text() {
        // Test basic wrapping
        let wrapped = wrap_text("Hello world this is a test", 10);
        assert_eq!(wrapped, vec!["Hello", "world this", "is a test"]);

        // Test text that fits on one line
        let wrapped = wrap_text("Short", 10);
        assert_eq!(wrapped, vec!["Short"]);

        // Test empty text
        let wrapped = wrap_text("", 10);
        assert_eq!(wrapped, vec![""]);

        // Test very long word
        let wrapped = wrap_text("superlongwordthatdoesntfit", 10);
        assert_eq!(wrapped, vec!["superlongwordthatdoesntfit"]);

        // Test multiple spaces
        let wrapped = wrap_text("Hello    world", 20);
        assert_eq!(wrapped, vec!["Hello world"]);

        // Test zero width
        let wrapped = wrap_text("Hello", 0);
        assert_eq!(wrapped, Vec::<String>::new());

        // Test unicode text
        let wrapped = wrap_text("こんにちは 世界 です", 10);
        assert_eq!(wrapped, vec!["こんにちは 世界", "です"]);
    }

    #[test]
    fn test_unicode_truncation() {
        // Test that unicode is handled correctly
        let japanese = "こんにちは世界、これは長いメッセージです";
        let truncated = truncate_message(japanese, 10);
        assert_eq!(truncated, "こんにちは世界..."); // 7 chars + "..."

        let emoji = "🔍🎯💻🎨🔧 Search tool";
        let truncated = truncate_message(emoji, 10);
        assert_eq!(truncated, "🔍🎯💻🎨🔧 S...");
    }
}
