#[cfg(test)]
mod performance_benchmarks {
    use std::time::Instant;
    use crate::query::condition::{QueryCondition, SearchResult};
    use crate::interactive_ratatui::tuirealm_v3::app::App;
    use crate::interactive_ratatui::tuirealm_v3::messages::AppMessage;
    use crate::interactive_ratatui::tuirealm_v3::AppMode;
    use tuirealm::Update;
    
    fn create_large_dataset(size: usize) -> Vec<SearchResult> {
        (0..size)
            .map(|i| SearchResult {
                file: format!("file_{}.jsonl", i / 100),
                uuid: format!("uuid-{}", i),
                timestamp: format!("2024-01-01T10:{:02}:{:02}Z", (i / 60) % 60, i % 60),
                session_id: format!("session-{}", i % 10),
                role: match i % 3 {
                    0 => "User",
                    1 => "Assistant",
                    _ => "System",
                }.to_string(),
                text: format!("This is message {} containing some search text about {}", 
                    i,
                    match i % 5 {
                        0 => "programming",
                        1 => "debugging",
                        2 => "testing",
                        3 => "documentation",
                        _ => "deployment",
                    }
                ),
                has_tools: i % 7 == 0,
                has_thinking: i % 5 == 0,
                message_type: if i % 10 == 0 { "thinking" } else { "message" }.to_string(),
                query: QueryCondition::Literal {
                    pattern: "search".to_string(),
                    case_sensitive: false,
                },
                project_path: "/test/project".to_string(),
                raw_json: Some(format!(
                    r#"{{"content": "Message {}", "metadata": {{"tools": {}, "thinking": {}}}}}"#,
                    i, i % 7 == 0, i % 5 == 0
                )),
            })
            .collect()
    }
    
    #[test]
    #[ignore] // Run with: cargo test performance_benchmarks -- --ignored --nocapture
    fn benchmark_search_results_loading() {
        println!("\n=== Search Results Loading Performance ===");
        
        let test_sizes = vec![100, 500, 1000, 5000, 10000];
        
        for size in test_sizes {
            let results = create_large_dataset(size);
            let mut app = App::new(None, None, None, None);
            
            let start = Instant::now();
            app.state.search_results = results;
            app.update(Some(AppMessage::SearchCompleted));
            let duration = start.elapsed();
            
            println!("Loading {} results: {:?}", size, duration);
        }
    }
    
    #[test]
    #[ignore]
    fn benchmark_navigation_performance() {
        println!("\n=== Navigation Performance ===");
        
        let results = create_large_dataset(10000);
        let mut app = App::new(None, None, None, None);
        app.state.search_results = results;
        
        // Benchmark scrolling down
        let start = Instant::now();
        for _ in 0..1000 {
            app.update(Some(AppMessage::ResultDown));
        }
        let down_duration = start.elapsed();
        println!("1000x ResultDown: {:?} ({:.2} μs/op)", 
            down_duration, 
            down_duration.as_micros() as f64 / 1000.0
        );
        
        // Benchmark page down
        let start = Instant::now();
        for _ in 0..100 {
            app.update(Some(AppMessage::ResultPageDown));
        }
        let page_duration = start.elapsed();
        println!("100x ResultPageDown: {:?} ({:.2} μs/op)", 
            page_duration,
            page_duration.as_micros() as f64 / 100.0
        );
        
        // Benchmark jumping to end
        let start = Instant::now();
        app.update(Some(AppMessage::ResultEnd));
        let end_duration = start.elapsed();
        println!("Jump to end: {:?}", end_duration);
        
        // Benchmark jumping to home
        let start = Instant::now();
        app.update(Some(AppMessage::ResultHome));
        let home_duration = start.elapsed();
        println!("Jump to home: {:?}", home_duration);
    }
    
    #[test]
    #[ignore]
    fn benchmark_filtering_performance() {
        println!("\n=== Filtering Performance ===");
        
        let results = create_large_dataset(10000);
        let mut app = App::new(None, None, None, None);
        app.state.search_results = results;
        
        let filters = vec!["User", "Assistant", "System"];
        
        for filter in filters {
            // Set role filter
            app.state.role_filter = Some(filter.to_string());
            
            let start = Instant::now();
            // In real implementation, filtering happens in UI layer
            let filtered_count = app.state.search_results.iter()
                .filter(|r| r.role == filter)
                .count();
            let duration = start.elapsed();
            
            println!("Filter by '{}': {} results in {:?}", filter, filtered_count, duration);
        }
    }
    
    #[test]
    #[ignore]
    fn benchmark_mode_transitions() {
        println!("\n=== Mode Transition Performance ===");
        
        let results = create_large_dataset(1000);
        let mut app = App::new(None, None, None, None);
        app.state.search_results = results;
        
        // Benchmark entering result detail
        let start = Instant::now();
        for i in 0..100 {
            app.state.selected_index = i;
            app.update(Some(AppMessage::EnterResultDetail(i)));
            app.update(Some(AppMessage::ExitResultDetail));
        }
        let detail_duration = start.elapsed();
        println!("100x Enter/Exit ResultDetail: {:?} ({:.2} μs/cycle)", 
            detail_duration,
            detail_duration.as_micros() as f64 / 100.0
        );
        
        // Benchmark help mode transitions
        let start = Instant::now();
        for _ in 0..100 {
            app.update(Some(AppMessage::ShowHelp));
            app.update(Some(AppMessage::ExitHelp));
        }
        let help_duration = start.elapsed();
        println!("100x Show/Exit Help: {:?} ({:.2} μs/cycle)", 
            help_duration,
            help_duration.as_micros() as f64 / 100.0
        );
    }
    
    #[test]
    #[ignore]
    fn benchmark_session_viewer_performance() {
        println!("\n=== Session Viewer Performance ===");
        
        let mut app = App::new(None, None, None, None);
        
        // Create large session
        let session_messages: Vec<String> = (0..5000)
            .map(|i| format!(
                r#"{{"timestamp": "2024-01-01T10:{:02}:{:02}Z", "role": "{}", "content": "Message {}: This is a longer message to simulate realistic content with search terms and various keywords"}}"#,
                (i / 60) % 60, i % 60,
                match i % 3 {
                    0 => "User",
                    1 => "Assistant",
                    _ => "System",
                },
                i
            ))
            .collect();
        
        app.state.session_messages = session_messages;
        app.state.session_filtered_indices = (0..5000).collect();
        app.state.mode = AppMode::SessionViewer;
        
        // Benchmark session navigation
        let start = Instant::now();
        for _ in 0..1000 {
            app.update(Some(AppMessage::SessionScrollDown));
        }
        let nav_duration = start.elapsed();
        println!("1000x SessionScrollDown: {:?} ({:.2} μs/op)", 
            nav_duration,
            nav_duration.as_micros() as f64 / 1000.0
        );
        
        // Benchmark session search
        app.update(Some(AppMessage::SessionSearchStart));
        let start = Instant::now();
        app.update(Some(AppMessage::SessionQueryChanged("Message 250".to_string())));
        let search_duration = start.elapsed();
        println!("Session search filter: {:?}", search_duration);
        println!("Filtered results: {}", app.state.session_filtered_indices.len());
    }
    
    #[test]
    #[ignore]
    fn benchmark_memory_usage() {
        println!("\n=== Memory Usage Estimation ===");
        
        let test_sizes = vec![1000, 5000, 10000, 50000];
        
        for size in test_sizes {
            let results = create_large_dataset(size);
            let result_size = std::mem::size_of::<SearchResult>();
            let vec_overhead = std::mem::size_of::<Vec<SearchResult>>();
            
            // Estimate memory usage
            let estimated_memory = result_size * size + vec_overhead;
            let actual_vec_capacity = results.capacity();
            let actual_memory = result_size * actual_vec_capacity + vec_overhead;
            
            println!("Dataset size {}: estimated {} KB, actual capacity {} KB",
                size,
                estimated_memory / 1024,
                actual_memory / 1024
            );
        }
    }
    
    #[test]
    #[ignore]
    fn benchmark_worst_case_scenarios() {
        println!("\n=== Worst Case Scenarios ===");
        
        // Test with very long messages
        let mut app = App::new(None, None, None, None);
        let long_message_results: Vec<SearchResult> = (0..100)
            .map(|_i| {
                let mut result = create_large_dataset(1)[0].clone();
                result.text = "x".repeat(10000); // 10KB per message
                result.raw_json = Some(format!(r#"{{"content": "{}"}}"#, "y".repeat(50000))); // 50KB JSON
                result
            })
            .collect();
        
        let start = Instant::now();
        app.state.search_results = long_message_results;
        app.update(Some(AppMessage::SearchCompleted));
        let duration = start.elapsed();
        println!("Loading 100 large messages (60KB each): {:?}", duration);
        
        // Test rapid mode switching
        let start = Instant::now();
        for _ in 0..1000 {
            app.update(Some(AppMessage::ShowHelp));
            app.update(Some(AppMessage::ExitHelp));
        }
        let rapid_duration = start.elapsed();
        println!("1000x rapid mode switches: {:?} ({:.2} μs/switch)", 
            rapid_duration,
            rapid_duration.as_micros() as f64 / 1000.0
        );
    }
    
    fn print_comparison_summary() {
        println!("\n=== Performance Comparison Summary ===");
        println!("tui-realm v3 implementation characteristics:");
        println!("- Message passing architecture adds small overhead per operation");
        println!("- Component-based design improves code organization");
        println!("- AttrValue string serialization adds parsing overhead");
        println!("- Event handling is more structured but slightly slower");
        println!("\nRecommended optimizations:");
        println!("1. Cache parsed AttrValue data to avoid repeated parsing");
        println!("2. Use batch updates for multiple state changes");
        println!("3. Implement virtual scrolling for large datasets");
        println!("4. Consider lazy loading for session messages");
    }
    
    #[test]
    #[ignore]
    fn run_all_benchmarks() {
        benchmark_search_results_loading();
        benchmark_navigation_performance();
        benchmark_filtering_performance();
        benchmark_mode_transitions();
        benchmark_session_viewer_performance();
        benchmark_memory_usage();
        benchmark_worst_case_scenarios();
        print_comparison_summary();
    }
}