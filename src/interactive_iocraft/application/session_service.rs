use crate::interactive_iocraft::SessionMessage;
use crate::interactive_iocraft::application::cache_service::CacheService;
use crate::interactive_iocraft::domain::filter::SessionFilter;
use crate::interactive_iocraft::domain::models::SessionOrder;
use crate::interactive_iocraft::domain::session_list_item::SessionListItem;
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
        let mut cache = self.cache.lock()
            .map_err(|e| anyhow::anyhow!("Failed to acquire cache lock: {}", e))?;
        let cached_file = cache.get_messages(path)?;
        Ok(cached_file.messages.clone())
    }

    pub fn get_raw_lines(&self, file_path: &str) -> Result<Vec<String>> {
        let path = Path::new(file_path);
        let mut cache = self.cache.lock()
            .map_err(|e| anyhow::anyhow!("Failed to acquire cache lock: {}", e))?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interactive_iocraft::SessionMessage;
    use std::fs;
    use std::io::Write;
    use tempfile::NamedTempFile;
    
    fn create_test_jsonl() -> (NamedTempFile, Vec<String>) {
        let mut file = NamedTempFile::new().unwrap();
        let lines = vec![
            r#"{"uuid":"123","timestamp":"2023-11-20T10:00:00Z","sessionId":"abc","role":"user","text":"First message","projectPath":"/test"}"#.to_string(),
            r#"{"uuid":"124","timestamp":"2023-11-20T10:01:00Z","sessionId":"abc","role":"assistant","text":"Second message","projectPath":"/test"}"#.to_string(),
            r#"{"uuid":"125","timestamp":"2023-11-20T10:02:00Z","sessionId":"abc","role":"system","text":"Third message","projectPath":"/test"}"#.to_string(),
        ];
        
        for line in &lines {
            writeln!(file, "{}", line).unwrap();
        }
        file.flush().unwrap();
        (file, lines)
    }
    
    #[test]
    fn test_session_service_new() {
        let cache = Arc::new(Mutex::new(CacheService::new()));
        let service = SessionService::new(cache);
        assert!(true); // Service created successfully
    }
    
    #[test]
    fn test_load_session() {
        let (file, _) = create_test_jsonl();
        let cache = Arc::new(Mutex::new(CacheService::new()));
        let service = SessionService::new(cache);
        
        let result = service.load_session(file.path().to_str().unwrap());
        assert!(result.is_ok());
        let messages = result.unwrap();
        assert_eq!(messages.len(), 3);
    }
    
    #[test]
    fn test_get_raw_lines() {
        let (file, lines) = create_test_jsonl();
        let cache = Arc::new(Mutex::new(CacheService::new()));
        let service = SessionService::new(cache);
        
        let result = service.get_raw_lines(file.path().to_str().unwrap());
        assert!(result.is_ok());
        let raw_lines = result.unwrap();
        assert_eq!(raw_lines.len(), 3);
        
        // Check that raw lines match original lines
        for (i, line) in raw_lines.iter().enumerate() {
            assert_eq!(line.trim(), lines[i]);
        }
    }
    
    #[test]
    fn test_filter_messages() {
        let (_, lines) = create_test_jsonl();
        
        // Test filtering by content
        let indices = SessionService::filter_messages(&lines, "First");
        assert_eq!(indices, vec![0]);
        
        // Test filtering by role
        let indices = SessionService::filter_messages(&lines, "assistant");
        assert_eq!(indices, vec![1]);
        
        // Test case-insensitive filtering
        let indices = SessionService::filter_messages(&lines, "MESSAGE");
        assert_eq!(indices, vec![0, 1, 2]);
        
        // Test no matches
        let indices = SessionService::filter_messages(&lines, "nonexistent");
        assert_eq!(indices, vec![]);
    }
    
    #[test]
    fn test_sort_messages_ascending() {
        let mut messages = vec![
            SessionMessage::User {
                uuid: "3".to_string(),
                timestamp: "2023-11-20T10:02:00Z".to_string(),
                message: serde_json::json!({"text": "Third"}),
            },
            SessionMessage::User {
                uuid: "1".to_string(),
                timestamp: "2023-11-20T10:00:00Z".to_string(),
                message: serde_json::json!({"text": "First"}),
            },
            SessionMessage::User {
                uuid: "2".to_string(),
                timestamp: "2023-11-20T10:01:00Z".to_string(),
                message: serde_json::json!({"text": "Second"}),
            },
        ];
        
        SessionService::sort_messages(&mut messages, SessionOrder::Ascending);
        
        // Check that messages are sorted ascending by timestamp
        assert_eq!(messages[0].get_uuid().unwrap(), "1");
        assert_eq!(messages[1].get_uuid().unwrap(), "2");
        assert_eq!(messages[2].get_uuid().unwrap(), "3");
    }
    
    #[test]
    fn test_sort_messages_descending() {
        let mut messages = vec![
            SessionMessage::User {
                uuid: "1".to_string(),
                timestamp: "2023-11-20T10:00:00Z".to_string(),
                message: serde_json::json!({"text": "First"}),
            },
            SessionMessage::User {
                uuid: "3".to_string(),
                timestamp: "2023-11-20T10:02:00Z".to_string(),
                message: serde_json::json!({"text": "Third"}),
            },
            SessionMessage::User {
                uuid: "2".to_string(),
                timestamp: "2023-11-20T10:01:00Z".to_string(),
                message: serde_json::json!({"text": "Second"}),
            },
        ];
        
        SessionService::sort_messages(&mut messages, SessionOrder::Descending);
        
        // Check that messages are sorted descending by timestamp
        assert_eq!(messages[0].get_uuid().unwrap(), "3");
        assert_eq!(messages[1].get_uuid().unwrap(), "2");
        assert_eq!(messages[2].get_uuid().unwrap(), "1");
    }
    
    #[test]
    fn test_sort_messages_original() {
        let mut messages = vec![
            SessionMessage::User {
                uuid: "3".to_string(),
                timestamp: "2023-11-20T10:02:00Z".to_string(),
                message: serde_json::json!({"text": "Third"}),
            },
            SessionMessage::User {
                uuid: "1".to_string(),
                timestamp: "2023-11-20T10:00:00Z".to_string(),
                message: serde_json::json!({"text": "First"}),
            },
            SessionMessage::User {
                uuid: "2".to_string(),
                timestamp: "2023-11-20T10:01:00Z".to_string(),
                message: serde_json::json!({"text": "Second"}),
            },
        ];
        
        let original_order: Vec<String> = messages.iter()
            .map(|m| m.get_uuid().unwrap().to_string())
            .collect();
        
        SessionService::sort_messages(&mut messages, SessionOrder::Original);
        
        // Check that order remains unchanged
        let after_order: Vec<String> = messages.iter()
            .map(|m| m.get_uuid().unwrap().to_string())
            .collect();
        assert_eq!(original_order, after_order);
    }
    
    #[test]
    fn test_load_session_nonexistent_file() {
        let cache = Arc::new(Mutex::new(CacheService::new()));
        let service = SessionService::new(cache);
        
        let result = service.load_session("/nonexistent/file.jsonl");
        assert!(result.is_err());
    }
    
    #[test]
    fn test_concurrent_access() {
        let (file, _) = create_test_jsonl();
        let cache = Arc::new(Mutex::new(CacheService::new()));
        let service = Arc::new(SessionService::new(cache));
        
        // Test concurrent access from multiple threads
        let mut handles = vec![];
        for _ in 0..5 {
            let service_clone = service.clone();
            let file_path = file.path().to_str().unwrap().to_string();
            let handle = std::thread::spawn(move || {
                service_clone.get_raw_lines(&file_path)
            });
            handles.push(handle);
        }
        
        // All threads should succeed
        for handle in handles {
            let result = handle.join().unwrap();
            assert!(result.is_ok());
            assert_eq!(result.unwrap().len(), 3);
        }
    }
}
