/// Fast case-insensitive string comparison utilities
/// Optimized for ASCII strings but handles Unicode correctly
/// Trait for fast lowercase comparison
pub trait FastLowercase {
    fn fast_to_lowercase(&self) -> String;
    fn fast_contains_ignore_case(&self, pattern: &str) -> bool;
}

impl FastLowercase for str {
    #[inline]
    fn fast_to_lowercase(&self) -> String {
        // Check if the string is ASCII-only for fast path
        if self.is_ascii() {
            // ASCII fast path - 2-7x faster than to_lowercase()
            self.to_ascii_lowercase()
        } else {
            // Unicode fallback
            self.to_lowercase()
        }
    }
    
    #[inline]
    fn fast_contains_ignore_case(&self, pattern: &str) -> bool {
        // If both strings are ASCII, use optimized comparison
        if self.is_ascii() && pattern.is_ascii() {
            // Convert to bytes for SIMD-friendly operations
            let self_bytes = self.as_bytes();
            let pattern_bytes = pattern.as_bytes();
            
            if pattern_bytes.is_empty() {
                return true;
            }
            
            if self_bytes.len() < pattern_bytes.len() {
                return false;
            }
            
            // ASCII case-insensitive substring search
            'outer: for i in 0..=(self_bytes.len() - pattern_bytes.len()) {
                for j in 0..pattern_bytes.len() {
                    let a = self_bytes[i + j];
                    let b = pattern_bytes[j];
                    
                    // Fast ASCII case-insensitive comparison
                    if a != b && !a.eq_ignore_ascii_case(&b) {
                        continue 'outer;
                    }
                }
                return true;
            }
            false
        } else {
            // Unicode fallback
            self.to_lowercase().contains(&pattern.to_lowercase())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_fast_to_lowercase_ascii() {
        assert_eq!("HELLO".fast_to_lowercase(), "hello");
        assert_eq!("Hello World".fast_to_lowercase(), "hello world");
        assert_eq!("123ABC".fast_to_lowercase(), "123abc");
    }
    
    #[test]
    fn test_fast_to_lowercase_unicode() {
        assert_eq!("CAFÉ".fast_to_lowercase(), "café");
        assert_eq!("Straße".fast_to_lowercase(), "straße");
        assert_eq!("МОСКВА".fast_to_lowercase(), "москва");
    }
    
    #[test]
    fn test_fast_contains_ignore_case_ascii() {
        assert!("Hello World".fast_contains_ignore_case("hello"));
        assert!("HELLO WORLD".fast_contains_ignore_case("world"));
        assert!("Testing 123".fast_contains_ignore_case("ING 1"));
        assert!(!"Hello".fast_contains_ignore_case("goodbye"));
    }
    
    #[test]
    fn test_fast_contains_ignore_case_unicode() {
        assert!("Café au lait".fast_contains_ignore_case("café"));
        assert!("МОСКВА".fast_contains_ignore_case("москва"));
        assert!(!"Hello".fast_contains_ignore_case("привет"));
    }
    
    #[test]
    fn test_edge_cases() {
        assert!("".fast_contains_ignore_case(""));
        assert!("Hello".fast_contains_ignore_case(""));
        assert!(!"".fast_contains_ignore_case("Hello"));
    }
}