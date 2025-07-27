use crate::SessionMessage;
use crate::interactive_ratatui::application::cache_service::CacheService;
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

}
