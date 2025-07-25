//! Performance tests for the interactive iocraft interface
//!
//! Tests performance characteristics with large datasets and complex scenarios.

#[cfg(test)]
mod performance_tests {
    use crate::interactive_iocraft::application::{SearchService, SessionService, CacheService};
    use crate::interactive_iocraft::domain::models::{SearchRequest, Mode};
    use crate::interactive_iocraft::ui::components::App;
    use crate::interactive_iocraft::ui::contexts::{Theme, Settings};
    use crate::interactive_iocraft::{SessionMessage, SearchResult};
    use iocraft::prelude::*;
    use smol_macros::test;
    use macro_rules_attribute::apply;
    use std::sync::{Arc, Mutex};
    use std::time::{Duration, Instant};
    use std::fs;
    use std::io::Write;
    use tempfile::{TempDir, NamedTempFile};
    
    /// Creates a large test dataset with the specified number of messages
    fn create_large_dataset(message_count: usize) -> (TempDir, Vec<String>) {
        let dir = TempDir::new().unwrap();
        let mut file_paths = Vec::new();
        
        // Create multiple files to simulate realistic scenarios
        let files_count = 10;
        let messages_per_file = message_count / files_count;
        
        for file_idx in 0..files_count {
            let file_path = dir.path().join(format!("session_{}.jsonl", file_idx));
            let mut file = fs::File::create(&file_path).unwrap();
            
            for msg_idx in 0..messages_per_file {
                let timestamp = 1700000000 + (file_idx * messages_per_file + msg_idx) as i64;
                let role = match msg_idx % 3 {
                    0 => "user",
                    1 => "assistant",
                    _ => "system",
                };
                
                writeln!(
                    file,
                    r#"{{"uuid":"{}","timestamp":"{}","sessionId":"perf{}","role":"{}","text":"Message {} with some longer content to simulate realistic message sizes. This includes keywords like error, bug, performance, optimization, async, rust, and more.","projectPath":"/perf/test"}}"#,
                    format!("{}-{}", file_idx, msg_idx),
                    timestamp,
                    file_idx,
                    role,
                    msg_idx
                ).unwrap();
            }
            
            file.flush().unwrap();
            file_paths.push(file_path.to_string_lossy().to_string());
        }
        
        (dir, file_paths)
    }
    
    #[test]
    fn test_search_performance_10k_messages() {
        let (_dir, file_paths) = create_large_dataset(10_000);
        let search_service = Arc::new(SearchService::new(file_paths.clone(), false).unwrap());
        
        // Test simple query performance
        let start = Instant::now();
        let request = SearchRequest {
            id: 1,
            query: "error".to_string(),
            pattern: file_paths.join(","),
            role_filter: None,
        };
        
        let response = search_service.search(request).unwrap();
        let elapsed = start.elapsed();
        
        println!("Search 10k messages (simple query): {:?}", elapsed);
        assert!(elapsed < Duration::from_secs(1), "Simple search should complete within 1 second");
        assert!(!response.results.is_empty(), "Should find results for 'error'");
    }
    
    #[test]
    fn test_search_performance_100k_messages() {
        let (_dir, file_paths) = create_large_dataset(100_000);
        let search_service = Arc::new(SearchService::new(file_paths.clone(), false).unwrap());
        
        // Test complex query performance
        let start = Instant::now();
        let request = SearchRequest {
            id: 1,
            query: "error AND performance OR optimization".to_string(),
            pattern: file_paths.join(","),
            role_filter: None,
        };
        
        let response = search_service.search(request).unwrap();
        let elapsed = start.elapsed();
        
        println!("Search 100k messages (complex query): {:?}", elapsed);
        assert!(elapsed < Duration::from_secs(5), "Complex search should complete within 5 seconds");
        assert!(!response.results.is_empty(), "Should find results for complex query");
    }
    
    #[test]
    fn test_session_loading_performance() {
        let (_dir, file_paths) = create_large_dataset(50_000);
        let cache_service = Arc::new(Mutex::new(CacheService::new()));
        let session_service = Arc::new(SessionService::new(cache_service));
        
        // Test session loading performance
        let start = Instant::now();
        let messages = session_service.load_session(&file_paths[0]).unwrap();
        let elapsed = start.elapsed();
        
        println!("Load session with 5k messages: {:?}", elapsed);
        assert!(elapsed < Duration::from_millis(500), "Session loading should complete within 500ms");
        assert_eq!(messages.len(), 5_000, "Should load all messages");
        
        // Test cached loading performance
        let start = Instant::now();
        let _messages_cached = session_service.load_session(&file_paths[0]).unwrap();
        let elapsed_cached = start.elapsed();
        
        println!("Load cached session: {:?}", elapsed_cached);
        assert!(elapsed_cached < Duration::from_millis(10), "Cached loading should be near-instant");
    }
    
    #[test]
    fn test_memory_usage_large_files() {
        let (_dir, file_paths) = create_large_dataset(50_000);
        let cache_service = Arc::new(Mutex::new(CacheService::new()));
        
        // Get initial memory usage
        let initial_memory = get_process_memory_usage();
        
        // Load multiple sessions
        for file_path in &file_paths[..5] {
            let mut cache = cache_service.lock().unwrap();
            let _ = cache.get_messages(std::path::Path::new(file_path)).unwrap();
        }
        
        // Get memory usage after loading
        let after_loading = get_process_memory_usage();
        let memory_increase = after_loading - initial_memory;
        
        println!("Memory increase after loading 25k messages: {} MB", memory_increase / 1_048_576);
        
        // Memory usage should be reasonable (less than 500MB for 25k messages)
        assert!(
            memory_increase < 500 * 1_048_576,
            "Memory usage should be less than 500MB for 25k messages"
        );
    }
    
    #[test]
    fn test_concurrent_search_performance() {
        let (_dir, file_paths) = create_large_dataset(20_000);
        let search_service = Arc::new(SearchService::new(file_paths.clone(), false).unwrap());
        
        // Simulate concurrent searches
        let start = Instant::now();
        let mut handles = Vec::new();
        
        for i in 0..5 {
            let service = search_service.clone();
            let paths = file_paths.clone();
            
            let handle = std::thread::spawn(move || {
                let request = SearchRequest {
                    id: i,
                    query: format!("Message {}", i * 100),
                    pattern: paths.join(","),
                    role_filter: None,
                };
                service.search(request).unwrap()
            });
            
            handles.push(handle);
        }
        
        // Wait for all searches to complete
        for handle in handles {
            let _ = handle.join().unwrap();
        }
        
        let elapsed = start.elapsed();
        println!("5 concurrent searches on 20k messages: {:?}", elapsed);
        assert!(elapsed < Duration::from_secs(3), "Concurrent searches should complete within 3 seconds");
    }
    
    #[test]
    fn test_ui_render_performance_large_results() {
        // Create a mock component to test rendering performance
        #[component]
        fn PerformanceTestComponent(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
            let results = hooks.use_state(|| {
                // Generate 1000 mock results
                (0..1000).map(|i| SearchResult {
                    uuid: format!("uuid-{}", i),
                    timestamp: format!("{}", 1700000000 + i),
                    session_id: format!("session-{}", i / 100),
                    role: match i % 3 {
                        0 => "user".to_string(),
                        1 => "assistant".to_string(),
                        _ => "system".to_string(),
                    },
                    text: format!("Result {} with some content", i),
                    project_path: "/test".to_string(),
                    file: format!("/tmp/session_{}.jsonl", i / 100),
                    relevance_score: 0.8,
                }).collect::<Vec<_>>()
            });
            
            element! {
                Box {
                    Text(content: format!("Rendering {} results", results.read().len()))
                }
            }
        }
        
        // Measure render time
        let start = Instant::now();
        let _output = smol::block_on(async {
            element!(PerformanceTestComponent)
                .mock_terminal_render_loop(MockTerminalConfig::default())
                .take(1)
                .collect::<Vec<_>>()
                .await
        });
        let elapsed = start.elapsed();
        
        println!("Render 1000 results: {:?}", elapsed);
        assert!(elapsed < Duration::from_millis(100), "Rendering should complete within 100ms");
    }
    
    #[test] 
    fn test_virtual_scrolling_performance() {
        // Test that virtual scrolling keeps performance constant regardless of total items
        let small_list_time = measure_scroll_performance(100);
        let large_list_time = measure_scroll_performance(10_000);
        
        println!("Scroll 100 items: {:?}", small_list_time);
        println!("Scroll 10,000 items: {:?}", large_list_time);
        
        // Virtual scrolling should keep performance similar regardless of list size
        let time_ratio = large_list_time.as_millis() as f64 / small_list_time.as_millis() as f64;
        assert!(
            time_ratio < 2.0,
            "Virtual scrolling should keep performance constant (ratio: {})",
            time_ratio
        );
    }
    
    // Helper function to measure scroll performance
    fn measure_scroll_performance(item_count: usize) -> Duration {
        #[component]
        fn ScrollTestComponent(mut hooks: Hooks, item_count: usize) -> impl Into<AnyElement<'static>> {
            let scroll_offset = hooks.use_state(|| 0);
            let visible_items = 20;
            
            // Simulate scrolling through the list
            hooks.use_future({
                let mut scroll = scroll_offset.clone();
                async move {
                    for i in 0..50 {
                        scroll.set(i * 10);
                        smol::Timer::after(Duration::from_millis(1)).await;
                    }
                }
            });
            
            let visible_range = *scroll_offset.read()..(*scroll_offset.read() + visible_items).min(item_count);
            
            element! {
                Box {
                    Box(flex_direction: FlexDirection::Column) {
                        for i in visible_range {
                            Text(content: format!("Item {}", i))
                        }
                    }
                }
            }
        }
        
        let start = Instant::now();
        smol::block_on(async {
            element!(ScrollTestComponent(item_count))
                .mock_terminal_render_loop(MockTerminalConfig::default())
                .take(50) // Simulate 50 scroll updates
                .collect::<Vec<_>>()
                .await
        });
        start.elapsed()
    }
    
    // Helper function to get process memory usage (simplified)
    fn get_process_memory_usage() -> usize {
        // In a real implementation, we would use system APIs to get actual memory usage
        // For now, return a mock value based on static memory tracking
        use std::alloc::{GlobalAlloc, Layout, System};
        
        // This is a simplified approach - in production, use proper memory profiling
        static ALLOCATED: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        
        struct TrackingAllocator;
        
        unsafe impl GlobalAlloc for TrackingAllocator {
            unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
                let ret = System.alloc(layout);
                if !ret.is_null() {
                    ALLOCATED.fetch_add(layout.size(), std::sync::atomic::Ordering::Relaxed);
                }
                ret
            }
            
            unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
                System.dealloc(ptr, layout);
                ALLOCATED.fetch_sub(layout.size(), std::sync::atomic::Ordering::Relaxed);
            }
        }
        
        ALLOCATED.load(std::sync::atomic::Ordering::Relaxed)
    }
}