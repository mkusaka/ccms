/// Application-wide messages
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AppMessage {
    // Navigation
    Quit,
    ChangeMode(AppMode),
    
    // Search operations
    SearchQueryChanged(String),
    SearchRequested,
    SearchCompleted, // Results are stored in state, not in message
    SearchFailed(String),
    
    // Result navigation
    ResultUp,
    ResultDown,
    ResultPageUp,
    ResultPageDown,
    ResultHome,
    ResultEnd,
    ResultSelect(usize),
    
    // Result detail
    EnterResultDetail(usize),
    ExitResultDetail,
    DetailScrollUp,
    DetailScrollDown,
    DetailPageUp,
    DetailPageDown,
    
    // Session viewer
    EnterSessionViewer(String),
    ExitSessionViewer,
    SessionScrollUp,
    SessionScrollDown,
    SessionPageUp,
    SessionPageDown,
    SessionToggleOrder,
    SessionSearchStart,
    SessionSearchEnd,
    SessionQueryChanged(String),
    
    // Clipboard operations
    CopyMessage,
    CopySession,
    CopyTimestamp,
    CopyRawJson,
    CopySessionId,
    ClipboardSuccess(String),
    ClipboardFailed(String),
    
    // UI operations
    ToggleTruncation,
    ToggleRoleFilter,
    ShowHelp,
    ExitHelp,
    
    // Status updates
    ShowMessage(String),
    ClearMessage,
    
    // Async operations
    DebouncedSearchReady(String),
    SessionLoaded(String, Vec<String>),
    SessionLoadFailed(String),
    
    // Error handling
    ShowError(String, String), // (error_type, details)
    CloseError,
    RetryLastOperation,
}

/// Application modes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AppMode {
    Search,
    ResultDetail,
    SessionViewer,
    Help,
    Error,
}

/// Component identifiers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ComponentId {
    SearchInput,
    ResultList,
    ResultDetail,
    SessionViewer,
    HelpDialog,
    ErrorDialog,
    StatusBar,
    GlobalShortcuts,
}