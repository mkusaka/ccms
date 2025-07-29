use crate::query::condition::SearchResult;

#[derive(Clone, Debug, PartialEq)]
pub enum CopyContent {
    FilePath(String),
    ProjectPath(String),
    SessionId(String),
    MessageContent(String),
    JsonData(String),
    FullMessageDetails(String),
}

#[derive(Clone, Debug)]
pub enum Message {
    // Search events
    QueryChanged(String),
    SearchRequested,
    SearchCompleted(Vec<SearchResult>),
    SelectResult(usize),
    ScrollUp,
    ScrollDown,
    ToggleSearchOrder,

    // Mode changes
    EnterMessageDetail,
    EnterSessionViewer,
    EnterMessageDetailFromSession(String, String, Option<String>), // (raw_json, file_path, session_id)
    ExitToSearch,
    ShowHelp,
    CloseHelp,

    // Navigation history
    NavigateBack,
    NavigateForward,

    // Session events
    LoadSession(String),
    SessionQueryChanged(String),
    SessionScrollUp,
    SessionScrollDown,
    SessionSelectUp,
    SessionSelectDown,
    SessionNavigated(usize, usize), // (selected_index, scroll_offset)
    ToggleSessionOrder,
    ToggleSessionRoleFilter,

    // Role filter
    ToggleRoleFilter,

    // Display options
    ToggleTruncation,

    // Clipboard
    CopyToClipboard(CopyContent),

    // Async events
    SearchStarted(u64),
    SearchProgress(u64, String),

    // UI events
    SetStatus(String),
    ClearStatus,

    // Terminal events
    Quit,
    Refresh,
}
