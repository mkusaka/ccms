#[derive(Clone, Debug)]
#[allow(dead_code)]
pub enum Command {
    None,
    ExecuteSearch,
    ScheduleSearch(u64), // delay in milliseconds
    LoadSession(String),
    CopyToClipboard(String),
    ShowMessage(String),
    ClearMessage,
    ScheduleClearMessage(u64), // delay in milliseconds
}
