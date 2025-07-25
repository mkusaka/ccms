#[cfg(test)]
mod workflow_tests {
    use crate::interactive_iocraft::application::{SearchService, SessionService, CacheService};
    use crate::interactive_iocraft::domain::models::{SearchRequest, Mode, SessionOrder};
    use crate::interactive_iocraft::domain::filter::SearchFilter;
    use crate::query::condition::SearchResult;
    use std::sync::{Arc, Mutex};
    use std::fs;
    use std::io::Write;
    use tempfile::TempDir;
    
    /// Helper to create a realistic test environment
    fn create_realistic_environment() -> (TempDir, Arc<SearchService>, Arc<SessionService>, Arc<Mutex<CacheService>>, Vec<String>) {
        let dir = TempDir::new().unwrap();
        let mut file_paths = Vec::new();
        
        // Create realistic session files
        // Session 1: Debugging session
        let file1 = dir.path().join("claude_session_debug.jsonl");
        let mut f1 = fs::File::create(&file1).unwrap();
        writeln!(f1, r#"{{"uuid":"d1","timestamp":"1700000000","sessionId":"debug123","role":"user","text":"I'm getting a segmentation fault in my Rust program","projectPath":"/projects/rust-app"}}"#).unwrap();
        writeln!(f1, r#"{{"uuid":"d2","timestamp":"1700000010","sessionId":"debug123","role":"assistant","text":"Let me help you debug the segmentation fault. Can you share the code that's causing the issue?","projectPath":"/projects/rust-app"}}"#).unwrap();
        writeln!(f1, r#"{{"uuid":"d3","timestamp":"1700000020","sessionId":"debug123","role":"user","text":"Here's the code: unsafe {{ ptr::write(null_mut(), 42); }}","projectPath":"/projects/rust-app"}}"#).unwrap();
        writeln!(f1, r#"{{"uuid":"d4","timestamp":"1700000030","sessionId":"debug123","role":"assistant","text":"The issue is that you're writing to a null pointer. This is undefined behavior and causes a segmentation fault.","projectPath":"/projects/rust-app"}}"#).unwrap();
        writeln!(f1, r#"{{"uuid":"d5","timestamp":"1700000040","sessionId":"debug123","role":"system","text":"Code analysis complete","projectPath":"/projects/rust-app"}}"#).unwrap();
        f1.flush().unwrap();
        file_paths.push(file1.to_string_lossy().to_string());
        
        // Session 2: Learning session
        let file2 = dir.path().join("claude_session_learning.jsonl");
        let mut f2 = fs::File::create(&file2).unwrap();
        writeln!(f2, r#"{{"uuid":"l1","timestamp":"1700001000","sessionId":"learn456","role":"user","text":"Can you explain async/await in Rust?","projectPath":"/tutorials/rust-async"}}"#).unwrap();
        writeln!(f2, r#"{{"uuid":"l2","timestamp":"1700001010","sessionId":"learn456","role":"assistant","text":"Async/await in Rust is a way to write asynchronous code that looks synchronous. Here's how it works...","projectPath":"/tutorials/rust-async"}}"#).unwrap();
        writeln!(f2, r#"{{"uuid":"l3","timestamp":"1700001020","sessionId":"learn456","role":"user","text":"How do I create an async function?","projectPath":"/tutorials/rust-async"}}"#).unwrap();
        writeln!(f2, r#"{{"uuid":"l4","timestamp":"1700001030","sessionId":"learn456","role":"assistant","text":"To create an async function, use the async keyword: async fn my_function() -> Result<T, E> {{ ... }}","projectPath":"/tutorials/rust-async"}}"#).unwrap();
        writeln!(f2, r#"{{"uuid":"l5","timestamp":"1700001040","sessionId":"learn456","role":"system","text":"Tutorial session saved","projectPath":"/tutorials/rust-async"}}"#).unwrap();
        f2.flush().unwrap();
        file_paths.push(file2.to_string_lossy().to_string());
        
        // Session 3: Code review session
        let file3 = dir.path().join("claude_session_review.jsonl");
        let mut f3 = fs::File::create(&file3).unwrap();
        writeln!(f3, r#"{{"uuid":"r1","timestamp":"1700002000","sessionId":"review789","role":"user","text":"Please review this code for potential improvements","projectPath":"/projects/web-server"}}"#).unwrap();
        writeln!(f3, r#"{{"uuid":"r2","timestamp":"1700002010","sessionId":"review789","role":"assistant","text":"I'll review your code. I notice several areas for improvement: 1) Error handling could be more robust...","projectPath":"/projects/web-server"}}"#).unwrap();
        writeln!(f3, r#"{{"uuid":"r3","timestamp":"1700002020","sessionId":"review789","role":"user","text":"What about performance optimizations?","projectPath":"/projects/web-server"}}"#).unwrap();
        writeln!(f3, r#"{{"uuid":"r4","timestamp":"1700002030","sessionId":"review789","role":"assistant","text":"For performance, consider: 1) Using connection pooling, 2) Implementing caching, 3) Optimizing database queries","projectPath":"/projects/web-server"}}"#).unwrap();
        writeln!(f3, r#"{{"uuid":"r5","timestamp":"1700002040","sessionId":"review789","role":"system","text":"Review completed","projectPath":"/projects/web-server"}}"#).unwrap();
        f3.flush().unwrap();
        file_paths.push(file3.to_string_lossy().to_string());
        
        let cache = Arc::new(Mutex::new(CacheService::new()));
        let search_service = Arc::new(SearchService::new(file_paths.clone(), false).unwrap());
        let session_service = Arc::new(SessionService::new(cache.clone()));
        
        (dir, search_service, session_service, cache, file_paths)
    }
    
    #[test]
    fn test_workflow_debug_issue() {
        let (_dir, search_service, session_service, _cache, file_paths) = create_realistic_environment();
        
        // Workflow: User wants to find their debugging session about segmentation fault
        
        // Step 1: Search for "segmentation fault"
        let request = SearchRequest {
            id: 1,
            query: "segmentation fault".to_string(),
            pattern: file_paths.join(","),
            role_filter: None,
        };
        
        let response = search_service.search(request).unwrap();
        assert_eq!(response.results.len(), 2); // User question and assistant response
        
        // Step 2: Select the first result (user's question)
        let selected = &response.results[0];
        assert_eq!(selected.role, "user");
        assert!(selected.text.contains("segmentation fault"));
        
        // Step 3: View the full session
        let messages = session_service.load_session(&selected.file).unwrap();
        assert_eq!(messages.len(), 5);
        
        // Step 4: Verify the solution was provided
        let assistant_messages: Vec<_> = messages.iter()
            .filter(|m| m.get_role() == Some("assistant"))
            .collect();
        assert_eq!(assistant_messages.len(), 2);
        assert!(assistant_messages[1].get_text().unwrap().contains("null pointer"));
    }
    
    #[test]
    fn test_workflow_learning_path() {
        let (_dir, search_service, session_service, _cache, file_paths) = create_realistic_environment();
        
        // Workflow: User wants to review their async/await learning session
        
        // Step 1: Search for async-related content
        let request = SearchRequest {
            id: 1,
            query: "async".to_string(),
            pattern: file_paths.join(","),
            role_filter: None,
        };
        
        let response = search_service.search(request).unwrap();
        assert!(response.results.len() >= 3); // Multiple messages about async
        
        // Step 2: Filter by user role to see questions asked
        let mut user_results = response.results.clone();
        let filter = SearchFilter::new(Some("user".to_string()));
        filter.apply(&mut user_results).unwrap();
        assert!(user_results.len() >= 1);
        
        // Step 3: Load the learning session
        let learning_session = &user_results[0];
        let raw_lines = session_service.get_raw_lines(&learning_session.file).unwrap();
        
        // Step 4: Search within session for "function"
        let function_indices = SessionService::filter_messages(&raw_lines, "function");
        assert!(!function_indices.is_empty());
        
        // Verify the answer about async functions was provided
        assert!(raw_lines[function_indices[0]].contains("async function"));
    }
    
    #[test]
    fn test_workflow_code_review_history() {
        let (_dir, search_service, session_service, _cache, file_paths) = create_realistic_environment();
        
        // Workflow: User wants to find performance optimization suggestions
        
        // Step 1: Search for performance-related content
        let request = SearchRequest {
            id: 1,
            query: "performance OR optimization".to_string(),
            pattern: file_paths.join(","),
            role_filter: None,
        };
        
        let response = search_service.search(request).unwrap();
        assert!(!response.results.is_empty());
        
        // Step 2: Filter to assistant responses only
        let mut assistant_results = response.results.clone();
        let filter = SearchFilter::new(Some("assistant".to_string()));
        filter.apply(&mut assistant_results).unwrap();
        
        // Step 3: Find the specific optimization advice
        let optimization_advice = assistant_results.iter()
            .find(|r| r.text.contains("connection pooling"))
            .expect("Should find optimization advice");
        
        // Step 4: View the context of this advice
        let messages = session_service.load_session(&optimization_advice.file).unwrap();
        
        // Verify this was part of a code review
        let first_message = &messages[0];
        assert!(first_message.get_text().unwrap().contains("review"));
    }
    
    #[test]
    fn test_workflow_cross_session_search() {
        let (_dir, search_service, _session_service, _cache, file_paths) = create_realistic_environment();
        
        // Workflow: User wants to find all assistant responses about Rust
        
        // Step 1: Search for "Rust" with assistant filter
        let request = SearchRequest {
            id: 1,
            query: "Rust".to_string(),
            pattern: file_paths.join(","),
            role_filter: Some("assistant".to_string()),
        };
        
        let response = search_service.search(request).unwrap();
        assert!(response.results.len() >= 2); // At least debug and learning sessions
        
        // Step 2: Group results by session
        let mut sessions = std::collections::HashMap::new();
        for result in &response.results {
            sessions.entry(&result.session_id).or_insert_with(Vec::new).push(result);
        }
        
        // Should have found multiple sessions
        assert!(sessions.len() >= 2);
        
        // Step 3: Verify different topics covered
        let topics_covered: Vec<String> = response.results.iter()
            .map(|r| {
                if r.text.contains("segmentation") {
                    "debugging".to_string()
                } else if r.text.contains("async") {
                    "async".to_string()
                } else {
                    "other".to_string()
                }
            })
            .collect();
        
        assert!(topics_covered.contains(&"debugging".to_string()));
        assert!(topics_covered.contains(&"async".to_string()));
    }
    
    #[test]
    fn test_workflow_time_based_review() {
        let (_dir, search_service, session_service, _cache, file_paths) = create_realistic_environment();
        
        // Workflow: User wants to review sessions in chronological order
        
        // Step 1: Get all messages
        let request = SearchRequest {
            id: 1,
            query: "".to_string(), // Empty query matches all
            pattern: file_paths.join(","),
            role_filter: None,
        };
        
        let response = search_service.search(request).unwrap();
        
        // Results should be sorted by timestamp descending (newest first)
        let timestamps: Vec<_> = response.results.iter()
            .map(|r| &r.timestamp)
            .collect();
        
        for i in 1..timestamps.len() {
            assert!(timestamps[i-1] >= timestamps[i], "Results should be sorted descending");
        }
        
        // Step 2: Find the most recent session
        let most_recent = &response.results[0];
        let most_recent_session_id = &most_recent.session_id;
        
        // Step 3: Load and verify it's the code review session
        let messages = session_service.load_session(&most_recent.file).unwrap();
        assert!(messages[0].get_text().unwrap().contains("review"));
        
        // Step 4: Sort messages in ascending order to see conversation flow
        let mut messages_clone = messages.clone();
        SessionService::sort_messages(&mut messages_clone, SessionOrder::Ascending);
        
        // Verify conversation starts with user request and ends with system message
        assert_eq!(messages_clone[0].get_role(), Some("user"));
        assert_eq!(messages_clone[messages_clone.len()-1].get_role(), Some("system"));
    }
    
    #[test]
    fn test_workflow_project_based_search() {
        let (_dir, search_service, _session_service, _cache, file_paths) = create_realistic_environment();
        
        // Workflow: User wants to find all conversations about a specific project
        
        // Note: In real usage, we'd search by project path, but our test setup
        // uses the text field for demonstration
        
        // Search for web-server project discussions
        let request = SearchRequest {
            id: 1,
            query: "web-server".to_string(),
            pattern: file_paths.join(","),
            role_filter: None,
        };
        
        let response = search_service.search(request).unwrap();
        
        // All results should be from the code review session
        let session_ids: std::collections::HashSet<_> = response.results.iter()
            .map(|r| &r.session_id)
            .collect();
        assert_eq!(session_ids.len(), 1);
        
        // Verify all messages are about the web-server project
        for result in &response.results {
            assert_eq!(result.project_path, "/projects/web-server");
        }
    }
    
    #[test]
    fn test_workflow_error_recovery() {
        let (_dir, search_service, session_service, cache, file_paths) = create_realistic_environment();
        
        // Workflow: User makes mistakes and recovers
        
        // Step 1: Invalid search query
        let request = SearchRequest {
            id: 1,
            query: "AND AND".to_string(), // Invalid syntax
            pattern: file_paths[0].clone(),
            role_filter: None,
        };
        
        let result = search_service.search(request);
        assert!(result.is_err());
        
        // Step 2: Recover with valid query
        let request = SearchRequest {
            id: 2,
            query: "Rust".to_string(),
            pattern: file_paths[0].clone(),
            role_filter: None,
        };
        
        let response = search_service.search(request).unwrap();
        assert!(!response.results.is_empty());
        
        // Step 3: Try to load non-existent session
        let bad_result = session_service.load_session("/non/existent.jsonl");
        assert!(bad_result.is_err());
        
        // Step 4: Cache should still be functional
        let cache_guard = cache.lock().unwrap();
        // Cache operations should not be affected by previous errors
        drop(cache_guard); // Release lock
        
        // Step 5: Successfully load a real session
        let good_result = session_service.load_session(&file_paths[0]);
        assert!(good_result.is_ok());
    }
}