#[cfg(all(test, feature = "async"))]
mod tests {
    use super::*;
    use crate::SearchOptions;
    use crate::schemas::{SessionMessage, MessageRole, ContentBlock};
    
    #[test]
    fn test_app_state_creation() {
        let options = SearchOptions::default();
        let state = AppState::new("test query".to_string(), options);
        
        assert_eq!(state.query, "test query");
        assert_eq!(state.search_results.len(), 0);
        assert_eq!(state.selected_index, 0);
        assert!(!state.is_searching);
        assert!(!state.show_help);
        assert_eq!(state.mode, ViewMode::Search);
    }
    
    #[test]
    fn test_get_selected_message() {
        let options = SearchOptions::default();
        let mut state = AppState::new("test".to_string(), options);
        
        // Test with empty results
        assert!(state.get_selected_message().is_none());
        
        // Add some messages
        state.search_results = vec![
            SessionMessage {
                role: MessageRole::User,
                content: Some(vec![ContentBlock {
                    text: Some("First message".to_string()),
                    ..Default::default()
                }]),
                ..Default::default()
            },
            SessionMessage {
                role: MessageRole::Assistant,
                content: Some(vec![ContentBlock {
                    text: Some("Second message".to_string()),
                    ..Default::default()
                }]),
                ..Default::default()
            },
        ];
        
        // Test selection
        assert!(state.get_selected_message().is_some());
        let msg = state.get_selected_message().unwrap();
        assert_eq!(msg.role, MessageRole::User);
        
        // Change selection
        state.selected_index = 1;
        let msg = state.get_selected_message().unwrap();
        assert_eq!(msg.role, MessageRole::Assistant);
        
        // Out of bounds
        state.selected_index = 10;
        assert!(state.get_selected_message().is_none());
    }
    
    #[tokio::test]
    async fn test_search_service_new() {
        let options = SearchOptions::default();
        let service = SearchService::new(options.clone(), "test_pattern".to_string());
        
        // Test that search with empty results doesn't panic
        let results = service.search("test").await;
        assert_eq!(results.len(), 0);
    }
    
    #[test]
    fn test_view_mode_transitions() {
        let options = SearchOptions::default();
        let mut state = AppState::new("test".to_string(), options);
        
        // Initial mode
        assert_eq!(state.mode, ViewMode::Search);
        
        // Transition to different modes
        state.mode = ViewMode::ResultDetail;
        assert_eq!(state.mode, ViewMode::ResultDetail);
        
        state.mode = ViewMode::SessionViewer;
        assert_eq!(state.mode, ViewMode::SessionViewer);
        
        state.mode = ViewMode::Help;
        assert_eq!(state.mode, ViewMode::Help);
    }
    
    #[test]
    fn test_app_signal_clone() {
        // Test that AppSignal can be cloned
        let signal = AppSignal::UpdateQuery("test".to_string());
        let cloned = signal.clone();
        
        match cloned {
            AppSignal::UpdateQuery(s) => assert_eq!(s, "test"),
            _ => panic!("Wrong signal type"),
        }
    }
}

#[cfg(all(test, feature = "async"))]
mod search_view_tests {
    use super::*;
    use tokio::sync::mpsc;
    use std::sync::Arc;
    
    #[tokio::test]
    async fn test_search_view_creation() {
        let options = SearchOptions::default();
        let search_service = Arc::new(SearchService::new(options, "test".to_string()));
        let (tx, _rx) = mpsc::channel(10);
        
        let _view = SearchView::new(search_service, tx);
        // Test passes if no panic
    }
}

#[cfg(all(test, feature = "async"))]
mod component_tests {
    use super::*;
    
    #[test]
    fn test_result_detail_view_creation() {
        let _view = ResultDetailView::new();
        // Test passes if no panic
    }
    
    #[test]
    fn test_session_view_creation() {
        let _view = SessionView::new();
        // Test passes if no panic
    }
    
    #[test]
    fn test_help_view_creation() {
        let _view = HelpView::new();
        // Test passes if no panic
    }
}