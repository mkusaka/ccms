#[cfg(test)]
mod integration_tests {
    use crate::interactive_iocraft::application::{SearchService, SessionService, CacheService};
    use crate::interactive_iocraft::domain::models::{SearchRequest, Mode};
    use crate::interactive_iocraft::domain::filter::{SearchFilter, SessionFilter};
    use crate::interactive_iocraft::domain::session_list_item::SessionListItem;
    use crate::query::condition::SearchResult;
    use std::sync::{Arc, Mutex};
    use std::fs;
    use std::io::Write;
    use tempfile::{NamedTempFile, TempDir};
    
    fn create_test_session_files() -> (TempDir, Vec<String>) {
        let dir = TempDir::new().unwrap();
        let mut files = Vec::new();
        
        // Create multiple session files
        for i in 0..3 {
            let file_path = dir.path().join(format!("session_{}.jsonl", i));
            let mut file = fs::File::create(&file_path).unwrap();
            
            writeln!(file, r#"{{"uuid":"{}1","timestamp":"170000000{}","sessionId":"session{}","role":"user","text":"User message {}","projectPath":"/project{}"}}}"#, 
                i, i, i, i, i).unwrap();
            writeln!(file, r#"{{"uuid":"{}2","timestamp":"170000001{}","sessionId":"session{}","role":"assistant","text":"Assistant response {}","projectPath":"/project{}"}}}"#, 
                i, i, i, i, i).unwrap();
            writeln!(file, r#"{{"uuid":"{}3","timestamp":"170000002{}","sessionId":"session{}","role":"system","text":"System info {}","projectPath":"/project{}"}}}"#, 
                i, i, i, i, i).unwrap();
            
            file.flush().unwrap();
            files.push(file_path.to_string_lossy().to_string());
        }
        
        (dir, files)
    }
    
    #[test]
    fn test_search_service_integration() {
        let (_dir, files) = create_test_session_files();
        let search_service = SearchService::new(vec![files[0].clone()], false).unwrap();
        
        // Test search with query
        let request = SearchRequest {
            id: 1,
            query: "User message".to_string(),
            pattern: files[0].clone(),
            role_filter: None,
        };
        
        let response = search_service.search(request).unwrap();
        assert_eq!(response.results.len(), 1);
        assert!(response.results[0].text.contains("User message"));
        
        // Test search with role filter
        let request = SearchRequest {
            id: 2,
            query: "".to_string(),
            pattern: files[0].clone(),
            role_filter: Some("assistant".to_string()),
        };
        
        let response = search_service.search(request).unwrap();
        assert_eq!(response.results.len(), 1);
        assert_eq!(response.results[0].role, "assistant");
    }
    
    #[test]
    fn test_session_service_integration() {
        let (_dir, files) = create_test_session_files();
        let cache = Arc::new(Mutex::new(CacheService::new()));
        let session_service = SessionService::new(cache.clone());
        
        // Load session
        let messages = session_service.load_session(&files[0]).unwrap();
        assert_eq!(messages.len(), 3);
        
        // Get raw lines
        let raw_lines = session_service.get_raw_lines(&files[0]).unwrap();
        assert_eq!(raw_lines.len(), 3);
        
        // Test filtering
        let indices = SessionService::filter_messages(&raw_lines, "User");
        assert_eq!(indices, vec![0]);
        
        // Test caching - second load should use cache
        let messages2 = session_service.load_session(&files[0]).unwrap();
        assert_eq!(messages2.len(), messages.len());
    }
    
    #[test]
    fn test_cache_service_integration() {
        let (_dir, files) = create_test_session_files();
        let mut cache = CacheService::new();
        
        // First access loads from file
        let cached1 = cache.get_messages(&std::path::Path::new(&files[0])).unwrap();
        assert_eq!(cached1.messages.len(), 3);
        assert_eq!(cached1.raw_lines.len(), 3);
        
        // Second access uses cache
        let cached2 = cache.get_messages(&std::path::Path::new(&files[0])).unwrap();
        assert_eq!(cached2.messages.len(), cached1.messages.len());
        
        // Test put/get
        cache.put("/test/path.jsonl".to_string(), vec!["line1".to_string(), "line2".to_string()]);
        let result = cache.get("/test/path.jsonl");
        assert!(result.is_some());
        assert_eq!(result.unwrap().len(), 2);
        
        // Test clear
        cache.clear();
        let result = cache.get("/test/path.jsonl");
        assert!(result.is_none());
    }
    
    #[test]
    fn test_search_filter_integration() {
        let results = vec![
            SearchResult {
                file: "file1.jsonl".to_string(),
                uuid: "1".to_string(),
                timestamp: "1700000000".to_string(),
                session_id: "abc".to_string(),
                role: "user".to_string(),
                text: "User message".to_string(),
                project_path: "/project".to_string(),
                raw_json: None,
            },
            SearchResult {
                file: "file1.jsonl".to_string(),
                uuid: "2".to_string(),
                timestamp: "1700000001".to_string(),
                session_id: "abc".to_string(),
                role: "assistant".to_string(),
                text: "Assistant message".to_string(),
                project_path: "/project".to_string(),
                raw_json: None,
            },
            SearchResult {
                file: "file1.jsonl".to_string(),
                uuid: "3".to_string(),
                timestamp: "1700000002".to_string(),
                session_id: "abc".to_string(),
                role: "system".to_string(),
                text: "System message".to_string(),
                project_path: "/project".to_string(),
                raw_json: None,
            },
        ];
        
        // Test role filter
        let filter = SearchFilter::new(Some("user".to_string()));
        let mut filtered = results.clone();
        filter.apply(&mut filtered).unwrap();
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].role, "user");
        
        // Test no filter
        let filter = SearchFilter::new(None);
        let mut filtered = results.clone();
        filter.apply(&mut filtered).unwrap();
        assert_eq!(filtered.len(), 3);
    }
    
    #[test]
    fn test_session_filter_integration() {
        let items = vec![
            SessionListItem {
                index: 0,
                role: "user".to_string(),
                timestamp: "2023-11-20T10:00:00Z".to_string(),
                text: "Hello world".to_string(),
            },
            SessionListItem {
                index: 1,
                role: "assistant".to_string(),
                timestamp: "2023-11-20T10:01:00Z".to_string(),
                text: "Hi there".to_string(),
            },
            SessionListItem {
                index: 2,
                role: "system".to_string(),
                timestamp: "2023-11-20T10:02:00Z".to_string(),
                text: "System info".to_string(),
            },
        ];
        
        // Test text search
        let indices = SessionFilter::filter_messages(&items, "Hello");
        assert_eq!(indices, vec![0]);
        
        // Test role search
        let indices = SessionFilter::filter_messages(&items, "assistant");
        assert_eq!(indices, vec![1]);
        
        // Test case-insensitive search
        let indices = SessionFilter::filter_messages(&items, "WORLD");
        assert_eq!(indices, vec![0]);
        
        // Test partial match
        let indices = SessionFilter::filter_messages(&items, "i");
        assert_eq!(indices, vec![1, 2]); // "Hi" and "info"
    }
    
    #[test]
    fn test_full_workflow_integration() {
        let (_dir, files) = create_test_session_files();
        let cache = Arc::new(Mutex::new(CacheService::new()));
        let search_service = Arc::new(SearchService::new(vec![files[0].clone()], false).unwrap());
        let session_service = Arc::new(SessionService::new(cache.clone()));
        
        // Step 1: Search for messages
        let search_request = SearchRequest {
            id: 1,
            query: "message".to_string(),
            pattern: files[0].clone(),
            role_filter: None,
        };
        
        let search_response = search_service.search(search_request).unwrap();
        assert!(search_response.results.len() > 0);
        
        // Step 2: Get first result and load its session
        let first_result = &search_response.results[0];
        let session_messages = session_service.load_session(&first_result.file).unwrap();
        assert!(session_messages.len() >= 1);
        
        // Step 3: Filter messages in session
        let raw_lines = session_service.get_raw_lines(&first_result.file).unwrap();
        let filtered_indices = SessionService::filter_messages(&raw_lines, "User");
        assert!(filtered_indices.len() >= 1);
        
        // Step 4: Verify cache is working
        let cache_guard = cache.lock().unwrap();
        let cached = cache_guard.get(&first_result.file);
        assert!(cached.is_some());
    }
    
    #[test]
    fn test_concurrent_service_access() {
        let (_dir, files) = create_test_session_files();
        let cache = Arc::new(Mutex::new(CacheService::new()));
        let search_service = Arc::new(SearchService::new(files.clone(), false).unwrap());
        let session_service = Arc::new(SessionService::new(cache.clone()));
        
        // Test concurrent access from multiple threads
        let mut handles = vec![];
        
        // Thread 1: Search operations
        let search_service_clone = search_service.clone();
        let file = files[0].clone();
        let handle1 = std::thread::spawn(move || {
            for i in 0..5 {
                let request = SearchRequest {
                    id: i,
                    query: format!("message {}", i),
                    pattern: file.clone(),
                    role_filter: None,
                };
                let result = search_service_clone.search(request);
                assert!(result.is_ok());
            }
        });
        handles.push(handle1);
        
        // Thread 2: Session loading
        let session_service_clone = session_service.clone();
        let file = files[0].clone();
        let handle2 = std::thread::spawn(move || {
            for _ in 0..5 {
                let result = session_service_clone.load_session(&file);
                assert!(result.is_ok());
            }
        });
        handles.push(handle2);
        
        // Thread 3: Cache operations
        let cache_clone = cache.clone();
        let handle3 = std::thread::spawn(move || {
            for i in 0..5 {
                let mut cache_guard = cache_clone.lock().unwrap();
                cache_guard.put(
                    format!("/test/path{}.jsonl", i),
                    vec![format!("line {}", i)],
                );
            }
        });
        handles.push(handle3);
        
        // Wait for all threads to complete
        for handle in handles {
            handle.join().unwrap();
        }
        
        // Verify final state
        let cache_guard = cache.lock().unwrap();
        assert!(cache_guard.get("/test/path0.jsonl").is_some());
    }
}