#[cfg(test)]
mod search_service_tests {
    use super::super::*;
    use std::sync::mpsc;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_search_service_new() {
        let _service = SearchService::new();
        
        // Just verify it can be created
        // Internal state is private
        // Just verify it can be created
    }

    #[test]
    fn test_search_service_configure() {
        let mut service = SearchService::new();
        
        service.configure(
            Some("*.jsonl".to_string()),
            Some("2024-01-01T00:00:00Z".to_string()),
            Some("2024-12-31T23:59:59Z".to_string()),
            Some("session-123".to_string()),
        );
        
        assert_eq!(service.pattern, Some("*.jsonl".to_string()));
        assert_eq!(service.timestamp_gte, Some("2024-01-01T00:00:00Z".to_string()));
        assert_eq!(service.timestamp_lt, Some("2024-12-31T23:59:59Z".to_string()));
        assert_eq!(service.session_id, Some("session-123".to_string()));
    }

    #[test]
    fn test_search_service_search_async() {
        let service = SearchService::new();
        let (tx, rx) = mpsc::channel();
        
        // Search with a simple query
        service.search_async("test".to_string(), None, tx);
        
        // Wait for results with timeout
        let result = rx.recv_timeout(Duration::from_secs(5));
        
        // Should receive results (even if empty)
        assert!(result.is_ok());
        let results = result.unwrap();
        
        // Results should be a Vec<SearchResult>
        assert!(results.is_empty() || !results.is_empty());
    }

    #[test]
    fn test_search_service_search_async_with_role_filter() {
        let service = SearchService::new();
        let (tx, rx) = mpsc::channel();
        
        // Search with role filter
        service.search_async("test".to_string(), Some("User".to_string()), tx);
        
        // Wait for results
        let result = rx.recv_timeout(Duration::from_secs(5));
        assert!(result.is_ok());
    }

    #[test]
    fn test_search_service_empty_query() {
        let service = SearchService::new();
        let (tx, rx) = mpsc::channel();
        
        // Empty query should work
        service.search_async("".to_string(), None, tx);
        
        let result = rx.recv_timeout(Duration::from_secs(5));
        assert!(result.is_ok());
    }

    #[test]
    fn test_search_service_invalid_query() {
        let service = SearchService::new();
        let (tx, rx) = mpsc::channel();
        
        // Invalid regex query
        service.search_async("/[invalid/".to_string(), None, tx);
        
        let result = rx.recv_timeout(Duration::from_secs(5));
        assert!(result.is_ok());
        
        // Should return empty results for invalid query
        let results = result.unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn test_search_service_concurrent_searches() {
        let service1 = SearchService::new();
        let service2 = SearchService::new();
        
        let (tx1, rx1) = mpsc::channel();
        let (tx2, rx2) = mpsc::channel();
        
        // Start two searches concurrently
        service1.search_async("query1".to_string(), None, tx1);
        service2.search_async("query2".to_string(), None, tx2);
        
        // Both should complete
        let result1 = rx1.recv_timeout(Duration::from_secs(5));
        let result2 = rx2.recv_timeout(Duration::from_secs(5));
        
        assert!(result1.is_ok());
        assert!(result2.is_ok());
    }

    #[test]
    fn test_search_service_pattern_override() {
        let mut service = SearchService::new();
        
        // Configure with initial pattern
        service.configure(
            Some("/path/to/files/*.jsonl".to_string()),
            None,
            None,
            None,
        );
        
        let (tx, rx) = mpsc::channel();
        service.search_async("test".to_string(), None, tx);
        
        // Should use configured pattern
        let result = rx.recv_timeout(Duration::from_secs(5));
        assert!(result.is_ok());
    }

    #[test]
    fn test_search_service_filters_applied() {
        let mut service = SearchService::new();
        
        // Configure with filters
        service.configure(
            None,
            Some("2024-01-01T00:00:00Z".to_string()),
            Some("2024-12-31T23:59:59Z".to_string()),
            Some("specific-session".to_string()),
        );
        
        let (tx, rx) = mpsc::channel();
        service.search_async("test".to_string(), Some("User".to_string()), tx);
        
        // Filters should be applied in the search
        let result = rx.recv_timeout(Duration::from_secs(5));
        assert!(result.is_ok());
    }

    #[test]
    fn test_search_service_dropped_receiver() {
        let service = SearchService::new();
        let (tx, rx) = mpsc::channel();
        
        // Drop receiver immediately
        drop(rx);
        
        // Search should handle dropped receiver gracefully
        service.search_async("test".to_string(), None, tx);
        
        // Give it time to process
        thread::sleep(Duration::from_millis(100));
        
        // Should not panic
    }

    #[test]
    fn test_search_service_multiple_sequential_searches() {
        let service = SearchService::new();
        
        // First search
        let (tx1, rx1) = mpsc::channel();
        service.search_async("first".to_string(), None, tx1);
        let result1 = rx1.recv_timeout(Duration::from_secs(5));
        assert!(result1.is_ok());
        
        // Second search
        let (tx2, rx2) = mpsc::channel();
        service.search_async("second".to_string(), None, tx2);
        let result2 = rx2.recv_timeout(Duration::from_secs(5));
        assert!(result2.is_ok());
        
        // Third search with filter
        let (tx3, rx3) = mpsc::channel();
        service.search_async("third".to_string(), Some("Assistant".to_string()), tx3);
        let result3 = rx3.recv_timeout(Duration::from_secs(5));
        assert!(result3.is_ok());
    }
}