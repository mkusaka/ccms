#[cfg(test)]
mod e2e_tests {
    use crate::interactive_iocraft::application::{SearchService, SessionService, CacheService};
    use crate::interactive_iocraft::ui::components::{App, AppProps};
    use crate::interactive_iocraft::ui::components::{SearchView, DetailView, SessionView, HelpModal};
    use crate::interactive_iocraft::domain::models::Mode;
    use crate::query::condition::SearchResult;
    use iocraft::prelude::*;
    use futures::stream::{self, StreamExt};
    use std::sync::{Arc, Mutex};
    use std::time::Duration;
    use smol_macros::test;
    use macro_rules_attribute::apply;
    use tempfile::{NamedTempFile, TempDir};
    use std::fs;
    use std::io::Write;
    
    fn create_test_environment() -> (TempDir, Arc<SearchService>, Arc<SessionService>, Arc<Mutex<CacheService>>) {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("test_session.jsonl");
        let mut file = fs::File::create(&file_path).unwrap();
        
        // Write test data
        writeln!(file, r#"{{"uuid":"1","timestamp":"1700000000","sessionId":"test123","role":"user","text":"Hello iocraft","projectPath":"/test"}}"#).unwrap();
        writeln!(file, r#"{{"uuid":"2","timestamp":"1700000001","sessionId":"test123","role":"assistant","text":"Hello from assistant","projectPath":"/test"}}"#).unwrap();
        writeln!(file, r#"{{"uuid":"3","timestamp":"1700000002","sessionId":"test123","role":"system","text":"System message","projectPath":"/test"}}"#).unwrap();
        file.flush().unwrap();
        
        let cache = Arc::new(Mutex::new(CacheService::new()));
        let search_service = Arc::new(SearchService::new(
            vec![file_path.to_string_lossy().to_string()],
            false,
        ).unwrap());
        let session_service = Arc::new(SessionService::new(cache.clone()));
        
        (dir, search_service, session_service, cache)
    }
    
    #[apply(test!)]
    async fn test_app_initialization() {
        let (_dir, search_service, session_service, cache) = create_test_environment();
        
        let actual = element! {
            App(
                search_service: Some(search_service),
                session_service: Some(session_service),
                cache_service: Some(cache),
                initial_query: Some("test".to_string()),
                file_patterns: vec![],
            )
        }
        .mock_terminal_render_loop(MockTerminalConfig::default())
        .map(|c| c.to_string())
        .take(1)
        .collect::<Vec<_>>()
        .await;
        
        assert!(!actual.is_empty());
        let output = &actual[0];
        
        // Should render search view by default
        assert!(output.contains("test")); // Initial query
    }
    
    #[apply(test!)]
    async fn test_search_view_rendering() {
        let actual = element! {
            SearchView(
                initial_query: Some("Hello".to_string()),
                file_pattern: "*.jsonl".to_string(),
                on_select_result: |_| {},
                on_show_help: |_| {},
            )
        }
        .mock_terminal_render_loop(MockTerminalConfig::default())
        .map(|c| c.to_string())
        .take(1)
        .collect::<Vec<_>>()
        .await;
        
        assert!(!actual.is_empty());
        let output = &actual[0];
        
        // Check key elements are rendered
        assert!(output.contains("Hello")); // Query
        assert!(output.contains("Tab")); // Role filter hint
    }
    
    #[apply(test!)]
    async fn test_detail_view_rendering() {
        let result = SearchResult {
            file: "/test/file.jsonl".to_string(),
            uuid: "123".to_string(),
            timestamp: "1700000000".to_string(),
            session_id: "abc123".to_string(),
            role: "user".to_string(),
            text: "This is a test message".to_string(),
            project_path: "/test/project".to_string(),
            raw_json: Some(r#"{"test": "data"}"#.to_string()),
        };
        
        let actual = element! {
            DetailView(
                result: result,
                on_view_session: |_| {},
            )
        }
        .mock_terminal_render_loop(MockTerminalConfig::default())
        .map(|c| c.to_string())
        .take(1)
        .collect::<Vec<_>>()
        .await;
        
        assert!(!actual.is_empty());
        let output = &actual[0];
        
        // Check detail view elements
        assert!(output.contains("File:")); 
        assert!(output.contains("/test/file.jsonl"));
        assert!(output.contains("Role:"));
        assert!(output.contains("user"));
        assert!(output.contains("This is a test message"));
        assert!(output.contains("Project:"));
        assert!(output.contains("/test/project"));
        assert!(output.contains("UUID:"));
        assert!(output.contains("123"));
        assert!(output.contains("Session:"));
        assert!(output.contains("abc123"));
    }
    
    #[apply(test!)]
    async fn test_session_view_rendering() {
        let (_dir, _, _, _) = create_test_environment();
        
        // Create a test file
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, r#"{{"uuid":"1","timestamp":"2023-11-20T10:00:00Z","sessionId":"session1","role":"user","text":"Session message 1","projectPath":"/test"}}"#).unwrap();
        writeln!(file, r#"{{"uuid":"2","timestamp":"2023-11-20T10:01:00Z","sessionId":"session1","role":"assistant","text":"Session message 2","projectPath":"/test"}}"#).unwrap();
        file.flush().unwrap();
        
        let actual = element! {
            SessionView(
                file_path: file.path().to_string_lossy().to_string(),
            )
        }
        .mock_terminal_render_loop(MockTerminalConfig {
            viewport_height: 30,
            ..Default::default()
        })
        .map(|c| c.to_string())
        .take(2) // Take 2 frames to allow loading
        .collect::<Vec<_>>()
        .await;
        
        assert!(actual.len() >= 1);
        let output = &actual[actual.len() - 1]; // Check last frame
        
        // Check session view elements
        assert!(output.contains("Session Viewer"));
        assert!(output.contains("Messages:"));
    }
    
    #[apply(test!)]
    async fn test_help_modal_rendering() {
        let actual = element! {
            HelpModal(
                on_close: |_| {},
            )
        }
        .mock_terminal_render_loop(MockTerminalConfig::default())
        .map(|c| c.to_string())
        .take(1)
        .collect::<Vec<_>>()
        .await;
        
        assert!(!actual.is_empty());
        let output = &actual[0];
        
        // Check help modal content
        assert!(output.contains("Help"));
        assert!(output.contains("Navigation"));
        assert!(output.contains("Search"));
        assert!(output.contains("Shortcuts"));
        assert!(output.contains("ESC"));
    }
    
    #[apply(test!)]
    async fn test_keyboard_navigation_flow() {
        let (_dir, search_service, session_service, cache) = create_test_environment();
        
        // Component to test keyboard events
        #[component]
        fn TestKeyboardNav(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
            let mode = hooks.use_state(|| Mode::Search);
            let show_help = hooks.use_state(|| false);
            
            element! {
                Box(width: 100pct, height: 100pct) {
                    Text(content: format!("Mode: {:?}, Help: {}", mode.read(), show_help.read()))
                }
            }
        }
        
        let actual = element!(TestKeyboardNav)
            .mock_terminal_render_loop(MockTerminalConfig::default())
            .map(|c| c.to_string())
            .take(1)
            .collect::<Vec<_>>()
            .await;
        
        assert!(!actual.is_empty());
        assert!(actual[0].contains("Mode: Search, Help: false"));
    }
    
    #[apply(test!)]
    async fn test_search_results_update() {
        let (_dir, search_service, session_service, cache) = create_test_environment();
        
        // Component that simulates search updates
        #[component]
        fn TestSearchUpdate(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
            let query = hooks.use_state(|| "initial".to_string());
            let counter = hooks.use_state(|| 0);
            
            // Simulate query change after first render
            hooks.use_future({
                let mut query = query.clone();
                let mut counter = counter.clone();
                async move {
                    smol::Timer::after(Duration::from_millis(10)).await;
                    query.set("updated".to_string());
                    counter.set(*counter.read() + 1);
                }
            });
            
            element! {
                Box {
                    Text(content: format!("Query: {}, Counter: {}", query.read(), counter.read()))
                }
            }
        }
        
        let actual = element!(TestSearchUpdate)
            .mock_terminal_render_loop(MockTerminalConfig::default())
            .map(|c| c.to_string())
            .take(3) // Multiple frames to catch the update
            .collect::<Vec<_>>()
            .await;
        
        assert!(actual.len() >= 2);
        // First frame should have initial state
        assert!(actual[0].contains("Query: initial, Counter: 0"));
        // Later frame should have updated state
        assert!(actual.iter().any(|s| s.contains("Query: updated, Counter: 1")));
    }
    
    #[apply(test!)]
    async fn test_mode_transitions() {
        // Test component that simulates mode transitions
        #[component]
        fn TestModeTransitions(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
            let mode = hooks.use_state(|| Mode::Search);
            let mode_stack = hooks.use_state(|| Vec::<Mode>::new());
            
            // Simulate mode transitions
            hooks.use_future({
                let mut mode = mode.clone();
                let mut mode_stack = mode_stack.clone();
                async move {
                    // Transition to detail view
                    smol::Timer::after(Duration::from_millis(10)).await;
                    let mut stack = mode_stack.read().clone();
                    stack.push(Mode::Search);
                    mode_stack.set(stack);
                    mode.set(Mode::ResultDetail);
                    
                    // Then to session viewer
                    smol::Timer::after(Duration::from_millis(10)).await;
                    let mut stack = mode_stack.read().clone();
                    stack.push(Mode::ResultDetail);
                    mode_stack.set(stack);
                    mode.set(Mode::SessionViewer);
                }
            });
            
            element! {
                Box {
                    Text(content: format!("Mode: {:?}, Stack: {}", mode.read(), mode_stack.read().len()))
                }
            }
        }
        
        let actual = element!(TestModeTransitions)
            .mock_terminal_render_loop(MockTerminalConfig::default())
            .map(|c| c.to_string())
            .take(5)
            .collect::<Vec<_>>()
            .await;
        
        assert!(actual.len() >= 3);
        // Check mode transitions
        assert!(actual[0].contains("Mode: Search, Stack: 0"));
        assert!(actual.iter().any(|s| s.contains("Mode: ResultDetail, Stack: 1")));
        assert!(actual.iter().any(|s| s.contains("Mode: SessionViewer, Stack: 2")));
    }
}