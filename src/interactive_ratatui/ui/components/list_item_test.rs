#[cfg(test)]
mod tests {
    use super::super::list_item::*;

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
}
