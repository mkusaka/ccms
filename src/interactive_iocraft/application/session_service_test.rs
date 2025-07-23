#[cfg(test)]
mod tests {
    use super::super::{CacheService, SessionService};
    #[allow(unused_imports)]
    use crate::SessionMessage;
    use crate::interactive_iocraft::domain::models::SessionOrder;
    use std::sync::{Arc, Mutex};

    fn create_test_service() -> (SessionService, Arc<Mutex<CacheService>>) {
        let cache = Arc::new(Mutex::new(CacheService::new()));
        let service = SessionService::new(cache.clone());
        (service, cache)
    }

    #[test]
    fn test_session_service_new() {
        let (_service, _cache) = create_test_service();
        // Service should be created successfully
    }

    #[test]
    fn test_load_session_non_existent_file() {
        let (service, _cache) = create_test_service();

        let result = service.load_session("non_existent_file.jsonl");
        assert!(result.is_err());
    }

    #[test]
    #[ignore] // TODO: Fix JSON format for assistant messages
    fn test_load_session_from_file() {
        let (service, _cache) = create_test_service();

        // Create a temporary file
        let tmp_file = std::env::temp_dir().join("test_session_service.jsonl");
        let file_path = tmp_file.to_str().unwrap();

        std::fs::write(&tmp_file, r#"{"type":"user","uuid":"1","timestamp":"2024-01-01T00:00:00Z","sessionId":"test","version":"1.0","cwd":"/","userType":"user","isSidechain":false,"parentUuid":null,"message":{"role":"user","content":"Hello"}}
{"type":"assistant","uuid":"2","timestamp":"2024-01-01T00:00:01Z","sessionId":"test","version":"1.0","cwd":"/","userType":"user","isSidechain":false,"parentUuid":"1","message":{"role":"assistant","content":"Hi there"}}
"#)
            .expect("Failed to write test file");

        let result = service.load_session(file_path);
        assert!(result.is_ok());

        let messages = result.unwrap();
        assert_eq!(messages.len(), 2);

        // Check message contents using get_content_text
        assert_eq!(messages[0].get_content_text(), "Hello");
        assert_eq!(messages[1].get_content_text(), "Hi there");

        // Clean up
        std::fs::remove_file(&tmp_file).ok();
    }

    #[test]
    fn test_get_raw_lines() {
        let (service, _cache) = create_test_service();

        // Create a temporary file
        let tmp_file = std::env::temp_dir().join("test_session_raw_lines.jsonl");
        let file_path = tmp_file.to_str().unwrap();

        let raw_content = r#"{"role": "user", "content": "Line 1"}
{"role": "assistant", "content": "Line 2"}
"#;
        std::fs::write(&tmp_file, raw_content).expect("Failed to write test file");

        let result = service.get_raw_lines(file_path);
        assert!(result.is_ok());

        let raw_lines = result.unwrap();
        assert_eq!(raw_lines.len(), 2);
        assert!(raw_lines[0].contains("Line 1"));
        assert!(raw_lines[1].contains("Line 2"));

        // Clean up
        std::fs::remove_file(&tmp_file).ok();
    }

    #[test]
    fn test_filter_messages_static() {
        let messages = vec![
            "Hello world".to_string(),
            "Goodbye world".to_string(),
            "Hello again".to_string(),
        ];

        let filtered = SessionService::filter_messages(&messages, "hello");
        assert_eq!(filtered, vec![0, 2]);
    }

    #[test]
    fn test_sort_messages() {
        // Since we can't easily create SessionMessage structs, we'll skip the detailed test
        // The actual functionality would be tested with real message loading

        // Create a temporary file with messages having different timestamps
        let tmp_file = std::env::temp_dir().join("test_session_sort.jsonl");
        let file_path = tmp_file.to_str().unwrap();

        std::fs::write(&tmp_file, r#"{"type":"user","timestamp":"2024-01-02T00:00:00Z","uuid":"1","sessionId":"test","version":"1.0","cwd":"/","userType":"user","isSidechain":false,"parentUuid":null,"message":{"role":"user","content":"Second"}}
{"type":"user","timestamp":"2024-01-01T00:00:00Z","uuid":"2","sessionId":"test","version":"1.0","cwd":"/","userType":"user","isSidechain":false,"parentUuid":null,"message":{"role":"user","content":"First"}}
{"type":"user","timestamp":"2024-01-03T00:00:00Z","uuid":"3","sessionId":"test","version":"1.0","cwd":"/","userType":"user","isSidechain":false,"parentUuid":null,"message":{"role":"user","content":"Third"}}
"#)
            .expect("Failed to write test file");

        let (service, _cache) = create_test_service();
        let result = service.load_session(file_path);
        assert!(result.is_ok());

        let messages = result.unwrap();
        assert_eq!(messages.len(), 3);

        // Test ascending sort
        let mut asc_messages = messages.clone();
        SessionService::sort_messages(&mut asc_messages, SessionOrder::Ascending);
        assert_eq!(asc_messages[0].get_content_text(), "First");
        assert_eq!(asc_messages[2].get_content_text(), "Third");

        // Test descending sort
        let mut desc_messages = messages.clone();
        SessionService::sort_messages(&mut desc_messages, SessionOrder::Descending);
        assert_eq!(desc_messages[0].get_content_text(), "Third");
        assert_eq!(desc_messages[2].get_content_text(), "First");

        // Test original order
        let mut orig_messages = messages.clone();
        SessionService::sort_messages(&mut orig_messages, SessionOrder::Original);
        assert_eq!(orig_messages[0].get_content_text(), "Second");

        // Clean up
        std::fs::remove_file(&tmp_file).ok();
    }

    #[test]
    #[ignore] // TODO: Fix JSON format for assistant messages
    fn test_load_session_with_unicode() {
        let (service, _cache) = create_test_service();

        // Create a temporary file with Unicode content
        let tmp_file = std::env::temp_dir().join("test_session_unicode.jsonl");
        let file_path = tmp_file.to_str().unwrap();

        std::fs::write(&tmp_file, r#"{"type":"user","uuid":"1","timestamp":"2024-01-01T00:00:00Z","sessionId":"test","version":"1.0","cwd":"/","userType":"user","isSidechain":false,"parentUuid":null,"message":{"role":"user","content":"こんにちは世界"}}
{"type":"assistant","uuid":"2","timestamp":"2024-01-01T00:00:01Z","sessionId":"test","version":"1.0","cwd":"/","userType":"user","isSidechain":false,"parentUuid":"1","message":{"role":"assistant","content":"Hello 世界"}}
"#)
            .expect("Failed to write test file");

        let result = service.load_session(file_path);
        assert!(result.is_ok());

        let messages = result.unwrap();
        assert_eq!(messages.len(), 2);

        // Check Unicode content is preserved
        let content1 = messages[0].get_content_text();
        assert!(content1.contains("世界"));
        assert!(content1.contains("こんにちは"));

        let content2 = messages[1].get_content_text();
        assert!(content2.contains("世界"));

        // Clean up
        std::fs::remove_file(&tmp_file).ok();
    }

    #[test]
    fn test_cached_access() {
        let (service, _cache) = create_test_service();

        // Create a temporary file
        let tmp_file = std::env::temp_dir().join("test_session_cache.jsonl");
        let file_path = tmp_file.to_str().unwrap();

        std::fs::write(&tmp_file, r#"{"type":"user","uuid":"1","timestamp":"2024-01-01T00:00:00Z","sessionId":"test","version":"1.0","cwd":"/","userType":"user","isSidechain":false,"parentUuid":null,"message":{"role":"user","content":"Test caching"}}
"#)
            .expect("Failed to write test file");

        // First load
        let result1 = service.load_session(file_path);
        assert!(result1.is_ok());

        // Second load should use cache
        let result2 = service.load_session(file_path);
        assert!(result2.is_ok());

        // Both should return the same content
        assert_eq!(result1.unwrap().len(), result2.unwrap().len());

        // Clean up
        std::fs::remove_file(&tmp_file).ok();
    }
}
