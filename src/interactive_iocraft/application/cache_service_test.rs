#[cfg(test)]
mod tests {
    use super::super::CacheService;
    use std::path::Path;

    #[test]
    fn test_cache_new_file() {
        let mut cache = CacheService::new();
        // Non-existent file should return an error
        assert!(
            cache
                .get_messages(Path::new("non_existent_file.jsonl"))
                .is_err()
        );
    }

    #[test]
    fn test_cache_stores_and_retrieves() {
        let mut cache = CacheService::new();

        // Create a temporary file
        let tmp_file = std::env::temp_dir().join("test_ccms_cache.jsonl");
        std::fs::write(&tmp_file, r#"{"type":"user","uuid":"test-uuid","timestamp":"2024-01-01T00:00:00Z","sessionId":"test","version":"1.0","cwd":"/","userType":"user","isSidechain":false,"parentUuid":null,"message":{"role":"user","content":"Hello world"}}
"#)
            .expect("Failed to write test file");

        // First access should load from disk
        let result = cache.get_messages(&tmp_file);
        assert!(result.is_ok());
        let cached_file = result.unwrap();
        assert_eq!(cached_file.messages.len(), 1);

        // Second access should use cache
        let result2 = cache.get_messages(&tmp_file);
        assert!(result2.is_ok());
        let cached_file2 = result2.unwrap();
        assert_eq!(cached_file2.messages.len(), 1);

        // Clean up
        std::fs::remove_file(&tmp_file).ok();
    }

    #[test]
    #[ignore] // TODO: Fix JSON format for assistant messages
    fn test_cache_detects_file_changes() {
        let mut cache = CacheService::new();

        // Create a temporary file
        let tmp_file = std::env::temp_dir().join("test_ccms_cache_changes.jsonl");
        std::fs::write(&tmp_file, r#"{"type":"user","uuid":"test-uuid","timestamp":"2024-01-01T00:00:00Z","sessionId":"test","version":"1.0","cwd":"/","userType":"user","isSidechain":false,"parentUuid":null,"message":{"role":"user","content":"Initial message"}}
"#)
            .expect("Failed to write test file");

        // Load initial content
        let result = cache.get_messages(&tmp_file);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().messages.len(), 1);

        // Sleep briefly to ensure different modification time
        std::thread::sleep(std::time::Duration::from_millis(10));

        // Update file content
        std::fs::write(&tmp_file, r#"{"type":"user","uuid":"test-uuid-1","timestamp":"2024-01-01T00:00:00Z","sessionId":"test","version":"1.0","cwd":"/","userType":"user","isSidechain":false,"parentUuid":null,"message":{"role":"user","content":"Initial message"}}
{"type":"assistant","uuid":"test-uuid-2","timestamp":"2024-01-01T00:00:01Z","sessionId":"test","version":"1.0","cwd":"/","userType":"user","isSidechain":false,"parentUuid":"test-uuid-1","message":{"role":"assistant","content":"Response"}}
"#)
            .expect("Failed to update test file");

        // Cache should detect change and reload
        let result2 = cache.get_messages(&tmp_file);
        assert!(result2.is_ok());
        assert_eq!(result2.unwrap().messages.len(), 2);

        // Clean up
        std::fs::remove_file(&tmp_file).ok();
    }

    #[test]
    fn test_clear_cache() {
        let mut cache = CacheService::new();

        // Create a temporary file
        let tmp_file = std::env::temp_dir().join("test_ccms_cache_clear.jsonl");
        std::fs::write(&tmp_file, r#"{"type":"user","uuid":"test-uuid","timestamp":"2024-01-01T00:00:00Z","sessionId":"test","version":"1.0","cwd":"/","userType":"user","isSidechain":false,"parentUuid":null,"message":{"role":"user","content":"Test message"}}
"#)
            .expect("Failed to write test file");

        // Load content
        let result = cache.get_messages(&tmp_file);
        assert!(result.is_ok());

        // Clear cache
        cache.clear();
        // After clearing, accessing the same file should reload from disk
        let result2 = cache.get_messages(&tmp_file);
        assert!(result2.is_ok());

        // Clean up
        std::fs::remove_file(&tmp_file).ok();
    }

    #[test]
    fn test_cache_handles_invalid_json() {
        let mut cache = CacheService::new();

        // Create a temporary file with invalid JSON
        let tmp_file = std::env::temp_dir().join("test_ccms_cache_invalid.jsonl");
        std::fs::write(&tmp_file, "invalid json content").expect("Failed to write test file");

        // Should skip invalid JSON lines but still return a result
        let result = cache.get_messages(&tmp_file);
        assert!(result.is_ok());
        // Invalid lines are skipped, so messages should be empty
        assert_eq!(result.unwrap().messages.len(), 0);

        // Clean up
        std::fs::remove_file(&tmp_file).ok();
    }

    #[test]
    fn test_cache_handles_unicode() {
        let mut cache = CacheService::new();

        // Create a temporary file with Unicode content
        let tmp_file = std::env::temp_dir().join("test_ccms_cache_unicode.jsonl");
        std::fs::write(&tmp_file, r#"{"type":"user","uuid":"test-uuid","timestamp":"2024-01-01T00:00:00Z","sessionId":"test","version":"1.0","cwd":"/","userType":"user","isSidechain":false,"parentUuid":null,"message":{"role":"user","content":"こんにちは世界 🌍"}}
"#)
            .expect("Failed to write test file");

        // Should handle Unicode properly
        let result = cache.get_messages(&tmp_file);
        assert!(result.is_ok());
        let cached_file = result.unwrap();
        assert_eq!(cached_file.messages.len(), 1);
        // Check the message content contains expected text
        let content = cached_file.messages[0].get_content_text();
        assert!(content.contains("こんにちは"));
        assert!(content.contains("🌍"));

        // Clean up
        std::fs::remove_file(&tmp_file).ok();
    }
}
