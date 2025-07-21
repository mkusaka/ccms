pub mod session_message;
pub mod tool_result;

// Re-export specific types to avoid conflicts
pub use session_message::{
    SessionMessage, BaseMessage, Content, UserMessageContent, UserContent,
    AssistantMessageContent, Usage, ToolResultContent, ImageContent,
    // Helper functions are not exported from session_message module
    // They are implemented as methods on SessionMessage
};

pub use tool_result::{
    ToolResult, TodoItem, StructuredPatchItem, EditItem, FileInfo,
    TextContent, TaskUsage, ServerToolUse, WebSearchResultItem,
    WebSearchContent, ImageFileInfo, ImageSource
};