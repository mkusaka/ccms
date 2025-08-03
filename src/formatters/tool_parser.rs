use serde_json::Value;

/// Represents a tool execution found in a message
#[derive(Debug, Clone, PartialEq)]
pub struct ToolExecution {
    /// The name of the tool (e.g., "Read", "Write", "Bash")
    pub name: String,
    /// The arguments passed to the tool as a JSON string
    pub arguments: String,
    /// The result of the tool execution, if available
    pub result: Option<String>,
}

/// Represents a thinking block in a message
#[derive(Debug, Clone, PartialEq)]
pub struct ThinkingBlock {
    /// The content of the thinking block
    pub content: String,
}

/// Parsed content from a message
#[derive(Debug, Clone, PartialEq)]
pub struct ParsedContent {
    /// Regular text content before any tool executions
    pub text_before: Vec<String>,
    /// Tool executions found in the message
    pub tool_executions: Vec<ToolExecution>,
    /// Thinking blocks found in the message  
    pub thinking_blocks: Vec<ThinkingBlock>,
    /// Text content between tool executions
    pub text_between: Vec<String>,
    /// Text content after all tool executions
    pub text_after: Vec<String>,
}

impl ParsedContent {
    /// Create a new empty ParsedContent
    pub fn new() -> Self {
        Self {
            text_before: Vec::new(),
            tool_executions: Vec::new(),
            thinking_blocks: Vec::new(),
            text_between: Vec::new(),
            text_after: Vec::new(),
        }
    }

    /// Check if the content has any tool executions
    pub fn has_tools(&self) -> bool {
        !self.tool_executions.is_empty()
    }

    /// Check if the content has any thinking blocks
    pub fn has_thinking(&self) -> bool {
        !self.thinking_blocks.is_empty()
    }
}

/// Parse a raw JSON message to extract tool executions and thinking blocks
pub fn parse_raw_json(raw_json: &str) -> ParsedContent {
    let mut parsed = ParsedContent::new();
    
    // Try to parse the JSON
    let json_value: Value = match serde_json::from_str(raw_json) {
        Ok(value) => value,
        Err(_) => return parsed, // Return empty if not valid JSON
    };
    
    // Extract message content
    let content_array = match json_value.get("message").and_then(|m| m.get("content")).and_then(|c| c.as_array()) {
        Some(arr) => arr,
        None => {
            // Try to get content as string for system messages
            if let Some(content) = json_value.get("content").and_then(|c| c.as_str()) {
                parsed.text_before.push(content.to_string());
            } else if let Some(content) = json_value.get("message").and_then(|m| m.get("content")).and_then(|c| c.as_str()) {
                parsed.text_before.push(content.to_string());
            }
            return parsed;
        }
    };
    
    let mut current_text = Vec::new();
    let mut after_tools = false;
    
    for item in content_array {
        if let Some(item_type) = item.get("type").and_then(|t| t.as_str()) {
            match item_type {
                "text" => {
                    if let Some(text) = item.get("text").and_then(|t| t.as_str()) {
                        current_text.push(text.to_string());
                    }
                }
                "thinking" => {
                    // Save any accumulated text
                    if !current_text.is_empty() {
                        if parsed.tool_executions.is_empty() && parsed.thinking_blocks.is_empty() {
                            parsed.text_before.extend(current_text.clone());
                        } else if after_tools {
                            parsed.text_after.extend(current_text.clone());
                        } else {
                            parsed.text_between.extend(current_text.clone());
                        }
                        current_text.clear();
                    }
                    
                    if let Some(thinking) = item.get("thinking").and_then(|t| t.as_str()) {
                        parsed.thinking_blocks.push(ThinkingBlock {
                            content: thinking.to_string(),
                        });
                    }
                }
                "tool_use" => {
                    // Save any accumulated text
                    if !current_text.is_empty() {
                        if parsed.tool_executions.is_empty() && parsed.thinking_blocks.is_empty() {
                            parsed.text_before.extend(current_text.clone());
                        } else {
                            parsed.text_between.extend(current_text.clone());
                        }
                        current_text.clear();
                    }
                    
                    after_tools = true;
                    
                    let name = item.get("name").and_then(|n| n.as_str()).unwrap_or("Tool").to_string();
                    let arguments = if let Some(input) = item.get("input") {
                        serde_json::to_string(input).unwrap_or_else(|_| "{}".to_string())
                    } else {
                        "{}".to_string()
                    };
                    
                    // Look for corresponding tool result
                    let result = find_tool_result(&json_value, item.get("id").and_then(|id| id.as_str()));
                    
                    parsed.tool_executions.push(ToolExecution {
                        name,
                        arguments,
                        result,
                    });
                }
                _ => {}
            }
        }
    }
    
    // Save any remaining text
    if !current_text.is_empty() {
        if parsed.tool_executions.is_empty() && parsed.thinking_blocks.is_empty() {
            parsed.text_before.extend(current_text);
        } else {
            parsed.text_after.extend(current_text);
        }
    }
    
    parsed
}

/// Find tool result for a given tool ID
fn find_tool_result(json_value: &Value, tool_id: Option<&str>) -> Option<String> {
    let tool_id = tool_id?;
    
    // Look for tool_result in the message
    if let Some(content_array) = json_value.get("message").and_then(|m| m.get("content")).and_then(|c| c.as_array()) {
        for item in content_array {
            if let Some(item_type) = item.get("type").and_then(|t| t.as_str()) {
                if item_type == "tool_result" {
                    if let Some(id) = item.get("tool_use_id").and_then(|id| id.as_str()) {
                        if id == tool_id {
                            if let Some(content) = item.get("content").and_then(|c| c.as_str()) {
                                return Some(content.to_string());
                            } else if let Some(content_array) = item.get("content").and_then(|c| c.as_array()) {
                                // Handle content as array of text blocks
                                let texts: Vec<String> = content_array.iter()
                                    .filter_map(|item| {
                                        if item.get("type").and_then(|t| t.as_str()) == Some("text") {
                                            item.get("text").and_then(|t| t.as_str()).map(|s| s.to_string())
                                        } else {
                                            None
                                        }
                                    })
                                    .collect();
                                return Some(texts.join("\n"));
                            }
                        }
                    }
                }
            }
        }
    }
    
    None
}

/// Parse text content to extract tool executions (fallback parser)
pub fn parse_text_content(text: &str) -> ParsedContent {
    let mut parsed = ParsedContent::new();
    
    // Simple regex-based parsing for tool executions
    // Looking for patterns like:
    // ⏺ ToolName(arguments)
    // ⎿ result content
    
    let lines: Vec<&str> = text.lines().collect();
    let mut i = 0;
    let mut current_text = Vec::new();
    let mut after_tools = false;
    
    while i < lines.len() {
        let line = lines[i];
        
        // Check for tool execution marker
        if line.starts_with("⏺ ") {
            // Save any accumulated text
            if !current_text.is_empty() {
                let text = current_text.join("\n");
                if parsed.tool_executions.is_empty() && parsed.thinking_blocks.is_empty() {
                    parsed.text_before.push(text);
                } else {
                    parsed.text_between.push(text);
                }
                current_text.clear();
            }
            
            after_tools = true;
            
            // Parse tool name and arguments
            let tool_line = line.strip_prefix("⏺ ").unwrap().trim();
            if let Some(paren_pos) = tool_line.find('(') {
                let name = tool_line[..paren_pos].to_string();
                let args_end = tool_line.rfind(')').unwrap_or(tool_line.len());
                let arguments = tool_line[paren_pos+1..args_end].to_string();
                
                // Look for result on next lines starting with ⎿
                let mut result_lines = Vec::new();
                i += 1;
                while i < lines.len() && lines[i].starts_with("⎿ ") {
                    result_lines.push(lines[i].strip_prefix("⎿ ").unwrap()); // Skip "⎿ "
                    i += 1;
                }
                
                let result = if !result_lines.is_empty() {
                    Some(result_lines.join("\n"))
                } else {
                    None
                };
                
                parsed.tool_executions.push(ToolExecution {
                    name,
                    arguments,
                    result,
                });
                
                continue; // Skip incrementing i since we already did it
            }
        }
        // Check for thinking block marker
        else if line.starts_with("✻ ") {
            // Save any accumulated text
            if !current_text.is_empty() {
                let text = current_text.join("\n");
                if parsed.tool_executions.is_empty() && parsed.thinking_blocks.is_empty() {
                    parsed.text_before.push(text);
                } else if after_tools {
                    parsed.text_after.push(text);
                } else {
                    parsed.text_between.push(text);
                }
                current_text.clear();
            }
            
            // Collect thinking content
            let mut thinking_lines = Vec::new();
            i += 1;
            while i < lines.len() && !lines[i].starts_with("⏺ ") && !lines[i].starts_with("✻ ") {
                // Stop at empty lines to separate thinking from subsequent content
                if lines[i].trim().is_empty() {
                    i += 1;
                    break;
                }
                thinking_lines.push(lines[i]);
                i += 1;
            }
            
            parsed.thinking_blocks.push(ThinkingBlock {
                content: thinking_lines.join("\n"),
            });
            
            continue; // Skip incrementing i since we already did it
        }
        else {
            current_text.push(line);
        }
        
        i += 1;
    }
    
    // Save any remaining text
    if !current_text.is_empty() {
        let text = current_text.join("\n");
        if parsed.tool_executions.is_empty() && parsed.thinking_blocks.is_empty() {
            parsed.text_before.push(text);
        } else {
            parsed.text_after.push(text);
        }
    }
    
    parsed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_text_message() {
        let raw_json = r#"{
            "type": "assistant",
            "message": {
                "content": [
                    {"type": "text", "text": "Hello, how can I help you?"}
                ]
            }
        }"#;
        
        let parsed = parse_raw_json(raw_json);
        assert_eq!(parsed.text_before, vec!["Hello, how can I help you?"]);
        assert!(parsed.tool_executions.is_empty());
        assert!(!parsed.has_tools());
    }

    #[test] 
    fn test_parse_tool_execution() {
        let raw_json = r#"{
            "type": "assistant",
            "message": {
                "content": [
                    {"type": "text", "text": "I'll read that file for you."},
                    {
                        "type": "tool_use",
                        "id": "tool_123",
                        "name": "Read",
                        "input": {"file_path": "/tmp/test.txt"}
                    },
                    {
                        "type": "tool_result",
                        "tool_use_id": "tool_123",
                        "content": "File contents here"
                    },
                    {"type": "text", "text": "The file contains the test data."}
                ]
            }
        }"#;
        
        let parsed = parse_raw_json(raw_json);
        assert_eq!(parsed.text_before, vec!["I'll read that file for you."]);
        assert_eq!(parsed.tool_executions.len(), 1);
        assert_eq!(parsed.tool_executions[0].name, "Read");
        assert_eq!(parsed.tool_executions[0].arguments, r#"{"file_path":"/tmp/test.txt"}"#);
        assert_eq!(parsed.tool_executions[0].result, Some("File contents here".to_string()));
        assert_eq!(parsed.text_after, vec!["The file contains the test data."]);
        assert!(parsed.has_tools());
    }

    #[test]
    fn test_parse_multiple_tools() {
        let raw_json = r#"{
            "type": "assistant",
            "message": {
                "content": [
                    {"type": "text", "text": "Let me search and then read a file."},
                    {
                        "type": "tool_use",
                        "id": "tool_1",
                        "name": "WebSearch",
                        "input": {"query": "rust async"}
                    },
                    {
                        "type": "tool_result",
                        "tool_use_id": "tool_1",
                        "content": "Search results..."
                    },
                    {"type": "text", "text": "Now reading the documentation."},
                    {
                        "type": "tool_use",
                        "id": "tool_2",
                        "name": "Read",
                        "input": {"file_path": "/docs/async.md"}
                    },
                    {
                        "type": "tool_result",
                        "tool_use_id": "tool_2",
                        "content": "Documentation content..."
                    },
                    {"type": "text", "text": "Based on the search and documentation..."}
                ]
            }
        }"#;
        
        let parsed = parse_raw_json(raw_json);
        assert_eq!(parsed.text_before, vec!["Let me search and then read a file."]);
        assert_eq!(parsed.tool_executions.len(), 2);
        assert_eq!(parsed.tool_executions[0].name, "WebSearch");
        assert_eq!(parsed.tool_executions[1].name, "Read");
        assert_eq!(parsed.text_between, vec!["Now reading the documentation."]);
        assert_eq!(parsed.text_after, vec!["Based on the search and documentation..."]);
    }

    #[test]
    fn test_parse_thinking_block() {
        let raw_json = r#"{
            "type": "assistant",
            "message": {
                "content": [
                    {"type": "text", "text": "Let me think about this."},
                    {
                        "type": "thinking",
                        "thinking": "This is a complex problem that requires careful analysis..."
                    },
                    {"type": "text", "text": "I have a solution."}
                ]
            }
        }"#;
        
        let parsed = parse_raw_json(raw_json);
        assert_eq!(parsed.text_before, vec!["Let me think about this."]);
        assert_eq!(parsed.thinking_blocks.len(), 1);
        assert_eq!(parsed.thinking_blocks[0].content, "This is a complex problem that requires careful analysis...");
        assert_eq!(parsed.text_after, vec!["I have a solution."]);
        assert!(parsed.has_thinking());
    }

    #[test]
    fn test_parse_text_with_tool_markers() {
        let text = r#"I'll help you with that.

⏺ Read(file_path="/tmp/test.txt")
⎿ This is the file content
⎿ It has multiple lines

The file has been read successfully."#;
        
        let parsed = parse_text_content(text);
        assert_eq!(parsed.text_before, vec!["I'll help you with that.\n"]);
        assert_eq!(parsed.tool_executions.len(), 1);
        assert_eq!(parsed.tool_executions[0].name, "Read");
        assert_eq!(parsed.tool_executions[0].arguments, r#"file_path="/tmp/test.txt""#);
        assert_eq!(parsed.tool_executions[0].result, Some("This is the file content\nIt has multiple lines".to_string()));
        assert_eq!(parsed.text_after, vec!["\nThe file has been read successfully."]);
    }

    #[test]
    fn test_parse_text_with_thinking_marker() {
        let text = r#"Let me consider this problem.

✻ Thinking...
This requires a multi-step approach:
1. First analyze the requirements
2. Then implement the solution

Based on my analysis, here's the solution."#;
        
        let parsed = parse_text_content(text);
        assert_eq!(parsed.text_before, vec!["Let me consider this problem.\n"]);
        assert_eq!(parsed.thinking_blocks.len(), 1);
        assert_eq!(parsed.thinking_blocks[0].content, "This requires a multi-step approach:\n1. First analyze the requirements\n2. Then implement the solution");
        assert_eq!(parsed.text_after, vec!["Based on my analysis, here's the solution."]);
    }

    #[test]
    fn test_system_message() {
        let raw_json = r#"{
            "type": "system",
            "content": "You are a helpful assistant."
        }"#;
        
        let parsed = parse_raw_json(raw_json);
        assert_eq!(parsed.text_before, vec!["You are a helpful assistant."]);
        assert!(parsed.tool_executions.is_empty());
        assert!(parsed.thinking_blocks.is_empty());
    }
}