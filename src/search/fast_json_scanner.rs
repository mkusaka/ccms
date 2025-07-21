/// Fast JSON scanner that extracts content without full parsing for initial filtering
pub struct FastJsonScanner;

impl FastJsonScanner {
    /// Quick scan to check if a line might contain matching content
    /// Returns true if the line should be parsed fully
    pub fn might_contain(json_line: &str, query_hint: &str) -> bool {
        // Skip obviously non-matching lines
        if json_line.len() < 50 {
            return false;
        }
        
        // Quick case-insensitive check for common patterns
        let lower_line = json_line.to_lowercase();
        let lower_query = query_hint.to_lowercase();
        
        lower_line.contains(&lower_query)
    }
    
    /// Extract text content from JSON without full parsing
    /// This is a fast but imprecise method for initial filtering
    pub fn extract_text_content(json_line: &str) -> Option<String> {
        // Look for content field patterns
        if let Some(content_start) = json_line.find(r#""content":"#) {
            let start = content_start + 11;
            let remaining = &json_line[start..];
            
            // Find the end of the content string
            let mut escaped = false;
            let mut end_pos = None;
            
            for (i, ch) in remaining.char_indices() {
                if escaped {
                    escaped = false;
                    continue;
                }
                
                match ch {
                    '\\' => escaped = true,
                    '"' => {
                        end_pos = Some(i);
                        break;
                    }
                    _ => {}
                }
            }
            
            if let Some(end) = end_pos {
                return Some(remaining[..end].to_string());
            }
        }
        
        // Try array content pattern
        if let Some(content_start) = json_line.find(r#""content":[{"#) {
            // Extract text from content array
            if let Some(text_start) = json_line[content_start..].find(r#""text":"#) {
                let start = content_start + text_start + 8;
                let remaining = &json_line[start..];
                
                let mut escaped = false;
                let mut end_pos = None;
                
                for (i, ch) in remaining.char_indices() {
                    if escaped {
                        escaped = false;
                        continue;
                    }
                    
                    match ch {
                        '\\' => escaped = true,
                        '"' => {
                            end_pos = Some(i);
                            break;
                        }
                        _ => {}
                    }
                }
                
                if let Some(end) = end_pos {
                    return Some(remaining[..end].to_string());
                }
            }
        }
        
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_might_contain() {
        let json = r#"{"type":"user","content":"Hello world"}"#;
        assert!(FastJsonScanner::might_contain(json, "hello"));
        assert!(!FastJsonScanner::might_contain(json, "goodbye"));
    }
    
    #[test]
    fn test_extract_text_content() {
        let json = r#"{"type":"user","content":"Hello world"}"#;
        let content = FastJsonScanner::extract_text_content(json);
        assert_eq!(content, Some("Hello world".to_string()));
        
        let json_array = r#"{"type":"user","content":[{"type":"text","text":"Hello array"}]}"#;
        let content = FastJsonScanner::extract_text_content(json_array);
        assert_eq!(content, Some("Hello array".to_string()));
    }
    
    #[test]
    fn test_escaped_quotes() {
        let json = r#"{"type":"user","content":"Say \"Hello\" to the world"}"#;
        let content = FastJsonScanner::extract_text_content(json);
        assert_eq!(content, Some(r#"Say \"Hello\" to the world"#.to_string()));
    }
}