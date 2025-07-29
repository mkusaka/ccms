use super::events::CopyContent;

#[derive(Clone, Debug)]
pub enum Command {
    None,
    ExecuteSearch,
    ScheduleSearch(u64), // delay in milliseconds
    LoadSession(String),
    CopyToClipboard(CopyContent),
    ShowMessage(String),
    ClearMessage,
    ScheduleClearMessage(u64), // delay in milliseconds
}
