use crate::interactive_ratatui::domain::models::{SearchRequest, SessionOrder};
use crate::query::condition::SearchResult;

/// Component identifiers for tui-realm
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ComponentId {
    SearchBar,
    ResultList,
    ResultDetail,
    SessionViewer,
    HelpDialog,
}

/// Application messages for tui-realm
#[derive(Debug, Clone, PartialEq)]
pub enum AppMessage {
    // Search related
    QueryChanged(String),
    RoleFilterChanged(Option<String>),
    SessionFilterChanged(Option<String>),
    StartSearch(SearchRequest),
    SearchCompleted(Vec<SearchResult>),
    SearchError(String),
    SearchRequested,
    ToggleRoleFilter,
    
    // Navigation
    NavigateUp,
    NavigateDown,
    NavigatePageUp,
    NavigatePageDown,
    NavigateHome,
    NavigateEnd,
    SelectResult(usize),
    
    // Mode changes
    EnterResultDetail,
    ExitResultDetail,
    EnterSessionViewer(String),
    ExitSessionViewer,
    EnterHelp,
    ExitHelp,
    
    // Session viewer
    SessionQueryChanged(String),
    SessionScrollUp,
    SessionScrollDown,
    ToggleSessionOrder,
    SessionOrderChanged(SessionOrder),
    
    // Clipboard
    CopyToClipboard(String),
    
    // Display options
    ToggleTruncation,
    
    // Status messages
    SetStatus(String),
    ClearStatus,
    
    // Application control
    Quit,
    None,
}