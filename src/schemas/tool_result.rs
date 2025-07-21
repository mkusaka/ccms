use serde::{Deserialize, Serialize};
use serde_json::Value;

// Common types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TodoItem {
    pub content: String,
    pub status: String, // "pending", "in_progress", "completed"
    pub priority: String, // "high", "medium", "low"
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StructuredPatchItem {
    pub old_start: u32,
    pub old_lines: u32,
    pub new_start: u32,
    pub new_lines: u32,
    pub lines: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditItem {
    pub old_string: String,
    pub new_string: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub replace_all: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileInfo {
    pub file_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base64: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "type")]
    pub file_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub num_lines: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_lines: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_line: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub original_size: Option<u64>,
}

// Tool result types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ToolResult {
    // Bash Result (26.85%)
    Bash {
        stdout: String,
        stderr: String,
        interrupted: bool,
        #[serde(rename = "isImage")]
        is_image: bool,
        #[serde(skip_serializing_if = "Option::is_none", rename = "returnCodeInterpretation")]
        return_code_interpretation: Option<String>,
    },
    
    // File Read Result (23.26%)
    FileRead {
        #[serde(rename = "type")]
        result_type: String, // "text"
        file: FileInfo,
    },
    
    // Edit File Result (15.97%)
    EditFile {
        #[serde(rename = "filePath")]
        file_path: String,
        #[serde(rename = "oldString")]
        old_string: String,
        #[serde(rename = "newString")]
        new_string: String,
        #[serde(rename = "originalFile")]
        original_file: String,
        #[serde(rename = "structuredPatch")]
        structured_patch: Vec<StructuredPatchItem>,
        #[serde(rename = "userModified")]
        user_modified: bool,
        #[serde(rename = "replaceAll")]
        replace_all: bool,
    },
    
    // Todo Update Result (12.36%)
    TodoUpdate {
        #[serde(rename = "oldTodos")]
        old_todos: Vec<TodoItem>,
        #[serde(rename = "newTodos")]
        new_todos: Vec<TodoItem>,
    },
    
    // Create File Result (5.21%)
    CreateFile {
        #[serde(rename = "type")]
        result_type: String, // "create"
        #[serde(rename = "filePath")]
        file_path: String,
        content: String,
        #[serde(rename = "structuredPatch")]
        structured_patch: Vec<StructuredPatchItem>,
    },
    
    // Multi-edit Result (3.33%)
    MultiEdit {
        #[serde(rename = "filePath")]
        file_path: String,
        edits: Vec<EditItem>,
        #[serde(rename = "originalFileContents")]
        original_file_contents: String,
        #[serde(rename = "structuredPatch")]
        structured_patch: Vec<StructuredPatchItem>,
        #[serde(rename = "userModified")]
        user_modified: bool,
    },
    
    // Glob Result (3.32%)
    Glob {
        filenames: Vec<String>,
        #[serde(rename = "durationMs")]
        duration_ms: u64,
        #[serde(rename = "numFiles")]
        num_files: u32,
        truncated: bool,
    },
    
    // Grep Result (3.26%)
    Grep {
        mode: String, // "content", "files_with_matches", "count"
        filenames: Vec<String>,
        #[serde(rename = "numFiles")]
        num_files: u32,
        #[serde(skip_serializing_if = "Option::is_none")]
        content: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none", rename = "numLines")]
        num_lines: Option<u32>,
        #[serde(skip_serializing_if = "Option::is_none", rename = "numMatches")]
        num_matches: Option<u32>,
    },
    
    // Task Completion Result (1.49%)
    TaskResult {
        content: Vec<TextContent>,
        #[serde(rename = "totalDurationMs")]
        total_duration_ms: u64,
        #[serde(rename = "totalTokens")]
        total_tokens: u32,
        #[serde(rename = "totalToolUseCount")]
        total_tool_use_count: u32,
        usage: TaskUsage,
        #[serde(rename = "wasInterrupted")]
        was_interrupted: bool,
    },
    
    // WebSearch Result (0.71%)
    WebSearch {
        query: String,
        results: Vec<WebSearchResultItem>,
        #[serde(rename = "durationSeconds")]
        duration_seconds: f64,
    },
    
    // WebFetch Result (0.42%)
    WebFetch {
        bytes: u64,
        code: u32,
        #[serde(rename = "codeText")]
        code_text: String,
        result: String,
        #[serde(rename = "durationMs")]
        duration_ms: u64,
        url: String,
    },
    
    // Simple filenames pattern
    SimpleFilenames {
        filenames: Vec<String>,
        #[serde(rename = "numFiles")]
        num_files: u32,
    },
    
    // Update File Result
    UpdateFile {
        #[serde(rename = "type")]
        result_type: String, // "update"
        #[serde(rename = "filePath")]
        file_path: String,
        content: String,
        #[serde(rename = "structuredPatch")]
        structured_patch: Vec<StructuredPatchItem>,
    },
    
    // Image results
    ImageResult {
        #[serde(rename = "type")]
        result_type: String, // "image"
        #[serde(skip_serializing_if = "Option::is_none")]
        file: Option<ImageFileInfo>,
    },
    
    // Fallback for any other tool result
    Other(Value),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextContent {
    #[serde(rename = "type")]
    pub content_type: String,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskUsage {
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum WebSearchResultItem {
    String(String),
    Structured {
        tool_use_id: String,
        content: Vec<WebSearchContent>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSearchContent {
    pub title: String,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageFileInfo {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base64: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "type")]
    pub file_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "originalSize")]
    pub original_size: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<ImageSource>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageSource {
    #[serde(rename = "type")]
    pub source_type: String,
    pub data: String,
    pub media_type: String,
}