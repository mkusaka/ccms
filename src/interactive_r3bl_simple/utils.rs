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
}