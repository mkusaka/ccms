use crate::query::condition::SearchResult;

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub enum Message {
    // Search events
    QueryChanged(String),
    SearchRequested,
    SearchCompleted(Vec<SearchResult>),
    SelectResult(usize),
    ScrollUp,
    ScrollDown,

    // Mode changes
    EnterResultDetail,
    EnterSessionViewer,
    ExitToSearch,
    ShowHelp,
    CloseHelp,

    // Session events
    LoadSession(String),
    SessionQueryChanged(String),
    SessionScrollUp,
    SessionScrollDown,
    SessionSelectUp,
    SessionSelectDown,
    ToggleSessionOrder,

    // Role filter
    ToggleRoleFilter,

    // Display options
    ToggleTruncation,

    // Clipboard
    CopyToClipboard(String),

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
