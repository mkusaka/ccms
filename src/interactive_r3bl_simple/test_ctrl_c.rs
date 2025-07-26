#[cfg(test)]
mod tests {
    use super::super::state::*;
    use std::time::Duration;
    use std::thread;

    #[test]
    fn test_ctrl_c_first_press() {
        let mut state = AppState::new();
        
        // First Ctrl+C should not exit
        let should_exit = state.handle_ctrl_c();
        assert!(!should_exit);
        
        // Should set status message
        assert_eq!(state.status_message.as_ref().unwrap(), "Press Ctrl+C again to exit");
        
        // Should record the time
        assert!(state.last_ctrl_c_time.is_some());
        assert_eq!(state.ctrl_c_count, 1);
    }

    #[test]
    fn test_ctrl_c_second_press_quick() {
        let mut state = AppState::new();
        
        // First Ctrl+C
        let should_exit = state.handle_ctrl_c();
        assert!(!should_exit);
        
        // Second Ctrl+C immediately (within 500ms)
        let should_exit = state.handle_ctrl_c();
        assert!(should_exit);
    }

    #[test]
    fn test_ctrl_c_second_press_slow() {
        let mut state = AppState::new();
        
        // First Ctrl+C
        let should_exit = state.handle_ctrl_c();
        assert!(!should_exit);
        
        // Wait more than 500ms
        thread::sleep(Duration::from_millis(600));
        
        // Second Ctrl+C after timeout should reset
        let should_exit = state.handle_ctrl_c();
        assert!(!should_exit);
        
        // Should update status message again
        assert_eq!(state.status_message.as_ref().unwrap(), "Press Ctrl+C again to exit");
    }

    #[test]
    fn test_ctrl_c_multiple_presses() {
        let mut state = AppState::new();
        
        // First press
        assert!(!state.handle_ctrl_c());
        
        // Wait and press again (resets)
        thread::sleep(Duration::from_millis(600));
        assert!(!state.handle_ctrl_c());
        
        // Quick second press (exits)
        assert!(state.handle_ctrl_c());
    }
}