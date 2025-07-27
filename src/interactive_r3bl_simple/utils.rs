//! Utilities for safe string operations

/// Truncate a string to a maximum number of characters (not bytes).
/// This is safe for multi-byte UTF-8 characters like Japanese text.
pub fn truncate_str(s: &str, max_chars: usize) -> String {
    let chars: Vec<_> = s.chars().collect();
    if chars.len() > max_chars {
        let truncated: String = chars.into_iter().take(max_chars).collect();
        format!("{truncated}...")
    } else {
        s.to_string()
    }
}

/// Highlight matching text with ANSI color codes
/// Returns the text with matched portions highlighted in yellow background
pub fn highlight_matches(text: &str, query: &str) -> String {
    if query.is_empty() {
        return text.to_string();
    }
    
    // Case-insensitive search
    let lower_text = text.to_lowercase();
    let lower_query = query.to_lowercase();
    
    let mut result = String::new();
    let mut last_end = 0;
    
    // Find all matches
    for (start, _) in lower_text.match_indices(&lower_query) {
        // Add text before the match
        result.push_str(&text[last_end..start]);
        
        // Add highlighted match
        // Yellow background with black text
        result.push_str("\x1b[43;30m");
        result.push_str(&text[start..start + query.len()]);
        result.push_str("\x1b[0m");
        
        last_end = start + query.len();
    }
    
    // Add remaining text
    result.push_str(&text[last_end..]);
    
    result
}

/// Truncate a string with highlighted matches preserved
/// This ensures highlights are maintained even when truncating
pub fn truncate_str_with_highlight(s: &str, query: &str, max_chars: usize) -> String {
    if s.is_empty() {
        return String::new();
    }
    
    // First apply highlighting
    let highlighted = highlight_matches(s, query);
    
    // Count visible characters (excluding ANSI codes)
    let mut visible_chars = 0;
    let mut in_ansi = false;
    let mut result = String::new();
    let mut chars_iter = highlighted.chars().peekable();
    
    while let Some(ch) = chars_iter.next() {
        if ch == '\x1b' && chars_iter.peek() == Some(&'[') {
            in_ansi = true;
        }
        
        if in_ansi {
            result.push(ch);
            if ch == 'm' {
                in_ansi = false;
            }
        } else {
            if visible_chars >= max_chars {
                result.push_str("...");
                break;
            }
            result.push(ch);
            visible_chars += 1;
        }
    }
    
    // Ensure we close any open ANSI sequences
    if in_ansi {
        result.push_str("\x1b[0m");
    }
    
    result
}

/// Truncate a string to fit within a maximum byte length while respecting UTF-8 boundaries.
/// This ensures we never split a multi-byte character.
pub fn truncate_str_bytes(s: &str, max_bytes: usize) -> &str {
    if s.len() <= max_bytes {
        return s;
    }
    
    let mut end = max_bytes;
    while !s.is_char_boundary(end) {
        end -= 1;
    }
    &s[..end]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate_str_ascii() {
        let text = "Hello, world!";
        assert_eq!(truncate_str(text, 5), "Hello...");
        assert_eq!(truncate_str(text, 20), "Hello, world!");
    }

    #[test]
    fn test_truncate_str_japanese() {
        let text = "こんにちは世界です";
        assert_eq!(truncate_str(text, 5), "こんにちは...");
        assert_eq!(truncate_str(text, 10), "こんにちは世界です");
    }

    #[test]
    fn test_truncate_str_mixed() {
        let text = "Hello こんにちは World";
        // Counting characters: "Hello " (6) + "こんにち" (4) = 10 chars
        assert_eq!(truncate_str(text, 10), "Hello こんにち...");
        assert_eq!(truncate_str(text, 20), "Hello こんにちは World");
    }

    #[test]
    fn test_truncate_str_emoji() {
        let text = "👨‍👩‍👧‍👦 Family emoji test";
        // The family emoji is made of 7 chars: 👨 + ZWJ + 👩 + ZWJ + 👧 + ZWJ + 👦
        // When truncating to 3 chars, we get: 👨 + ZWJ + 👩
        assert_eq!(truncate_str(text, 3), "👨\u{200d}👩...");
        // The full text is more than 20 chars, so it gets truncated
        assert_eq!(truncate_str(text, 30), "👨‍👩‍👧‍👦 Family emoji test");
    }

    #[test]
    fn test_truncate_str_bytes_ascii() {
        let text = "Hello, world!";
        assert_eq!(truncate_str_bytes(text, 5), "Hello");
        assert_eq!(truncate_str_bytes(text, 20), "Hello, world!");
    }

    #[test]
    fn test_truncate_str_bytes_japanese() {
        // Each Japanese character is 3 bytes in UTF-8
        let text = "こんにちは"; // 15 bytes total
        assert_eq!(truncate_str_bytes(text, 6), "こん"); // 6 bytes = 2 chars
        assert_eq!(truncate_str_bytes(text, 7), "こん"); // 7 bytes -> rounds down to 6
        assert_eq!(truncate_str_bytes(text, 9), "こんに"); // 9 bytes = 3 chars
        assert_eq!(truncate_str_bytes(text, 20), "こんにちは"); // Full string
    }

    #[test]
    fn test_truncate_str_bytes_boundary() {
        let text = "aこb"; // 'a' = 1 byte, 'こ' = 3 bytes, 'b' = 1 byte
        assert_eq!(truncate_str_bytes(text, 1), "a");
        assert_eq!(truncate_str_bytes(text, 2), "a"); // Can't include partial 'こ'
        assert_eq!(truncate_str_bytes(text, 3), "a"); // Still can't include partial 'こ'
        assert_eq!(truncate_str_bytes(text, 4), "aこ"); // Now we can include full 'こ'
        assert_eq!(truncate_str_bytes(text, 5), "aこb");
    }
    
    #[test]
    fn test_highlight_matches_basic() {
        let text = "Hello world, hello again";
        let highlighted = highlight_matches(text, "hello");
        // Should highlight both "Hello" and "hello" (case-insensitive)
        assert!(highlighted.contains("\x1b[43;30mHello\x1b[0m"));
        assert!(highlighted.contains("\x1b[43;30mhello\x1b[0m"));
    }
    
    #[test]
    fn test_highlight_matches_japanese() {
        let text = "こんにちは世界、こんにちは";
        let highlighted = highlight_matches(text, "こんにちは");
        // Should highlight both occurrences
        assert_eq!(highlighted.matches("\x1b[43;30mこんにちは\x1b[0m").count(), 2);
    }
    
    #[test]
    fn test_highlight_matches_empty_query() {
        let text = "Hello world";
        let highlighted = highlight_matches(text, "");
        assert_eq!(highlighted, "Hello world");
    }
    
    #[test]
    fn test_truncate_str_with_highlight() {
        let text = "Hello world, hello again";
        let truncated = truncate_str_with_highlight(text, "hello", 15);
        // Should truncate but preserve the highlight
        assert!(truncated.contains("\x1b[43;30mHello\x1b[0m"));
        assert!(truncated.contains("..."));
        
        // Verify visible character count (excluding ANSI codes)
        let visible: String = truncated
            .chars()
            .scan(false, |in_ansi, ch| {
                if ch == '\x1b' {
                    *in_ansi = true;
                    Some(None)
                } else if *in_ansi && ch == 'm' {
                    *in_ansi = false;
                    Some(None)
                } else if *in_ansi {
                    Some(None)
                } else {
                    Some(Some(ch))
                }
            })
            .flatten()
            .collect();
        assert!(visible.len() <= 18); // 15 chars + "..."
    }
    
    #[test]
    fn test_truncate_str_with_highlight_japanese() {
        let text = "こんにちは世界です";
        let truncated = truncate_str_with_highlight(text, "世界", 7);
        // Should highlight "世界" even when truncating
        assert!(truncated.contains("\x1b[43;30m世界\x1b[0m"));
        assert!(truncated.contains("..."));
    }
}