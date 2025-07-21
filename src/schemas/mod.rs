pub mod session_message;
pub mod tool_result;

// Re-export specific types to avoid conflicts
pub use session_message::{
    AssistantMessageContent,
    BaseMessage,
    Content,
    ImageContent,
    // Helper functions are not exported from session_message module
    // They are implemented as methods on SessionMessage
    SessionMessage,
    ToolResultContent,
    Usage,
    UserContent,
    UserMessageContent,
};

pub use tool_result::{
    EditItem, FileInfo, ImageFileInfo, ImageSource, ServerToolUse, StructuredPatchItem, TaskUsage,
    TextContent, TodoItem, ToolResult, WebSearchContent, WebSearchResultItem,
};
