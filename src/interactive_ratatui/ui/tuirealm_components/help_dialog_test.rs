#[cfg(test)]
mod tests {
    use super::super::help_dialog::HelpDialog;
    use crate::interactive_ratatui::ui::tuirealm_components::messages::AppMessage;
    use tuirealm::event::{Key, KeyEvent, KeyModifiers};
    use tuirealm::{Component, Event, NoUserEvent, MockComponent, State};

    fn create_key_event(code: Key) -> Event<NoUserEvent> {
        Event::Keyboard(KeyEvent {
            code,
            modifiers: KeyModifiers::NONE,
        })
    }

    #[test]
    fn test_help_dialog_creation() {
        let help_dialog = HelpDialog::new();
        // State should be None for help dialog
        match help_dialog.state() {
            State::None => {}
            _ => panic!("Expected State::None"),
        }
    }

    #[test]
    fn test_any_key_closes_dialog() {
        let mut help_dialog = HelpDialog::new();
        
        // Test various keys - all should close the dialog
        let msg = help_dialog.on(create_key_event(Key::Char('a')));
        assert_eq!(msg, Some(AppMessage::ExitHelp));
        
        let msg = help_dialog.on(create_key_event(Key::Enter));
        assert_eq!(msg, Some(AppMessage::ExitHelp));
        
        let msg = help_dialog.on(create_key_event(Key::Esc));
        assert_eq!(msg, Some(AppMessage::ExitHelp));
        
        let msg = help_dialog.on(create_key_event(Key::Up));
        assert_eq!(msg, Some(AppMessage::ExitHelp));
        
        let msg = help_dialog.on(create_key_event(Key::Down));
        assert_eq!(msg, Some(AppMessage::ExitHelp));
        
        let msg = help_dialog.on(create_key_event(Key::Tab));
        assert_eq!(msg, Some(AppMessage::ExitHelp));
        
        let msg = help_dialog.on(create_key_event(Key::Backspace));
        assert_eq!(msg, Some(AppMessage::ExitHelp));
        
        let msg = help_dialog.on(create_key_event(Key::Delete));
        assert_eq!(msg, Some(AppMessage::ExitHelp));
        
        let msg = help_dialog.on(create_key_event(Key::Home));
        assert_eq!(msg, Some(AppMessage::ExitHelp));
        
        let msg = help_dialog.on(create_key_event(Key::End));
        assert_eq!(msg, Some(AppMessage::ExitHelp));
        
        let msg = help_dialog.on(create_key_event(Key::PageUp));
        assert_eq!(msg, Some(AppMessage::ExitHelp));
        
        let msg = help_dialog.on(create_key_event(Key::PageDown));
        assert_eq!(msg, Some(AppMessage::ExitHelp));
    }

    #[test]
    fn test_special_key_combinations() {
        let mut help_dialog = HelpDialog::new();
        
        // Test with modifiers
        let msg = help_dialog.on(Event::Keyboard(KeyEvent {
            code: Key::Char('a'),
            modifiers: KeyModifiers::CONTROL,
        }));
        assert_eq!(msg, Some(AppMessage::ExitHelp));
        
        let msg = help_dialog.on(Event::Keyboard(KeyEvent {
            code: Key::Char('a'),
            modifiers: KeyModifiers::ALT,
        }));
        assert_eq!(msg, Some(AppMessage::ExitHelp));
        
        let msg = help_dialog.on(Event::Keyboard(KeyEvent {
            code: Key::Char('a'),
            modifiers: KeyModifiers::SHIFT,
        }));
        assert_eq!(msg, Some(AppMessage::ExitHelp));
    }

    #[test]
    fn test_state_is_none() {
        let help_dialog = HelpDialog::new();
        match help_dialog.state() {
            State::None => {}
            _ => panic!("HelpDialog should always return State::None"),
        }
    }

    #[test]
    fn test_default_trait() {
        let help_dialog = HelpDialog::default();
        match help_dialog.state() {
            State::None => {}
            _ => panic!("Expected State::None"),
        }
    }
}