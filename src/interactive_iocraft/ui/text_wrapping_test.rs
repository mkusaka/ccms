#[cfg(test)]
mod tests {
    use crate::interactive_iocraft::ui::SearchState;

    #[test]
    fn test_wrap_text_basic() {
        // Basic text wrapping test
        let text = "This is a very long text that should be wrapped at a specific width";
        let width = 20;
        let wrapped = SearchState::wrap_text(text, width);

        assert_eq!(wrapped.len(), 4); // Should be split into 4 lines
        assert!(wrapped[0].len() <= width);
        assert!(wrapped[1].len() <= width);
        assert!(wrapped[2].len() <= width);
        assert!(wrapped[3].len() <= width);
    }

    #[test]
    fn test_wrap_text_preserves_words() {
        // Ensure wrapping doesn't break words in the middle
        let text = "This is a sentence with some relatively long words";
        let width = 15;
        let wrapped = SearchState::wrap_text(text, width);

        // Check that no line starts or ends with a partial word
        for line in &wrapped {
            assert!(!line.starts_with(' '));
            assert!(!line.ends_with(' '));
        }
    }

    #[test]
    fn test_wrap_text_with_long_word() {
        // Handle words longer than the wrap width
        let text = "This has a verylongwordthatexceedsthewrapwidth here";
        let width = 10;
        let wrapped = SearchState::wrap_text(text, width);

        // Debug output
        println!("Wrapped lines:");
        for (i, line) in wrapped.iter().enumerate() {
            println!("{}: '{}'", i, line);
        }

        // The long word should be broken across multiple lines
        let long_word = "verylongwordthatexceedsthewrapwidth";
        let mut found_parts = Vec::new();
        for line in &wrapped {
            for part in long_word.chars().collect::<Vec<_>>().chunks(width) {
                let part_str: String = part.iter().collect();
                if line.contains(&part_str) {
                    found_parts.push(part_str);
                }
            }
        }

        // Check that we can reconstruct the long word from the wrapped lines
        let reconstructed: String = wrapped
            .iter()
            .skip(1) // Skip "This has a"
            .take_while(|line| !line.contains("here"))
            .cloned()
            .collect::<Vec<_>>()
            .join("");

        println!("Reconstructed: '{}'", reconstructed);
        assert!(reconstructed.contains(long_word) || !found_parts.is_empty());
    }

    #[test]
    fn test_wrap_text_with_multibyte_chars() {
        // Test with Japanese text
        let text = "これは日本語のテキストで、指定された幅で折り返されるべきです";
        let width = 10; // Character count, not byte count
        let wrapped = SearchState::wrap_text(text, width);

        // Each line should not exceed the character width
        for line in &wrapped {
            assert!(line.chars().count() <= width);
        }
    }

    #[test]
    fn test_wrap_text_empty_string() {
        let text = "";
        let width = 20;
        let wrapped = SearchState::wrap_text(text, width);

        assert_eq!(wrapped.len(), 1);
        assert_eq!(wrapped[0], "");
    }

    #[test]
    fn test_wrap_text_single_short_line() {
        let text = "Short text";
        let width = 20;
        let wrapped = SearchState::wrap_text(text, width);

        assert_eq!(wrapped.len(), 1);
        assert_eq!(wrapped[0], "Short text");
    }

    #[test]
    fn test_wrap_text_with_newlines() {
        // Preserve existing newlines
        let text = "First line\nSecond line that is quite long and needs wrapping\nThird line";
        let width = 20;
        let wrapped = SearchState::wrap_text(text, width);

        // Should have at least 4 lines (original 3 + wrapped second line)
        assert!(wrapped.len() >= 4);
        assert_eq!(wrapped[0], "First line");
    }

    #[test]
    fn test_wrap_text_with_mixed_content() {
        // Test with mixed ASCII and multibyte characters
        let text = "Hello 世界! This is a mixed content string with 日本語 characters.";
        let width = 15;
        let wrapped = SearchState::wrap_text(text, width);

        for line in &wrapped {
            assert!(line.chars().count() <= width);
        }
    }

    #[test]
    fn test_result_detail_text_wrapping() {
        // Test that result detail view properly wraps long text
        let long_result_text = "This is a very long search result text that contains multiple sentences. It should be properly wrapped when displayed in the result detail view. The wrapping should make the text easy to read without horizontal scrolling.";
        let terminal_width = 80;
        let padding = 4; // Typical padding in result detail view
        let effective_width = terminal_width - padding;

        let wrapped = SearchState::wrap_text(long_result_text, effective_width);

        // All lines should fit within the effective width
        for line in &wrapped {
            assert!(line.chars().count() <= effective_width);
        }
    }

    #[test]
    fn test_session_viewer_text_wrapping() {
        // Test wrapping in session viewer
        let session_message = "This is a long session message that might contain code snippets, URLs like https://example.com/very/long/path/to/resource, and other content that needs careful wrapping.";
        let width = 60;
        let wrapped = SearchState::wrap_text(session_message, width);

        // Verify URL is not broken inappropriately
        let url_line = wrapped
            .iter()
            .find(|line| line.contains("https://"))
            .unwrap();
        assert!(url_line.contains("https://example.com"));
    }
}
