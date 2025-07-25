#[cfg(test)]
mod feature_tests {
    use crate::interactive_iocraft::application::{SearchService, SessionService, CacheService};
    use crate::interactive_iocraft::domain::models::{SearchRequest, Mode, SessionOrder};
    use crate::interactive_iocraft::domain::filter::{SearchFilter, SessionFilter};
    use crate::interactive_iocraft::domain::session_list_item::SessionListItem;
    use crate::query::condition::SearchResult;
    use std::sync::{Arc, Mutex};
    use std::fs;
    use std::io::Write;
    use tempfile::TempDir;
    
    struct TestEnvironment {
        _dir: TempDir,
        search_service: Arc<SearchService>,
        session_service: Arc<SessionService>,
        cache_service: Arc<Mutex<CacheService>>,
        file_paths: Vec<String>,
    }
    
    impl TestEnvironment {
        fn new() -> Self {
            let dir = TempDir::new().unwrap();
            let mut file_paths = Vec::new();
            
            // Create test files with various content
            for i in 0..5 {
                let file_path = dir.path().join(format!("session_{}.jsonl", i));
                let mut file = fs::File::create(&file_path).unwrap();
                
                // Write diverse content for testing
                writeln!(file, r#"{{"uuid":"{}a","timestamp":"17000000{}0","sessionId":"session{}","role":"user","text":"User query about Rust programming {}","projectPath":"/project{}"}}}"#, i, i, i, i, i).unwrap();
                writeln!(file, r#"{{"uuid":"{}b","timestamp":"17000000{}1","sessionId":"session{}","role":"assistant","text":"Here's information about Rust {}","projectPath":"/project{}"}}}"#, i, i, i, i, i).unwrap();
                writeln!(file, r#"{{"uuid":"{}c","timestamp":"17000000{}2","sessionId":"session{}","role":"system","text":"System: Processing request {}","projectPath":"/project{}"}}}"#, i, i, i, i, i).unwrap();
                writeln!(file, r#"{{"uuid":"{}d","timestamp":"17000000{}3","sessionId":"session{}","role":"user","text":"Follow-up question about async {}","projectPath":"/project{}"}}}"#, i, i, i, i, i).unwrap();
                writeln!(file, r#"{{"uuid":"{}e","timestamp":"17000000{}4","sessionId":"session{}","role":"assistant","text":"Async in Rust works like this {}","projectPath":"/project{}"}}}"#, i, i, i, i, i).unwrap();
                
                file.flush().unwrap();
                file_paths.push(file_path.to_string_lossy().to_string());
            }
            
            let cache = Arc::new(Mutex::new(CacheService::new()));
            let search_service = Arc::new(SearchService::new(file_paths.clone(), false).unwrap());
            let session_service = Arc::new(SessionService::new(cache.clone()));
            
            Self {
                _dir: dir,
                search_service,
                session_service,
                cache_service: cache,
                file_paths,
            }
        }
    }
    
    #[test]
    fn test_search_feature() {
        let env = TestEnvironment::new();
        
        // Test 1: Basic search
        let request = SearchRequest {
            id: 1,
            query: "Rust".to_string(),
            pattern: env.file_paths.join(","),
            role_filter: None,
        };
        
        let response = env.search_service.search(request).unwrap();
        assert!(response.results.len() > 5); // Should find multiple matches
        assert!(response.results.iter().all(|r| r.text.contains("Rust")));
        
        // Test 2: Search with role filter
        let request = SearchRequest {
            id: 2,
            query: "".to_string(),
            pattern: env.file_paths[0].clone(),
            role_filter: Some("user".to_string()),
        };
        
        let response = env.search_service.search(request).unwrap();
        assert_eq!(response.results.len(), 2); // Two user messages per file
        assert!(response.results.iter().all(|r| r.role == "user"));
        
        // Test 3: Complex query
        let request = SearchRequest {
            id: 3,
            query: "async AND Rust".to_string(),
            pattern: env.file_paths.join(","),
            role_filter: None,
        };
        
        let response = env.search_service.search(request).unwrap();
        assert!(response.results.len() > 0);
        assert!(response.results.iter().all(|r| 
            r.text.contains("async") && r.text.contains("Rust")
        ));
        
        // Test 4: NOT query
        let request = SearchRequest {
            id: 4,
            query: "Rust NOT async".to_string(),
            pattern: env.file_paths[0].clone(),
            role_filter: None,
        };
        
        let response = env.search_service.search(request).unwrap();
        assert!(response.results.iter().all(|r| 
            r.text.contains("Rust") && !r.text.contains("async")
        ));
    }
    
    #[test]
    fn test_session_viewing_feature() {
        let env = TestEnvironment::new();
        
        // Load a session
        let messages = env.session_service.load_session(&env.file_paths[0]).unwrap();
        assert_eq!(messages.len(), 5); // 5 messages per file
        
        // Get raw lines
        let raw_lines = env.session_service.get_raw_lines(&env.file_paths[0]).unwrap();
        assert_eq!(raw_lines.len(), 5);
        
        // Test session filtering
        let filtered_indices = SessionService::filter_messages(&raw_lines, "async");
        assert_eq!(filtered_indices.len(), 2); // Two messages mention async
        
        // Test sorting
        let mut messages_clone = messages.clone();
        SessionService::sort_messages(&mut messages_clone, SessionOrder::Descending);
        // First message should have the highest timestamp
        let first_ts = messages_clone[0].get_timestamp().unwrap();
        let last_ts = messages_clone[messages_clone.len() - 1].get_timestamp().unwrap();
        assert!(first_ts > last_ts);
    }
    
    #[test]
    fn test_caching_feature() {
        let env = TestEnvironment::new();
        
        // First access - loads from file
        let start = std::time::Instant::now();
        let messages1 = env.session_service.load_session(&env.file_paths[0]).unwrap();
        let first_load_time = start.elapsed();
        
        // Second access - should use cache and be faster
        let start = std::time::Instant::now();
        let messages2 = env.session_service.load_session(&env.file_paths[0]).unwrap();
        let second_load_time = start.elapsed();
        
        assert_eq!(messages1.len(), messages2.len());
        // Cache access should typically be faster (though not guaranteed in tests)
        
        // Test cache invalidation
        std::thread::sleep(std::time::Duration::from_millis(10));
        let mut file = fs::OpenOptions::new()
            .append(true)
            .open(&env.file_paths[0])
            .unwrap();
        writeln!(file, r#"{{"uuid":"new","timestamp":"1700000099","sessionId":"session0","role":"user","text":"New message","projectPath":"/project0"}}"#).unwrap();
        file.flush().unwrap();
        
        // Should reload due to file modification
        let messages3 = env.session_service.load_session(&env.file_paths[0]).unwrap();
        assert_eq!(messages3.len(), messages1.len() + 1);
    }
    
    #[test]
    fn test_cross_feature_workflow() {
        let env = TestEnvironment::new();
        
        // Step 1: Search for messages about Rust
        let search_request = SearchRequest {
            id: 1,
            query: "Rust programming".to_string(),
            pattern: env.file_paths.join(","),
            role_filter: None,
        };
        
        let search_response = env.search_service.search(search_request).unwrap();
        assert!(!search_response.results.is_empty());
        
        // Step 2: Take first result and view its session
        let first_result = &search_response.results[0];
        let session_messages = env.session_service.load_session(&first_result.file).unwrap();
        
        // Step 3: Filter messages in that session
        let raw_lines = env.session_service.get_raw_lines(&first_result.file).unwrap();
        let async_messages = SessionService::filter_messages(&raw_lines, "async");
        assert!(!async_messages.is_empty());
        
        // Step 4: Apply role filter to search results
        let mut filtered_results = search_response.results.clone();
        let filter = SearchFilter::new(Some("assistant".to_string()));
        filter.apply(&mut filtered_results).unwrap();
        assert!(filtered_results.iter().all(|r| r.role == "assistant"));
        
        // Step 5: Verify caching is working
        let cache_guard = env.cache_service.lock().unwrap();
        let cached = cache_guard.get(&first_result.file);
        assert!(cached.is_some());
    }
    
    #[test]
    fn test_performance_with_large_dataset() {
        let env = TestEnvironment::new();
        
        // Test searching across all files
        let start = std::time::Instant::now();
        let request = SearchRequest {
            id: 1,
            query: "".to_string(), // Match all
            pattern: env.file_paths.join(","),
            role_filter: None,
        };
        
        let response = env.search_service.search(request).unwrap();
        let search_time = start.elapsed();
        
        // Should return all messages (5 files * 5 messages = 25)
        assert_eq!(response.results.len(), 25);
        
        // Search should complete reasonably quickly
        assert!(search_time.as_millis() < 1000); // Less than 1 second
        
        // Test caching performance
        let mut total_cached_time = std::time::Duration::new(0, 0);
        for path in &env.file_paths {
            let start = std::time::Instant::now();
            let _ = env.session_service.load_session(path).unwrap();
            total_cached_time += start.elapsed();
        }
        
        // Loading all sessions should be fast
        assert!(total_cached_time.as_millis() < 500); // Less than 500ms total
    }
    
    #[test]
    fn test_error_handling() {
        let env = TestEnvironment::new();
        
        // Test invalid query
        let request = SearchRequest {
            id: 1,
            query: "AND AND AND".to_string(), // Invalid syntax
            pattern: env.file_paths[0].clone(),
            role_filter: None,
        };
        
        let result = env.search_service.search(request);
        assert!(result.is_err());
        
        // Test non-existent file
        let result = env.session_service.load_session("/non/existent/file.jsonl");
        assert!(result.is_err());
        
        // Test invalid role filter (should just return no results)
        let request = SearchRequest {
            id: 2,
            query: "".to_string(),
            pattern: env.file_paths[0].clone(),
            role_filter: Some("invalid_role".to_string()),
        };
        
        let response = env.search_service.search(request).unwrap();
        assert_eq!(response.results.len(), 0);
    }
    
    #[test]
    fn test_unicode_and_special_characters() {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("unicode_test.jsonl");
        let mut file = fs::File::create(&file_path).unwrap();
        
        // Write messages with unicode and special characters
        writeln!(file, r#"{{"uuid":"u1","timestamp":"1700000000","sessionId":"unicode","role":"user","text":"こんにちは 🌸 Unicode test","projectPath":"/unicode"}}"#).unwrap();
        writeln!(file, r#"{{"uuid":"u2","timestamp":"1700000001","sessionId":"unicode","role":"assistant","text":"Special chars: <>&\"'","projectPath":"/unicode"}}"#).unwrap();
        writeln!(file, r#"{{"uuid":"u3","timestamp":"1700000002","sessionId":"unicode","role":"user","text":"Emoji test: 🎉🎊🎈","projectPath":"/unicode"}}"#).unwrap();
        file.flush().unwrap();
        
        let cache = Arc::new(Mutex::new(CacheService::new()));
        let search_service = Arc::new(SearchService::new(
            vec![file_path.to_string_lossy().to_string()],
            false,
        ).unwrap());
        let session_service = Arc::new(SessionService::new(cache));
        
        // Test searching for unicode
        let request = SearchRequest {
            id: 1,
            query: "こんにちは".to_string(),
            pattern: file_path.to_string_lossy().to_string(),
            role_filter: None,
        };
        
        let response = search_service.search(request).unwrap();
        assert_eq!(response.results.len(), 1);
        assert!(response.results[0].text.contains("こんにちは"));
        
        // Test loading session with special characters
        let messages = session_service.load_session(&file_path.to_string_lossy()).unwrap();
        assert_eq!(messages.len(), 3);
        
        // Test filtering with emoji
        let raw_lines = session_service.get_raw_lines(&file_path.to_string_lossy()).unwrap();
        let emoji_indices = SessionService::filter_messages(&raw_lines, "🎉");
        assert_eq!(emoji_indices.len(), 1);
    }
}