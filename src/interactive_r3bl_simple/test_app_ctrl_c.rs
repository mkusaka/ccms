#[cfg(test)]
mod tests {
    use super::super::app::*;
    use super::super::state::*;
    use crate::SearchOptions;
    use std::sync::Arc;
    use tokio::sync::Mutex;

    fn create_test_app() -> (SearchApp, Arc<Mutex<AppState>>) {
        let state = Arc::new(Mutex::new(AppState::new()));
        let options = SearchOptions::default();
        let app = SearchApp::new("test.jsonl".to_string(), options, state.clone());
        (app, state)
    }

    #[tokio::test]
    async fn test_handle_ctrl_c_first_press() {
        let (mut app, state) = create_test_app();
        
        let mut state_lock = state.lock().await;
        let should_exit = app.handle_input('\x03', &mut state_lock).await.unwrap();
        
        assert!(!should_exit);
        assert_eq!(state_lock.status_message.as_ref().unwrap(), "Press Ctrl+C again to exit");
    }

    #[tokio::test]
    async fn test_handle_ctrl_c_double_press() {
        let (mut app, state) = create_test_app();
        
        let mut state_lock = state.lock().await;
        
        // First Ctrl+C
        let should_exit = app.handle_input('\x03', &mut state_lock).await.unwrap();
        assert!(!should_exit);
        
        // Second Ctrl+C (immediately)
        let should_exit = app.handle_input('\x03', &mut state_lock).await.unwrap();
        assert!(should_exit);
    }
}