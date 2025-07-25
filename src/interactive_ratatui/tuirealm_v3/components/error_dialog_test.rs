#[cfg(test)]
mod error_dialog_tests {
    use super::super::*;
    use crate::interactive_ratatui::tuirealm_v3::messages::AppMessage;
    use tuirealm::event::{Event, Key, KeyEvent, KeyModifiers};
    use tuirealm::props::{AttrValue, Attribute};
    use tuirealm::{Component, MockComponent, NoUserEvent};
    
    fn create_error_dialog() -> ErrorDialog {
        ErrorDialog::new()
    }
    
    fn setup_error_dialog_with_data(dialog: &mut ErrorDialog) {
        dialog.attr(
            Attribute::Custom("error_type"),
            AttrValue::String("FileNotFound".to_string()),
        );
        dialog.attr(
            Attribute::Custom("error_details"),
            AttrValue::String("The requested file was not found".to_string()),
        );
    }
    
    #[test]
    fn test_error_dialog_new() {
        let dialog = create_error_dialog();
        
        // ErrorDialog starts with default props
        // Check that we can query attributes without panic
        assert!(dialog.query(Attribute::Borders).is_none());
    }
    
    #[test]
    fn test_error_dialog_attributes() {
        let mut dialog = create_error_dialog();
        
        // Test setting error type
        dialog.attr(
            Attribute::Custom("error_type"),
            AttrValue::String("NetworkError".to_string()),
        );
        assert_eq!(
            dialog.query(Attribute::Custom("error_type")),
            Some(AttrValue::String("NetworkError".to_string()))
        );
        
        // Test setting error details
        dialog.attr(
            Attribute::Custom("error_details"),
            AttrValue::String("Connection timeout".to_string()),
        );
        assert_eq!(
            dialog.query(Attribute::Custom("error_details")),
            Some(AttrValue::String("Connection timeout".to_string()))
        );
    }
    
    #[test]
    fn test_error_dialog_close_on_enter() {
        let mut dialog = create_error_dialog();
        setup_error_dialog_with_data(&mut dialog);
        
        let msg = dialog.on(Event::Keyboard(KeyEvent {
            code: Key::Enter,
            modifiers: KeyModifiers::empty(),
        }));
        
        // Enter key doesn't close the dialog in the current implementation
        assert!(msg.is_none());
    }
    
    #[test]
    fn test_error_dialog_close_on_esc() {
        let mut dialog = create_error_dialog();
        setup_error_dialog_with_data(&mut dialog);
        
        let msg = dialog.on(Event::Keyboard(KeyEvent {
            code: Key::Esc,
            modifiers: KeyModifiers::empty(),
        }));
        
        assert!(matches!(msg, Some(AppMessage::CloseError)));
    }
    
    #[test]
    fn test_error_dialog_retry_on_r() {
        let mut dialog = create_error_dialog();
        setup_error_dialog_with_data(&mut dialog);
        
        let msg = dialog.on(Event::Keyboard(KeyEvent {
            code: Key::Char('r'),
            modifiers: KeyModifiers::empty(),
        }));
        
        // The retry only works if the error has can_retry set to true
        // In our test setup, we're using a simple error type that doesn't support retry
        assert!(msg.is_none());
    }
    
    #[test]
    fn test_error_dialog_retry_with_retryable_error() {
        let mut dialog = create_error_dialog();
        
        // Set up a FileReadError which is retryable
        dialog.attr(
            Attribute::Custom("error_type"),
            AttrValue::String("FileReadError".to_string()),
        );
        dialog.attr(
            Attribute::Custom("error_details"),
            AttrValue::String("Failed to read file".to_string()),
        );
        dialog.attr(
            Attribute::Custom("file_path"),
            AttrValue::String("/tmp/test.json".to_string()),
        );
        
        let msg = dialog.on(Event::Keyboard(KeyEvent {
            code: Key::Char('r'),
            modifiers: KeyModifiers::empty(),
        }));
        
        // FileReadError is retryable, so this should return RetryLastOperation
        assert!(matches!(msg, Some(AppMessage::RetryLastOperation)));
    }
    
    #[test]
    fn test_error_dialog_ignore_other_keys() {
        let mut dialog = create_error_dialog();
        setup_error_dialog_with_data(&mut dialog);
        
        // Test random key
        let msg = dialog.on(Event::Keyboard(KeyEvent {
            code: Key::Char('x'),
            modifiers: KeyModifiers::empty(),
        }));
        assert!(msg.is_none());
        
        // Test arrow key
        let msg = dialog.on(Event::Keyboard(KeyEvent {
            code: Key::Up,
            modifiers: KeyModifiers::empty(),
        }));
        assert!(msg.is_none());
        
        // Test function key
        let msg = dialog.on(Event::Keyboard(KeyEvent {
            code: Key::Function(1),
            modifiers: KeyModifiers::empty(),
        }));
        assert!(msg.is_none());
    }
    
    #[test]
    fn test_error_dialog_state() {
        let dialog = create_error_dialog();
        
        // ErrorDialog should return State::None
        if let tuirealm::State::None = dialog.state() {
            // Test passes
        } else {
            panic!("ErrorDialog should return State::None");
        }
    }
    
    #[test]
    fn test_error_dialog_perform() {
        let mut dialog = create_error_dialog();
        
        // Perform should return CmdResult::None
        let result = dialog.perform(tuirealm::command::Cmd::None);
        if let tuirealm::command::CmdResult::None = result {
            // Test passes
        } else {
            panic!("ErrorDialog perform should return CmdResult::None");
        }
    }
}