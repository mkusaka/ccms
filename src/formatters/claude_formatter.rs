use crate::formatters::tool_parser::{ParsedContent, parse_raw_json, parse_text_content};
use crate::query::condition::SearchResult;

/// Display modes for Claude Code formatting
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DisplayMode {
    /// Single-line format for list views
    List,
    /// Multi-line with truncation for preview views
    Preview { max_lines: usize },
    /// Complete content for detail views
    Detail,
}

/// Claude Code special markers
pub const TOOL_MARKER: char = '⏺';
pub const RESULT_MARKER: char = '⎿';
pub const THINKING_MARKER: char = '✻';
pub const TRUNCATION_MARKER: &str = "…";

/// Format a SearchResult according to Claude Code style
pub fn format_search_result(result: &SearchResult, mode: DisplayMode) -> String {
    // Parse the content to extract tool executions
    let parsed = if let Some(raw_json) = &result.raw_json {
        parse_raw_json(raw_json)
    } else {
        parse_text_content(&result.text)
    };
    
    match mode {
        DisplayMode::List => format_list_view(&parsed),
        DisplayMode::Preview { max_lines } => format_preview_view(&parsed, max_lines),
        DisplayMode::Detail => format_detail_view(&parsed),
    }
}

/// Convenience function to format for list view
pub fn format_for_list(result: &SearchResult) -> String {
    format_search_result(result, DisplayMode::List)
}

/// Convenience function to format for preview with default line count
pub fn format_for_preview(result: &SearchResult, max_lines: usize) -> String {
    format_search_result(result, DisplayMode::Preview { max_lines })
}

/// Convenience function to format for detail view
pub fn format_for_detail(result: &SearchResult) -> String {
    format_search_result(result, DisplayMode::Detail)
}

/// Format content for list view (single line)
fn format_list_view(parsed: &ParsedContent) -> String {
    if parsed.has_tools() {
        // Show first tool execution
        let tool = &parsed.tool_executions[0];
        let tool_str = format!("{} {}({})", TOOL_MARKER, tool.name, tool.arguments);
        
        // Add any text before tools
        if !parsed.text_before.is_empty() {
            let text = parsed.text_before.join(" ").replace('\n', " ");
            format!("{} | {}", tool_str, text)
        } else if !parsed.text_after.is_empty() {
            let text = parsed.text_after.join(" ").replace('\n', " ");
            format!("{} | {}", tool_str, text)
        } else {
            tool_str
        }
    } else if parsed.has_thinking() {
        // Show thinking marker
        format!("{} Thinking...", THINKING_MARKER)
    } else {
        // Regular message - join all text and replace newlines with spaces
        let mut all_text = Vec::new();
        all_text.extend(parsed.text_before.clone());
        all_text.extend(parsed.text_between.clone());
        all_text.extend(parsed.text_after.clone());
        all_text.join(" ").replace('\n', " ")
    }
}

/// Format content for preview view (multi-line with truncation)
fn format_preview_view(parsed: &ParsedContent, max_lines: usize) -> String {
    let mut lines = Vec::new();
    let mut line_count = 0;
    
    // Add text before tools
    for text in &parsed.text_before {
        for line in text.lines() {
            if line_count >= max_lines {
                let remaining = count_remaining_lines(parsed, line_count);
                if remaining > 0 {
                    lines.push(format!("{} +{} lines (press Enter to view full)", TRUNCATION_MARKER, remaining));
                }
                return lines.join("\n");
            }
            lines.push(line.to_string());
            line_count += 1;
        }
    }
    
    // Add thinking blocks
    for (i, thinking) in parsed.thinking_blocks.iter().enumerate() {
        if line_count >= max_lines {
            let remaining = count_remaining_lines(parsed, line_count);
            if remaining > 0 {
                lines.push(format!("{} +{} lines (press Enter to view full)", TRUNCATION_MARKER, remaining));
            }
            return lines.join("\n");
        }
        
        if i > 0 && line_count > 0 {
            lines.push(String::new());
            line_count += 1;
        }
        
        lines.push(format!("{} Thinking...", THINKING_MARKER));
        line_count += 1;
        
        for line in thinking.content.lines() {
            if line_count >= max_lines {
                let remaining = count_remaining_lines(parsed, line_count);
                if remaining > 0 {
                    lines.push(format!("{} +{} lines (press Enter to view full)", TRUNCATION_MARKER, remaining));
                }
                return lines.join("\n");
            }
            lines.push(line.to_string());
            line_count += 1;
        }
    }
    
    // Add tool executions
    for (i, tool) in parsed.tool_executions.iter().enumerate() {
        if line_count >= max_lines {
            let remaining = count_remaining_lines(parsed, line_count);
            if remaining > 0 {
                lines.push(format!("{} +{} lines (press Enter to view full)", TRUNCATION_MARKER, remaining));
            }
            return lines.join("\n");
        }
        
        // Add spacing between sections
        if i > 0 || !parsed.text_before.is_empty() || !parsed.thinking_blocks.is_empty() {
            if line_count > 0 {
                lines.push(String::new());
                line_count += 1;
            }
        }
        
        lines.push(format!("{} {}({})", TOOL_MARKER, tool.name, tool.arguments));
        line_count += 1;
        
        if let Some(result) = &tool.result {
            for line in result.lines() {
                if line_count >= max_lines {
                    let remaining = count_remaining_lines(parsed, line_count);
                    if remaining > 0 {
                        lines.push(format!("{} +{} lines (press Enter to view full)", TRUNCATION_MARKER, remaining));
                    }
                    return lines.join("\n");
                }
                lines.push(format!("{} {}", RESULT_MARKER, line));
                line_count += 1;
            }
        }
        
        // Add text between tools
        if i < parsed.text_between.len() {
            for line in parsed.text_between[i].lines() {
                if line_count >= max_lines {
                    let remaining = count_remaining_lines(parsed, line_count);
                    if remaining > 0 {
                        lines.push(format!("{} +{} lines (press Enter to view full)", TRUNCATION_MARKER, remaining));
                    }
                    return lines.join("\n");
                }
                lines.push(line.to_string());
                line_count += 1;
            }
        }
    }
    
    // Add text after tools
    for text in &parsed.text_after {
        if !text.is_empty() && line_count > 0 {
            lines.push(String::new());
            line_count += 1;
        }
        
        for line in text.lines() {
            if line_count >= max_lines {
                let remaining = count_remaining_lines(parsed, line_count);
                if remaining > 0 {
                    lines.push(format!("{} +{} lines (press Enter to view full)", TRUNCATION_MARKER, remaining));
                }
                return lines.join("\n");
            }
            lines.push(line.to_string());
            line_count += 1;
        }
    }
    
    lines.join("\n")
}

/// Format content for detail view (complete content)
fn format_detail_view(parsed: &ParsedContent) -> String {
    let mut output = Vec::new();
    
    // Add text before tools
    for text in &parsed.text_before {
        if !text.is_empty() {
            output.push(text.clone());
        }
    }
    
    // Add thinking blocks
    for thinking in &parsed.thinking_blocks {
        if !output.is_empty() {
            output.push(String::new());
        }
        output.push(format!("{} Thinking...", THINKING_MARKER));
        output.push(thinking.content.clone());
    }
    
    // Add tool executions
    for (i, tool) in parsed.tool_executions.iter().enumerate() {
        if !output.is_empty() {
            output.push(String::new());
        }
        
        output.push(format!("{} {}({})", TOOL_MARKER, tool.name, tool.arguments));
        
        if let Some(result) = &tool.result {
            for line in result.lines() {
                output.push(format!("{} {}", RESULT_MARKER, line));
            }
        }
        
        // Add text between tools
        if i < parsed.text_between.len() && !parsed.text_between[i].is_empty() {
            output.push(String::new());
            output.push(parsed.text_between[i].clone());
        }
    }
    
    // Add text after tools
    for text in &parsed.text_after {
        if !text.is_empty() {
            if !output.is_empty() {
                output.push(String::new());
            }
            output.push(text.clone());
        }
    }
    
    output.join("\n")
}

/// Count remaining lines in parsed content
fn count_remaining_lines(parsed: &ParsedContent, current_lines: usize) -> usize {
    let total_lines = count_total_lines(parsed);
    if total_lines > current_lines {
        total_lines - current_lines
    } else {
        0
    }
}

/// Count total lines in parsed content
fn count_total_lines(parsed: &ParsedContent) -> usize {
    let mut count = 0;
    
    // Count text before
    for text in &parsed.text_before {
        count += text.lines().count();
    }
    
    // Count thinking blocks
    for thinking in &parsed.thinking_blocks {
        count += 1; // Thinking marker
        count += thinking.content.lines().count();
    }
    
    // Count tool executions
    for tool in &parsed.tool_executions {
        count += 1; // Tool marker
        if let Some(result) = &tool.result {
            count += result.lines().count();
        }
    }
    
    // Count text between
    for text in &parsed.text_between {
        count += text.lines().count();
    }
    
    // Count text after
    for text in &parsed.text_after {
        count += text.lines().count();
    }
    
    count
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::condition::QueryCondition;

    fn create_test_result(text: &str, raw_json: Option<String>) -> SearchResult {
        SearchResult {
            file: "test.jsonl".to_string(),
            uuid: "test-uuid".to_string(),
            timestamp: "2024-01-01T12:00:00Z".to_string(),
            session_id: "test-session".to_string(),
            role: "assistant".to_string(),
            text: text.to_string(),
            message_type: "message".to_string(),
            query: QueryCondition::Literal {
                pattern: "test".to_string(),
                case_sensitive: false,
            },
            cwd: "/test".to_string(),
            raw_json,
        }
    }

    #[test]
    fn test_format_simple_message_list() {
        let result = create_test_result("Hello, how can I help you today?", None);
        let formatted = format_search_result(&result, DisplayMode::List);
        assert_eq!(formatted, "Hello, how can I help you today?");
    }

    #[test]
    fn test_format_multiline_message_list() {
        let result = create_test_result("Line 1\nLine 2\nLine 3", None);
        let formatted = format_search_result(&result, DisplayMode::List);
        assert_eq!(formatted, "Line 1 Line 2 Line 3");
    }

    #[test]
    fn test_format_tool_execution_list() {
        let raw_json = r#"{
            "type": "assistant",
            "message": {
                "content": [
                    {"type": "text", "text": "I'll read that file."},
                    {
                        "type": "tool_use",
                        "id": "tool_123",
                        "name": "Read",
                        "input": {"file_path": "/tmp/test.txt"}
                    },
                    {
                        "type": "tool_result",
                        "tool_use_id": "tool_123",
                        "content": "File contents"
                    }
                ]
            }
        }"#;
        
        let result = create_test_result("", Some(raw_json.to_string()));
        let formatted = format_search_result(&result, DisplayMode::List);
        assert_eq!(formatted, r#"⏺ Read({"file_path":"/tmp/test.txt"}) | I'll read that file."#);
    }

    #[test]
    fn test_format_thinking_list() {
        let raw_json = r#"{
            "type": "assistant",
            "message": {
                "content": [
                    {
                        "type": "thinking",
                        "thinking": "This is complex..."
                    },
                    {"type": "text", "text": "Here's the answer."}
                ]
            }
        }"#;
        
        let result = create_test_result("", Some(raw_json.to_string()));
        let formatted = format_search_result(&result, DisplayMode::List);
        assert_eq!(formatted, "✻ Thinking...");
    }

    #[test]
    fn test_format_preview_with_truncation() {
        let mut long_text = String::new();
        for i in 0..20 {
            long_text.push_str(&format!("Line {}\n", i + 1));
        }
        
        let result = create_test_result(&long_text, None);
        let formatted = format_search_result(&result, DisplayMode::Preview { max_lines: 10 });
        
        let lines: Vec<String> = formatted.lines().map(|s| s.to_string()).collect();
        assert_eq!(lines.len(), 11); // 10 lines + truncation marker
        assert!(lines[10].starts_with("… +"));
        assert!(lines[10].contains("lines (press Enter to view full)"));
    }

    #[test]
    fn test_format_detail_complete() {
        let raw_json = r#"{
            "type": "assistant",
            "message": {
                "content": [
                    {"type": "text", "text": "Before tool"},
                    {
                        "type": "tool_use",
                        "id": "tool_123",
                        "name": "WebSearch",
                        "input": {"query": "test"}
                    },
                    {
                        "type": "tool_result",
                        "tool_use_id": "tool_123",
                        "content": "Search results here"
                    },
                    {"type": "text", "text": "After tool"}
                ]
            }
        }"#;
        
        let result = create_test_result("", Some(raw_json.to_string()));
        let formatted = format_search_result(&result, DisplayMode::Detail);
        
        assert!(formatted.contains("Before tool"));
        assert!(formatted.contains(r#"⏺ WebSearch({"query":"test"})"#));
        assert!(formatted.contains("⎿ Search results here"));
        assert!(formatted.contains("After tool"));
    }
}