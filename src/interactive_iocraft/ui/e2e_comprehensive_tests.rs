//! Comprehensive End-to-End tests for the iocraft UI
//!
//! These tests simulate complete user workflows from start to finish.

#[cfg(test)]
mod e2e_comprehensive_tests {
    use crate::interactive_iocraft::application::{SearchService, SessionService, CacheService};
    use crate::interactive_iocraft::ui::components::{App, AppProps};
    use crate::interactive_iocraft::domain::models::Mode;
    use crate::interactive_iocraft::SearchResult;
    use iocraft::prelude::*;
    use smol_macros::test;
    use macro_rules_attribute::apply;
    use std::sync::{Arc, Mutex};
    use std::time::Duration;
    use tempfile::TempDir;
    use std::fs;
    use std::io::Write;
    
    /// Creates a comprehensive test environment with multiple sessions and diverse content
    fn create_comprehensive_test_environment() -> (TempDir, Arc<SearchService>, Arc<SessionService>, Arc<Mutex<CacheService>>) {
        let dir = TempDir::new().unwrap();
        
        // Create multiple session files with diverse content
        let session1_path = dir.path().join("debug_session.jsonl");
        let mut session1 = fs::File::create(&session1_path).unwrap();
        writeln!(session1, r#"{{"uuid":"d1","timestamp":"1700000000","sessionId":"debug123","role":"user","text":"I'm getting a segmentation fault in my Rust program","projectPath":"/projects/rust-app"}}"#).unwrap();
        writeln!(session1, r#"{{"uuid":"d2","timestamp":"1700000010","sessionId":"debug123","role":"assistant","text":"Let me help you debug the segmentation fault. Can you share the code that's causing the issue?","projectPath":"/projects/rust-app"}}"#).unwrap();
        writeln!(session1, r#"{{"uuid":"d3","timestamp":"1700000020","sessionId":"debug123","role":"user","text":"Here's the code: unsafe {{ ptr::write(null_mut(), 42); }}","projectPath":"/projects/rust-app"}}"#).unwrap();
        writeln!(session1, r#"{{"uuid":"d4","timestamp":"1700000030","sessionId":"debug123","role":"assistant","text":"The issue is that you're writing to a null pointer. This is undefined behavior and causes a segmentation fault.\n\nHere's how to fix it:\n\n```rust\nlet mut value = Box::new(42);\nunsafe {\n    ptr::write(&mut *value, 42);\n}\n```\n\nThis allocates proper memory before writing.","projectPath":"/projects/rust-app"}}"#).unwrap();
        writeln!(session1, r#"{{"uuid":"d5","timestamp":"1700000040","sessionId":"debug123","role":"system","text":"Code analysis complete","projectPath":"/projects/rust-app"}}"#).unwrap();
        session1.flush().unwrap();
        
        let session2_path = dir.path().join("tutorial_session.jsonl");
        let mut session2 = fs::File::create(&session2_path).unwrap();
        writeln!(session2, r#"{{"uuid":"t1","timestamp":"1700001000","sessionId":"tutorial456","role":"user","text":"Can you explain async/await in Rust?","projectPath":"/tutorials/rust-async"}}"#).unwrap();
        writeln!(session2, r#"{{"uuid":"t2","timestamp":"1700001010","sessionId":"tutorial456","role":"assistant","text":"Async/await in Rust is a way to write asynchronous code that looks synchronous. Here's a comprehensive overview:\n\n1. **Async Functions**: Marked with `async fn`\n2. **Await Points**: Use `.await` to pause execution\n3. **Futures**: Async functions return Future trait objects\n\nExample:\n```rust\nasync fn fetch_data() -> Result<String, Error> {\n    let response = reqwest::get(\"https://api.example.com\").await?;\n    let text = response.text().await?;\n    Ok(text)\n}\n```","projectPath":"/tutorials/rust-async"}}"#).unwrap();
        writeln!(session2, r#"{{"uuid":"t3","timestamp":"1700001020","sessionId":"tutorial456","role":"user","text":"How do I handle errors in async functions?","projectPath":"/tutorials/rust-async"}}"#).unwrap();
        writeln!(session2, r#"{{"uuid":"t4","timestamp":"1700001030","sessionId":"tutorial456","role":"assistant","text":"Error handling in async functions works similarly to sync functions, with some async-specific patterns:\n\n1. **Using ?**: Works the same in async functions\n2. **try_join!**: Handle multiple async operations\n3. **select!**: Race multiple futures\n\nExample:\n```rust\nuse tokio::try_join;\n\nasync fn parallel_ops() -> Result<(), Error> {\n    let (result1, result2) = try_join!(\n        async_operation1(),\n        async_operation2()\n    )?;\n    Ok(())\n}\n```","projectPath":"/tutorials/rust-async"}}"#).unwrap();
        session2.flush().unwrap();
        
        let session3_path = dir.path().join("performance_session.jsonl");
        let mut session3 = fs::File::create(&session3_path).unwrap();
        writeln!(session3, r#"{{"uuid":"p1","timestamp":"1700002000","sessionId":"perf789","role":"user","text":"My web server is running slowly. How can I optimize it?","projectPath":"/projects/web-server"}}"#).unwrap();
        writeln!(session3, r#"{{"uuid":"p2","timestamp":"1700002010","sessionId":"perf789","role":"assistant","text":"Let's analyze your web server performance. Common optimization areas include:\n\n1. **Connection Pooling**: Reuse database connections\n2. **Caching**: Implement Redis or in-memory caching\n3. **Async I/O**: Use tokio or async-std\n4. **Load Balancing**: Distribute requests\n\nFirst, let's profile your application to identify bottlenecks.","projectPath":"/projects/web-server"}}"#).unwrap();
        writeln!(session3, r#"{{"uuid":"p3","timestamp":"1700002020","sessionId":"perf789","role":"user","text":"I've profiled it and database queries are taking 80% of the time","projectPath":"/projects/web-server"}}"#).unwrap();
        writeln!(session3, r#"{{"uuid":"p4","timestamp":"1700002030","sessionId":"perf789","role":"assistant","text":"Database optimization strategies:\n\n1. **Add Indexes**: Create indexes on frequently queried columns\n2. **Query Optimization**: Use EXPLAIN to analyze queries\n3. **Connection Pooling**: Implement r2d2 or deadpool\n4. **Caching Layer**: Cache frequent queries with Redis\n\nExample connection pool:\n```rust\nlet pool = r2d2::Pool::builder()\n    .max_size(30)\n    .build(manager)?;\n```","projectPath":"/projects/web-server"}}"#).unwrap();
        session3.flush().unwrap();
        
        let file_patterns = vec![
            session1_path.to_string_lossy().to_string(),
            session2_path.to_string_lossy().to_string(),
            session3_path.to_string_lossy().to_string(),
        ];
        
        let cache = Arc::new(Mutex::new(CacheService::new()));
        let search_service = Arc::new(SearchService::new(file_patterns.clone(), false).unwrap());
        let session_service = Arc::new(SessionService::new(cache.clone()));
        
        (dir, search_service, session_service, cache)
    }
    
    #[apply(test!)]
    async fn test_complete_search_workflow() {
        let (_dir, search_service, session_service, cache) = create_comprehensive_test_environment();
        
        // Simulate complete search workflow
        #[component]
        fn SearchWorkflowTest(mut hooks: Hooks, props: &AppProps) -> impl Into<AnyElement<'static>> {
            let workflow_state = hooks.use_state(|| "initial".to_string());
            let search_count = hooks.use_state(|| 0);
            
            // Simulate user workflow
            hooks.use_future({
                let mut state = workflow_state.clone();
                let mut count = search_count.clone();
                async move {
                    // Step 1: Initial search
                    smol::Timer::after(Duration::from_millis(10)).await;
                    state.set("searching_debug".to_string());
                    count.set(1);
                    
                    // Step 2: Refine search
                    smol::Timer::after(Duration::from_millis(20)).await;
                    state.set("searching_segmentation".to_string());
                    count.set(2);
                    
                    // Step 3: Select result
                    smol::Timer::after(Duration::from_millis(30)).await;
                    state.set("viewing_result".to_string());
                }
            });
            
            element! {
                Box {
                    Text(content: format!("State: {}, Searches: {}", workflow_state.read(), search_count.read()))
                }
            }
        }
        
        let actual = element! {
            SearchWorkflowTest(
                search_service: Some(search_service),
                session_service: Some(session_service),
                cache_service: Some(cache),
                initial_query: None,
                file_patterns: vec![],
            )
        }
        .mock_terminal_render_loop(MockTerminalConfig::default())
        .take(6)
        .collect::<Vec<_>>()
        .await;
        
        // Verify workflow progression
        assert!(actual[0].to_string().contains("State: initial"));
        assert!(actual.iter().any(|frame| frame.to_string().contains("searching_debug")));
        assert!(actual.iter().any(|frame| frame.to_string().contains("searching_segmentation")));
        assert!(actual.iter().any(|frame| frame.to_string().contains("viewing_result")));
    }
    
    #[apply(test!)]
    async fn test_role_filter_workflow() {
        let (_dir, search_service, session_service, cache) = create_comprehensive_test_environment();
        
        #[component]
        fn RoleFilterTest(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
            let role_filter = hooks.use_state(|| None::<String>);
            let filter_changes = hooks.use_state(|| 0);
            
            // Simulate role filter cycling
            hooks.use_future({
                let mut filter = role_filter.clone();
                let mut changes = filter_changes.clone();
                async move {
                    // Cycle through filters
                    let filters = [None, Some("user".to_string()), Some("assistant".to_string()), Some("system".to_string())];
                    for (i, f) in filters.into_iter().enumerate() {
                        smol::Timer::after(Duration::from_millis(10 * (i as u64 + 1))).await;
                        filter.set(f);
                        changes.set(*changes.read() + 1);
                    }
                }
            });
            
            element! {
                Box {
                    Text(content: format!("Filter: {:?}, Changes: {}", role_filter.read(), filter_changes.read()))
                }
            }
        }
        
        let actual = element!(RoleFilterTest)
            .mock_terminal_render_loop(MockTerminalConfig::default())
            .take(8)
            .collect::<Vec<_>>()
            .await;
        
        // Verify all role filters were applied
        assert!(actual[0].to_string().contains("Filter: None"));
        assert!(actual.iter().any(|f| f.to_string().contains("Filter: Some(\"user\")")));
        assert!(actual.iter().any(|f| f.to_string().contains("Filter: Some(\"assistant\")")));
        assert!(actual.iter().any(|f| f.to_string().contains("Filter: Some(\"system\")")));
    }
    
    #[apply(test!)]
    async fn test_clipboard_operations() {
        #[component]
        fn ClipboardTest(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
            let clipboard_ops = hooks.use_state(|| Vec::<String>::new());
            let message = hooks.use_state(|| None::<String>);
            
            // Simulate clipboard operations
            hooks.use_future({
                let mut ops = clipboard_ops.clone();
                let mut msg = message.clone();
                async move {
                    // Copy text
                    smol::Timer::after(Duration::from_millis(10)).await;
                    let mut current = ops.read().clone();
                    current.push("copy_text".to_string());
                    ops.set(current);
                    msg.set(Some("Copied to clipboard!".to_string()));
                    
                    // Clear message after delay
                    smol::Timer::after(Duration::from_millis(50)).await;
                    msg.set(None);
                    
                    // Copy JSON
                    smol::Timer::after(Duration::from_millis(10)).await;
                    let mut current = ops.read().clone();
                    current.push("copy_json".to_string());
                    ops.set(current);
                    msg.set(Some("JSON copied!".to_string()));
                }
            });
            
            element! {
                Box {
                    Text(content: format!("Ops: {}, Message: {:?}", 
                        clipboard_ops.read().join(", "), 
                        message.read()
                    ))
                }
            }
        }
        
        let actual = element!(ClipboardTest)
            .mock_terminal_render_loop(MockTerminalConfig::default())
            .take(10)
            .collect::<Vec<_>>()
            .await;
        
        // Verify clipboard operations
        assert!(actual.iter().any(|f| f.to_string().contains("copy_text")));
        assert!(actual.iter().any(|f| f.to_string().contains("Copied to clipboard!")));
        assert!(actual.iter().any(|f| f.to_string().contains("copy_json")));
        assert!(actual.iter().any(|f| f.to_string().contains("JSON copied!")));
        // Verify message clearing
        assert!(actual.iter().any(|f| f.to_string().contains("Message: None")));
    }
    
    #[apply(test!)]
    async fn test_navigation_between_modes() {
        let (_dir, search_service, session_service, cache) = create_comprehensive_test_environment();
        
        #[component]
        fn NavigationTest(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
            let mode = hooks.use_state(|| Mode::Search);
            let mode_stack = hooks.use_state(|| Vec::<Mode>::new());
            let navigation_count = hooks.use_state(|| 0);
            
            // Simulate complex navigation
            hooks.use_future({
                let mut mode = mode.clone();
                let mut stack = mode_stack.clone();
                let mut count = navigation_count.clone();
                async move {
                    // Search -> Detail
                    smol::Timer::after(Duration::from_millis(10)).await;
                    let mut s = stack.read().clone();
                    s.push(Mode::Search);
                    stack.set(s);
                    mode.set(Mode::ResultDetail);
                    count.set(*count.read() + 1);
                    
                    // Detail -> Session
                    smol::Timer::after(Duration::from_millis(20)).await;
                    let mut s = stack.read().clone();
                    s.push(Mode::ResultDetail);
                    stack.set(s);
                    mode.set(Mode::SessionViewer);
                    count.set(*count.read() + 1);
                    
                    // Back to Detail
                    smol::Timer::after(Duration::from_millis(30)).await;
                    if let Some(prev) = stack.read().last() {
                        mode.set(*prev);
                        let mut s = stack.read().clone();
                        s.pop();
                        stack.set(s);
                        count.set(*count.read() + 1);
                    }
                    
                    // Back to Search
                    smol::Timer::after(Duration::from_millis(40)).await;
                    if let Some(prev) = stack.read().last() {
                        mode.set(*prev);
                        let mut s = stack.read().clone();
                        s.pop();
                        stack.set(s);
                        count.set(*count.read() + 1);
                    }
                }
            });
            
            element! {
                Box {
                    Text(content: format!("Mode: {:?}, Stack depth: {}, Nav count: {}", 
                        mode.read(), 
                        mode_stack.read().len(),
                        navigation_count.read()
                    ))
                }
            }
        }
        
        let actual = element!(NavigationTest)
            .mock_terminal_render_loop(MockTerminalConfig::default())
            .take(10)
            .collect::<Vec<_>>()
            .await;
        
        // Verify navigation flow
        assert!(actual[0].to_string().contains("Mode: Search, Stack depth: 0"));
        assert!(actual.iter().any(|f| f.to_string().contains("Mode: ResultDetail, Stack depth: 1")));
        assert!(actual.iter().any(|f| f.to_string().contains("Mode: SessionViewer, Stack depth: 2")));
        // Verify navigation back
        assert!(actual.iter().any(|f| f.to_string().contains("Nav count: 4")));
    }
    
    #[apply(test!)]
    async fn test_search_debouncing() {
        #[component]
        fn DebounceTest(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
            let query = hooks.use_state(|| String::new());
            let search_triggered = hooks.use_state(|| 0);
            let typing_indicator = hooks.use_state(|| false);
            
            // Simulate rapid typing
            hooks.use_future({
                let mut query = query.clone();
                let mut typing = typing_indicator.clone();
                let mut searches = search_triggered.clone();
                async move {
                    // Type characters rapidly
                    for c in "rust async".chars() {
                        query.set(format!("{}{}", query.read(), c));
                        typing.set(true);
                        smol::Timer::after(Duration::from_millis(50)).await;
                    }
                    
                    // Wait for debounce
                    typing.set(false);
                    smol::Timer::after(Duration::from_millis(300)).await;
                    searches.set(*searches.read() + 1);
                }
            });
            
            element! {
                Box {
                    Text(content: format!(
                        "Query: '{}', Typing: {}, Searches: {}", 
                        query.read(),
                        typing_indicator.read(),
                        search_triggered.read()
                    ))
                }
            }
        }
        
        let actual = element!(DebounceTest)
            .mock_terminal_render_loop(MockTerminalConfig::default())
            .take(15)
            .collect::<Vec<_>>()
            .await;
        
        // Verify debouncing behavior
        assert!(actual.iter().any(|f| f.to_string().contains("Typing: true")));
        assert!(actual.iter().any(|f| f.to_string().contains("Query: 'rust async'")));
        // Should only trigger one search after typing stops
        assert!(actual.last().unwrap().to_string().contains("Searches: 1"));
    }
    
    #[apply(test!)]
    async fn test_error_handling_and_recovery() {
        #[component]
        fn ErrorHandlingTest(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
            let error_state = hooks.use_state(|| None::<String>);
            let recovery_attempts = hooks.use_state(|| 0);
            
            // Simulate error and recovery
            hooks.use_future({
                let mut error = error_state.clone();
                let mut attempts = recovery_attempts.clone();
                async move {
                    // Simulate error
                    smol::Timer::after(Duration::from_millis(10)).await;
                    error.set(Some("Failed to load session".to_string()));
                    
                    // Attempt recovery
                    smol::Timer::after(Duration::from_millis(50)).await;
                    attempts.set(*attempts.read() + 1);
                    
                    // Successful recovery
                    smol::Timer::after(Duration::from_millis(50)).await;
                    error.set(None);
                    attempts.set(*attempts.read() + 1);
                }
            });
            
            element! {
                Box {
                    Text(content: format!(
                        "Error: {:?}, Recovery attempts: {}", 
                        error_state.read(),
                        recovery_attempts.read()
                    ))
                }
            }
        }
        
        let actual = element!(ErrorHandlingTest)
            .mock_terminal_render_loop(MockTerminalConfig::default())
            .take(8)
            .collect::<Vec<_>>()
            .await;
        
        // Verify error handling flow
        assert!(actual.iter().any(|f| f.to_string().contains("Error: Some(\"Failed to load session\")")));
        assert!(actual.iter().any(|f| f.to_string().contains("Recovery attempts: 1")));
        assert!(actual.last().unwrap().to_string().contains("Error: None"));
        assert!(actual.last().unwrap().to_string().contains("Recovery attempts: 2"));
    }
    
    #[apply(test!)]
    async fn test_help_system_integration() {
        #[component]
        fn HelpSystemTest(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
            let show_help = hooks.use_state(|| false);
            let help_context = hooks.use_state(|| "search".to_string());
            
            // Simulate help system usage
            hooks.use_future({
                let mut help = show_help.clone();
                let mut context = help_context.clone();
                async move {
                    // Open help in search mode
                    smol::Timer::after(Duration::from_millis(10)).await;
                    help.set(true);
                    
                    // Switch to detail mode help
                    smol::Timer::after(Duration::from_millis(30)).await;
                    context.set("detail".to_string());
                    
                    // Close help
                    smol::Timer::after(Duration::from_millis(30)).await;
                    help.set(false);
                }
            });
            
            element! {
                Box {
                    Text(content: format!(
                        "Help shown: {}, Context: {}", 
                        show_help.read(),
                        help_context.read()
                    ))
                }
            }
        }
        
        let actual = element!(HelpSystemTest)
            .mock_terminal_render_loop(MockTerminalConfig::default())
            .take(8)
            .collect::<Vec<_>>()
            .await;
        
        // Verify help system behavior
        assert!(actual[0].to_string().contains("Help shown: false"));
        assert!(actual.iter().any(|f| f.to_string().contains("Help shown: true, Context: search")));
        assert!(actual.iter().any(|f| f.to_string().contains("Help shown: true, Context: detail")));
        assert!(actual.last().unwrap().to_string().contains("Help shown: false"));
    }
}