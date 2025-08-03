use super::events::CopyContent;

#[derive(Clone, Debug, PartialEq)]
pub enum Command {
    None,
    ExecuteSearch,
    ExecuteSessionSearch, // Execute search with session_id filter
    ScheduleSearch(u64),  // delay in milliseconds
    LoadSession(String),
    CopyToClipboard(CopyContent),
    ShowMessage(String),
    ClearMessage,
    ScheduleClearMessage(u64), // delay in milliseconds
}
