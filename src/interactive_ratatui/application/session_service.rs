use crate::SessionMessage;
use crate::interactive_ratatui::application::cache_service::CacheService;
use crate::interactive_ratatui::domain::filter::SessionFilter;
use crate::interactive_ratatui::domain::models::SessionOrder;
use crate::interactive_ratatui::domain::session_list_item::SessionListItem;
use anyhow::Result;
use std::path::Path;
use std::sync::{Arc, Mutex};

pub struct SessionService {
    cache: Arc<Mutex<CacheService>>,
}

impl SessionService {
    pub fn new(cache: Arc<Mutex<CacheService>>) -> Self {
        Self { cache }
    }

    pub fn load_session(&self, file_path: &str) -> Result<Vec<SessionMessage>> {
        let path = Path::new(file_path);
        let mut cache = self.cache.lock().unwrap();
        let cached_file = cache.get_messages(path)?;
        Ok(cached_file.messages.clone())
    }

    pub fn get_raw_lines(&self, file_path: &str) -> Result<Vec<String>> {
        let path = Path::new(file_path);
        let mut cache = self.cache.lock().unwrap();
        let cached_file = cache.get_messages(path)?;
        Ok(cached_file.raw_lines.clone())
    }

    #[allow(dead_code)]
    pub fn filter_messages(messages: &[String], query: &str) -> Vec<usize> {
        // Convert raw JSON strings to SessionListItems for search
        let items: Vec<SessionListItem> = messages
            .iter()
            .enumerate()
            .filter_map(|(idx, line)| SessionListItem::from_json_line(idx, line))
            .collect();

        SessionFilter::filter_messages(&items, query)
    }

    #[allow(dead_code)]
    pub fn sort_messages(messages: &mut [SessionMessage], order: SessionOrder) {
        match order {
            SessionOrder::Ascending => {
                messages.sort_by(|a, b| {
                    let a_ts = a.get_timestamp().unwrap_or("");
                    let b_ts = b.get_timestamp().unwrap_or("");
                    a_ts.cmp(b_ts)
                });
            }
            SessionOrder::Descending => {
                messages.sort_by(|a, b| {
                    let a_ts = a.get_timestamp().unwrap_or("");
                    let b_ts = b.get_timestamp().unwrap_or("");
                    b_ts.cmp(a_ts)
                });
            }
            SessionOrder::Original => {
                // Keep original order
            }
        }
    }
}
