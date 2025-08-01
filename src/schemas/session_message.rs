use serde::{Deserialize, Serialize};
use serde_json::Value;

// Base message fields common to most message types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BaseMessage {
    pub parent_uuid: Option<String>,
    pub is_sidechain: bool,
    pub user_type: String,
    pub cwd: String,
    pub session_id: String,
    pub version: String,
    pub uuid: String,
    pub timestamp: String,
}

// Content types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Content {
    Text {
        text: String,
    },
    ToolUse {
        id: String,
        name: String,
        input: Value,
    },
    ToolResult {
        tool_use_id: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        content: Option<ToolResultContent>,
        #[serde(skip_serializing_if = "Option::is_none")]
        is_error: Option<bool>,
    },
    Thinking {
        thinking: String,
        signature: String,
    },
    Image {
        source: ImageSource,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ToolResultContent {
    String(String),
    TextArray(Vec<TextContent>),
    ImageArray(Vec<ImageContent>),
    // Handle other tool result types dynamically
    Value(Value),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextContent {
    #[serde(rename = "type")]
    pub content_type: String,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageContent {
    #[serde(rename = "type")]
    pub content_type: String,
    pub source: ImageSource,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageSource {
    #[serde(rename = "type")]
    pub source_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub media_type: Option<String>,
}

// Usage information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Usage {
    pub input_tokens: u32,
    pub cache_creation_input_tokens: u32,
    pub cache_read_input_tokens: u32,
    pub output_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_tier: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server_tool_use: Option<ServerToolUse>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerToolUse {
    pub web_search_requests: u32,
}

// Message content structures
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserMessageContent {
    pub role: String,
    pub content: UserContent,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum UserContent {
    String(String),
    Array(Vec<Content>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssistantMessageContent {
    pub id: String,
    #[serde(rename = "type")]
    pub message_type: String,
    pub role: String,
    pub model: String,
    pub content: Vec<Content>,
    pub stop_reason: Option<String>,
    pub stop_sequence: Option<String>,
    pub usage: Usage,
}

// Main message types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SessionMessage {
    Summary {
        summary: String,
        #[serde(rename = "leafUuid")]
        leaf_uuid: String,
    },
    System {
        #[serde(flatten)]
        base: BaseMessage,
        content: String,
        #[serde(rename = "isMeta")]
        is_meta: bool,
        #[serde(skip_serializing_if = "Option::is_none", rename = "toolUseID")]
        tool_use_id: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        level: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none", rename = "gitBranch")]
        git_branch: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none", rename = "requestId")]
        request_id: Option<String>,
    },
    User {
        #[serde(flatten)]
        base: BaseMessage,
        message: UserMessageContent,
        #[serde(skip_serializing_if = "Option::is_none", rename = "gitBranch")]
        git_branch: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none", rename = "isMeta")]
        is_meta: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none", rename = "isCompactSummary")]
        is_compact_summary: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none", rename = "toolUseResult")]
        tool_use_result: Option<Value>,
    },
    Assistant {
        #[serde(flatten)]
        base: BaseMessage,
        message: AssistantMessageContent,
        #[serde(skip_serializing_if = "Option::is_none", rename = "requestId")]
        request_id: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none", rename = "gitBranch")]
        git_branch: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none", rename = "isApiErrorMessage")]
        is_api_error_message: Option<bool>,
    },
}

// Helper methods
impl SessionMessage {
    pub fn get_type(&self) -> &'static str {
        match self {
            SessionMessage::Summary { .. } => "summary",
            SessionMessage::System { .. } => "system",
            SessionMessage::User { .. } => "user",
            SessionMessage::Assistant { .. } => "assistant",
        }
    }

    pub fn get_content_text(&self) -> String {
        match self {
            SessionMessage::Summary { summary, .. } => summary.clone(),
            SessionMessage::System { content, .. } => content.clone(),
            SessionMessage::User { message, .. } => {
                let mut texts = Vec::new();

                // Extract content text
                match &message.content {
                    UserContent::String(s) => texts.push(s.clone()),
                    UserContent::Array(contents) => {
                        for content in contents {
                            match content {
                                Content::Text { text } => texts.push(text.clone()),
                                Content::ToolResult {
                                    tool_use_id: _,
                                    content: Some(tool_content),
                                    is_error: _,
                                } => {
                                    let result_text = match tool_content {
                                        ToolResultContent::String(s) => {
                                            if !s.is_empty() {
                                                s.clone()
                                            } else {
                                                continue;
                                            }
                                        }
                                        ToolResultContent::TextArray(arr) => {
                                            if !arr.is_empty() {
                                                arr.iter()
                                                    .map(|item| &item.text)
                                                    .cloned()
                                                    .collect::<Vec<_>>()
                                                    .join("\n")
                                            } else {
                                                continue;
                                            }
                                        }
                                        ToolResultContent::Value(val) => {
                                            if let Some(s) = val.as_str() {
                                                s.to_string()
                                            } else {
                                                continue;
                                            }
                                        }
                                        _ => continue,
                                    };
                                    texts.push(result_text);
                                }
                                Content::ToolResult {
                                    tool_use_id: _,
                                    content: None,
                                    ..
                                } => {
                                    // Skip tool results with no content
                                    continue;
                                }
                                Content::ToolUse { name, input, .. } => {
                                    let mut tool_text = name.clone();

                                    // Extract key information from input based on tool type
                                    if let Some(obj) = input.as_object() {
                                        match name.as_str() {
                                            "Bash" => {
                                                if let Some(cmd) =
                                                    obj.get("command").and_then(|v| v.as_str())
                                                {
                                                    tool_text.push_str(": ");
                                                    tool_text.push_str(
                                                        &cmd.chars().take(50).collect::<String>(),
                                                    );
                                                    if cmd.len() > 50 {
                                                        tool_text.push_str("...");
                                                    }
                                                }
                                            }
                                            "Read" | "Write" | "Edit" => {
                                                if let Some(path) =
                                                    obj.get("file_path").and_then(|v| v.as_str())
                                                {
                                                    tool_text.push_str(": ");
                                                    tool_text.push_str(
                                                        path.split('/').next_back().unwrap_or(path),
                                                    );
                                                }
                                            }
                                            "Grep" => {
                                                if let Some(pattern) =
                                                    obj.get("pattern").and_then(|v| v.as_str())
                                                {
                                                    tool_text.push_str(": ");
                                                    tool_text.push_str(
                                                        &pattern
                                                            .chars()
                                                            .take(30)
                                                            .collect::<String>(),
                                                    );
                                                    if pattern.len() > 30 {
                                                        tool_text.push_str("...");
                                                    }
                                                }
                                            }
                                            _ => {
                                                // For other tools, try to find a descriptive field
                                                if let Some(desc) =
                                                    obj.get("description").and_then(|v| v.as_str())
                                                {
                                                    tool_text.push_str(": ");
                                                    tool_text.push_str(
                                                        &desc.chars().take(40).collect::<String>(),
                                                    );
                                                    if desc.len() > 40 {
                                                        tool_text.push_str("...");
                                                    }
                                                }
                                            }
                                        }
                                    }

                                    texts.push(tool_text);
                                }
                                Content::Image { .. } => {
                                    // Skip image entries
                                    continue;
                                }
                                Content::Thinking { thinking, .. } => texts.push(thinking.clone()),
                            }
                        }
                    }
                }

                texts.join("\n")
            }
            SessionMessage::Assistant { message, .. } => {
                let mut texts = Vec::new();

                for content in &message.content {
                    match content {
                        Content::Text { text } => texts.push(text.clone()),
                        Content::Thinking { thinking, .. } => texts.push(thinking.clone()),
                        Content::ToolUse { name, input, .. } => {
                            let mut tool_text = name.clone();

                            // Extract key information from input based on tool type
                            if let Some(obj) = input.as_object() {
                                match name.as_str() {
                                    "Bash" => {
                                        if let Some(cmd) =
                                            obj.get("command").and_then(|v| v.as_str())
                                        {
                                            tool_text.push_str(": ");
                                            tool_text.push_str(
                                                &cmd.chars().take(50).collect::<String>(),
                                            );
                                            if cmd.len() > 50 {
                                                tool_text.push_str("...");
                                            }
                                        }
                                    }
                                    "Read" | "Write" | "Edit" => {
                                        if let Some(path) =
                                            obj.get("file_path").and_then(|v| v.as_str())
                                        {
                                            tool_text.push_str(": ");
                                            tool_text.push_str(
                                                path.split('/').next_back().unwrap_or(path),
                                            );
                                        }
                                    }
                                    "Grep" => {
                                        if let Some(pattern) =
                                            obj.get("pattern").and_then(|v| v.as_str())
                                        {
                                            tool_text.push_str(": ");
                                            tool_text.push_str(
                                                &pattern.chars().take(30).collect::<String>(),
                                            );
                                            if pattern.len() > 30 {
                                                tool_text.push_str("...");
                                            }
                                        }
                                    }
                                    _ => {
                                        // For other tools, try to find a descriptive field
                                        if let Some(desc) =
                                            obj.get("description").and_then(|v| v.as_str())
                                        {
                                            tool_text.push_str(": ");
                                            tool_text.push_str(
                                                &desc.chars().take(40).collect::<String>(),
                                            );
                                            if desc.len() > 40 {
                                                tool_text.push_str("...");
                                            }
                                        }
                                    }
                                }
                            }

                            texts.push(tool_text);
                        }
                        Content::ToolResult {
                            tool_use_id: _,
                            content: Some(tool_content),
                            is_error: _,
                        } => {
                            let result_text = match tool_content {
                                ToolResultContent::String(s) => {
                                    if !s.is_empty() {
                                        s.clone()
                                    } else {
                                        continue;
                                    }
                                }
                                ToolResultContent::TextArray(arr) => {
                                    if !arr.is_empty() {
                                        arr.iter()
                                            .map(|item| &item.text)
                                            .cloned()
                                            .collect::<Vec<_>>()
                                            .join("\n")
                                    } else {
                                        continue;
                                    }
                                }
                                ToolResultContent::Value(val) => {
                                    if let Some(s) = val.as_str() {
                                        s.to_string()
                                    } else {
                                        continue;
                                    }
                                }
                                _ => continue,
                            };
                            texts.push(result_text);
                        }
                        Content::ToolResult {
                            tool_use_id: _,
                            content: None,
                            ..
                        } => {
                            // Skip tool results with no content
                            continue;
                        }
                        Content::Image { .. } => {
                            // Skip image entries
                            continue;
                        }
                    }
                }

                texts.join("\n")
            }
        }
    }

    pub fn get_uuid(&self) -> Option<&str> {
        match self {
            SessionMessage::Summary { leaf_uuid, .. } => Some(leaf_uuid),
            SessionMessage::System { base, .. } => Some(&base.uuid),
            SessionMessage::User { base, .. } => Some(&base.uuid),
            SessionMessage::Assistant { base, .. } => Some(&base.uuid),
        }
    }

    pub fn get_timestamp(&self) -> Option<&str> {
        match self {
            SessionMessage::Summary { .. } => None,
            SessionMessage::System { base, .. } => Some(&base.timestamp),
            SessionMessage::User { base, .. } => Some(&base.timestamp),
            SessionMessage::Assistant { base, .. } => Some(&base.timestamp),
        }
    }

    pub fn get_session_id(&self) -> Option<&str> {
        match self {
            SessionMessage::Summary { .. } => None,
            SessionMessage::System { base, .. } => Some(&base.session_id),
            SessionMessage::User { base, .. } => Some(&base.session_id),
            SessionMessage::Assistant { base, .. } => Some(&base.session_id),
        }
    }

    pub fn get_cwd(&self) -> Option<&str> {
        match self {
            SessionMessage::Summary { .. } => None,
            SessionMessage::System { base, .. } => Some(&base.cwd),
            SessionMessage::User { base, .. } => Some(&base.cwd),
            SessionMessage::Assistant { base, .. } => Some(&base.cwd),
        }
    }

    pub fn has_tool_use(&self) -> bool {
        match self {
            SessionMessage::Assistant { message, .. } => message
                .content
                .iter()
                .any(|c| matches!(c, Content::ToolUse { .. })),
            _ => false,
        }
    }

    pub fn has_thinking(&self) -> bool {
        match self {
            SessionMessage::Assistant { message, .. } => message
                .content
                .iter()
                .any(|c| matches!(c, Content::Thinking { .. })),
            _ => false,
        }
    }

    pub fn get_searchable_text(&self) -> String {
        let mut parts = vec![self.get_content_text()];

        // Add Session ID
        if let Some(session_id) = self.get_session_id() {
            parts.push(session_id.to_string());
        }

        // Add UUID
        if let Some(uuid) = self.get_uuid() {
            parts.push(uuid.to_string());
        }

        parts.join(" ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;

    #[test]
    fn test_parse_user_message() {
        let json = r#"{
            "type": "user",
            "message": {
                "role": "user",
                "content": "Hello, Claude!"
            },
            "uuid": "test-uuid",
            "timestamp": "2024-01-01T00:00:00Z",
            "sessionId": "test-session",
            "parentUuid": null,
            "isSidechain": false,
            "userType": "external",
            "cwd": "/test",
            "version": "1.0"
        }"#;

        let msg: SessionMessage = serde_json::from_str(json).unwrap();

        assert_eq!(msg.get_type(), "user");
        assert_eq!(msg.get_content_text(), "Hello, Claude!");
        assert_eq!(msg.get_uuid(), Some("test-uuid"));
        assert_eq!(msg.get_timestamp(), Some("2024-01-01T00:00:00Z"));
        assert_eq!(msg.get_session_id(), Some("test-session"));
        assert!(!msg.has_tool_use());
        assert!(!msg.has_thinking());
    }

    #[test]
    fn test_parse_assistant_message_with_tool_use() {
        let json = r#"{
            "type": "assistant",
            "message": {
                "id": "msg_01",
                "type": "message",
                "role": "assistant",
                "model": "claude-3-5-sonnet",
                "content": [
                    {"type": "text", "text": "I'll help you with that."},
                    {
                        "type": "tool_use",
                        "id": "tool_1",
                        "name": "read_file",
                        "input": {"path": "test.txt"}
                    }
                ],
                "stop_reason": "tool_use",
                "stop_sequence": null,
                "usage": {
                    "input_tokens": 100,
                    "cache_creation_input_tokens": 0,
                    "cache_read_input_tokens": 0,
                    "output_tokens": 50
                }
            },
            "uuid": "assistant-uuid",
            "timestamp": "2024-01-01T00:00:01Z",
            "sessionId": "test-session",
            "parentUuid": "test-uuid",
            "isSidechain": false,
            "userType": "external",
            "cwd": "/test",
            "version": "1.0"
        }"#;

        let msg: SessionMessage = serde_json::from_str(json).unwrap();

        assert_eq!(msg.get_type(), "assistant");
        assert_eq!(
            msg.get_content_text(),
            "I'll help you with that.\nread_file"
        );
        assert!(msg.has_tool_use());
        assert!(!msg.has_thinking());
    }

    #[test]
    fn test_parse_assistant_message_with_thinking() {
        let json = r#"{
            "type": "assistant",
            "message": {
                "id": "msg_02",
                "type": "message",
                "role": "assistant",
                "model": "claude-3-5-sonnet",
                "content": [
                    {
                        "type": "thinking",
                        "thinking": "Let me think about this problem...",
                        "signature": "signature"
                    },
                    {"type": "text", "text": "Here's my answer."}
                ],
                "stop_reason": "end_turn",
                "stop_sequence": null,
                "usage": {
                    "input_tokens": 100,
                    "cache_creation_input_tokens": 0,
                    "cache_read_input_tokens": 0,
                    "output_tokens": 50
                }
            },
            "uuid": "assistant-uuid-2",
            "timestamp": "2024-01-01T00:00:02Z",
            "sessionId": "test-session",
            "parentUuid": "test-uuid",
            "isSidechain": false,
            "userType": "external",
            "cwd": "/test",
            "version": "1.0"
        }"#;

        let msg: SessionMessage = serde_json::from_str(json).unwrap();

        assert_eq!(msg.get_type(), "assistant");
        assert_eq!(
            msg.get_content_text(),
            "Let me think about this problem...\nHere's my answer."
        );
        assert!(!msg.has_tool_use());
        assert!(msg.has_thinking());
    }

    #[test]
    fn test_parse_system_message() {
        let json = r#"{
            "type": "system",
            "content": "System notification: Task completed",
            "uuid": "system-uuid",
            "timestamp": "2024-01-01T00:00:03Z",
            "sessionId": "test-session",
            "parentUuid": null,
            "isSidechain": false,
            "userType": "external",
            "cwd": "/test",
            "version": "1.0",
            "isMeta": false
        }"#;

        let msg: SessionMessage = serde_json::from_str(json).unwrap();

        assert_eq!(msg.get_type(), "system");
        assert_eq!(
            msg.get_content_text(),
            "System notification: Task completed"
        );
        assert_eq!(msg.get_uuid(), Some("system-uuid"));
    }

    #[test]
    fn test_parse_summary_message() {
        let json = r#"{
            "type": "summary",
            "summary": "User asked about Rust programming",
            "leafUuid": "leaf-uuid-123"
        }"#;

        let msg: SessionMessage = serde_json::from_str(json).unwrap();

        assert_eq!(msg.get_type(), "summary");
        assert_eq!(msg.get_content_text(), "User asked about Rust programming");
        assert_eq!(msg.get_uuid(), Some("leaf-uuid-123"));
        assert_eq!(msg.get_timestamp(), None);
        assert_eq!(msg.get_session_id(), None);
    }

    #[test]
    fn test_user_message_with_array_content() {
        let json = r#"{
            "type": "user",
            "message": {
                "role": "user",
                "content": [
                    {"type": "text", "text": "Here's the result:"},
                    {
                        "type": "tool_result",
                        "tool_use_id": "tool_1",
                        "content": "File contents: Hello World"
                    }
                ]
            },
            "uuid": "user-uuid-2",
            "timestamp": "2024-01-01T00:00:04Z",
            "sessionId": "test-session",
            "parentUuid": null,
            "isSidechain": false,
            "userType": "external",
            "cwd": "/test",
            "version": "1.0"
        }"#;

        let msg: SessionMessage = serde_json::from_str(json).unwrap();

        assert_eq!(msg.get_type(), "user");
        assert_eq!(
            msg.get_content_text(),
            "Here's the result:\nFile contents: Hello World"
        );
    }

    #[test]
    fn test_user_message_with_tool_result_array() {
        let json = r#"{
            "type": "user",
            "message": {
                "role": "user",
                "content": [
                    {
                        "type": "tool_result",
                        "tool_use_id": "tool_2",
                        "content": [
                            {"type": "text", "text": "Line 1"},
                            {"type": "text", "text": "Line 2"}
                        ]
                    }
                ]
            },
            "uuid": "user-uuid-3",
            "timestamp": "2024-01-01T00:00:05Z",
            "sessionId": "test-session",
            "parentUuid": null,
            "isSidechain": false,
            "userType": "external",
            "cwd": "/test",
            "version": "1.0"
        }"#;

        let msg: SessionMessage = serde_json::from_str(json).unwrap();

        assert_eq!(msg.get_type(), "user");
        assert_eq!(msg.get_content_text(), "Line 1\nLine 2");
    }

    #[test]
    fn test_assistant_message_with_multiple_content_types() {
        let json = r#"{
            "type": "assistant",
            "message": {
                "id": "msg_03",
                "type": "message",
                "role": "assistant",
                "model": "claude-3-5-sonnet",
                "content": [
                    {"type": "text", "text": "Starting analysis..."},
                    {
                        "type": "tool_use",
                        "id": "tool_3",
                        "name": "analyze_code",
                        "input": {"file": "main.rs"}
                    },
                    {"type": "text", "text": "Analysis complete."}
                ],
                "stop_reason": "end_turn",
                "stop_sequence": null,
                "usage": {
                    "input_tokens": 100,
                    "cache_creation_input_tokens": 0,
                    "cache_read_input_tokens": 0,
                    "output_tokens": 50
                }
            },
            "uuid": "assistant-uuid-3",
            "timestamp": "2024-01-01T00:00:06Z",
            "sessionId": "test-session",
            "parentUuid": "user-uuid-3",
            "isSidechain": false,
            "userType": "external",
            "cwd": "/test",
            "version": "1.0"
        }"#;

        let msg: SessionMessage = serde_json::from_str(json).unwrap();

        assert_eq!(msg.get_type(), "assistant");
        assert_eq!(
            msg.get_content_text(),
            "Starting analysis...\nanalyze_code\nAnalysis complete."
        );
        assert!(msg.has_tool_use());
    }

    #[test]
    fn test_system_message_with_tool_use_id() {
        let json = r#"{
            "type": "system",
            "content": "Tool execution result",
            "uuid": "system-uuid-2",
            "timestamp": "2024-01-01T00:00:07Z",
            "sessionId": "test-session",
            "parentUuid": null,
            "isSidechain": false,
            "userType": "external",
            "cwd": "/test",
            "version": "1.0",
            "isMeta": true,
            "toolUseID": "tool_3"
        }"#;

        let msg: SessionMessage = serde_json::from_str(json).unwrap();

        if let SessionMessage::System {
            tool_use_id,
            is_meta,
            ..
        } = &msg
        {
            assert_eq!(tool_use_id.as_deref(), Some("tool_3"));
            assert!(is_meta);
        } else {
            panic!("Expected System message");
        }
    }

    #[test]
    fn test_message_with_git_branch() {
        let json = r#"{
            "type": "user",
            "message": {
                "role": "user",
                "content": "Check current branch"
            },
            "uuid": "user-uuid-4",
            "timestamp": "2024-01-01T00:00:08Z",
            "sessionId": "test-session",
            "parentUuid": null,
            "isSidechain": false,
            "userType": "external",
            "cwd": "/test",
            "version": "1.0",
            "gitBranch": "feature/test-branch"
        }"#;

        let msg: SessionMessage = serde_json::from_str(json).unwrap();

        if let SessionMessage::User { git_branch, .. } = &msg {
            assert_eq!(git_branch.as_deref(), Some("feature/test-branch"));
        } else {
            panic!("Expected User message");
        }
    }

    #[test]
    fn test_get_searchable_text() {
        // Test user message with session ID and UUID
        let json = r#"{
            "type": "user",
            "message": {
                "role": "user",
                "content": "Hello world"
            },
            "uuid": "user-uuid-123",
            "timestamp": "2024-01-01T00:00:00Z",
            "sessionId": "session-abc-456",
            "parentUuid": null,
            "isSidechain": false,
            "userType": "external",
            "cwd": "/test",
            "version": "1.0"
        }"#;

        let msg: SessionMessage = serde_json::from_str(json).unwrap();
        let searchable_text = msg.get_searchable_text();

        assert!(searchable_text.contains("Hello world"));
        assert!(searchable_text.contains("session-abc-456"));
        assert!(searchable_text.contains("user-uuid-123"));
    }

    #[test]
    fn test_get_searchable_text_summary() {
        // Test summary message (no session ID, but has UUID)
        let json = r#"{
            "type": "summary",
            "summary": "User asked about Rust",
            "leafUuid": "leaf-uuid-789"
        }"#;

        let msg: SessionMessage = serde_json::from_str(json).unwrap();
        let searchable_text = msg.get_searchable_text();

        assert!(searchable_text.contains("User asked about Rust"));
        assert!(searchable_text.contains("leaf-uuid-789"));
        assert!(!searchable_text.contains("session")); // No session ID for summary
    }
}
