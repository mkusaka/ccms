#[cfg(test)]
mod tests {
    use super::super::cache_service::*;
    use std::path::Path;

    #[test]
    fn test_cache_service_creation() {
        let _cache = CacheService::new();
        // Ensure it can be created
    }

    #[test]
    fn test_get_messages_nonexistent_file() {
        let mut cache = CacheService::new();
        let result = cache.get_messages(Path::new("/nonexistent/file.jsonl"));

        assert!(result.is_err());
    }
}
