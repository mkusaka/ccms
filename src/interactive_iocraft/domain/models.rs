use crate::SessionMessage;
use crate::query::condition::SearchResult;
use std::time::SystemTime;

#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub enum Mode {
    #[default]
    Search,
    ResultDetail,
    SessionViewer,
    Help,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum SessionOrder {
    Ascending,
    Descending,
    Original,
}

#[derive(Debug)]
pub struct CachedFile {
    pub messages: Vec<SessionMessage>,
    pub raw_lines: Vec<String>,
    pub last_modified: SystemTime,
}

// Search request and response for async communication
#[derive(Clone)]
pub struct SearchRequest {
    pub id: u64,
    pub query: String,
    pub role_filter: Option<String>,
    pub pattern: String,
}

pub struct SearchResponse {
    pub id: u64,
    pub results: Vec<SearchResult>,
}
