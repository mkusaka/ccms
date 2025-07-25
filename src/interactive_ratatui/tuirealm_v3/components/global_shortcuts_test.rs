#[cfg(test)]
mod global_shortcuts_tests {
    use super::super::*;
    use crate::interactive_ratatui::tuirealm_v3::messages::{AppMessage, AppMode};
    use tuirealm::event::{Event, Key, KeyEvent, KeyModifiers};
    use tuirealm::props::{AttrValue, Attribute};
    use tuirealm::{Component, MockComponent, NoUserEvent};
    use std::time::Duration;
    use std::thread;
    
    fn create_global_shortcuts() -> GlobalShortcuts {
        GlobalShortcuts::new()
    }
    
    fn set_mode(shortcuts: &mut GlobalShortcuts, mode: AppMode) {
        let mode_value = match mode {
            AppMode::Search => 0,
            AppMode::ResultDetail => 1,
            AppMode::SessionViewer => 2,
            AppMode::Help => 3,
            AppMode::Error => 4,
        };
        shortcuts.attr(
            Attribute::Custom("current_mode"),
            AttrValue::Number(mode_value),
        );
    }
    
    #[test]
    fn test_ctrl_c_double_press_exit() {
        let mut shortcuts = create_global_shortcuts();
        set_mode(&mut shortcuts, AppMode::Search);
        
        let key_event = KeyEvent {
            code: Key::Char('c'),
            modifiers: KeyModifiers::CONTROL,
        };
        
        // First press
        let msg = shortcuts.on(Event::Keyboard(key_event.clone()));
        assert!(matches!(msg, Some(AppMessage::ShowMessage(msg)) if msg.contains("Press Ctrl+C again")));
        
        // Second press within timeout
        let msg = shortcuts.on(Event::Keyboard(key_event));
        assert!(matches!(msg, Some(AppMessage::Quit)));
    }
    
    #[test]
    fn test_ctrl_c_timeout() {
        let mut shortcuts = create_global_shortcuts();
        set_mode(&mut shortcuts, AppMode::Search);
        
        let key_event = KeyEvent {
            code: Key::Char('c'),
            modifiers: KeyModifiers::CONTROL,
        };
        
        // First press
        let msg = shortcuts.on(Event::Keyboard(key_event.clone()));
        assert!(matches!(msg, Some(AppMessage::ShowMessage(msg)) if msg.contains("Press Ctrl+C again")));
        
        // Wait for timeout
        thread::sleep(Duration::from_millis(600));
        
        // Second press after timeout - should show message again, not quit
        let msg = shortcuts.on(Event::Keyboard(key_event));
        assert!(matches!(msg, Some(AppMessage::ShowMessage(msg)) if msg.contains("Press Ctrl+C again")));
    }
    
    #[test]
    fn test_question_mark_help() {
        let mut shortcuts = create_global_shortcuts();
        
        let key_event = KeyEvent {
            code: Key::Char('?'),
            modifiers: KeyModifiers::empty(),
        };
        
        // Test in different modes
        for mode in &[AppMode::Search, AppMode::ResultDetail, AppMode::SessionViewer, AppMode::Error] {
            set_mode(&mut shortcuts, mode.clone());
            let msg = shortcuts.on(Event::Keyboard(key_event.clone()));
            assert!(matches!(msg, Some(AppMessage::ShowHelp)));
        }
        
        // Should not trigger in Help mode
        set_mode(&mut shortcuts, AppMode::Help);
        let msg = shortcuts.on(Event::Keyboard(key_event));
        assert!(msg.is_none());
    }
    
    #[test]
    fn test_h_key_help() {
        let mut shortcuts = create_global_shortcuts();
        
        let key_event = KeyEvent {
            code: Key::Char('h'),
            modifiers: KeyModifiers::empty(),
        };
        
        // Test in different modes
        for mode in &[AppMode::Search, AppMode::ResultDetail, AppMode::SessionViewer, AppMode::Error] {
            set_mode(&mut shortcuts, mode.clone());
            let msg = shortcuts.on(Event::Keyboard(key_event.clone()));
            assert!(matches!(msg, Some(AppMessage::ShowHelp)));
        }
        
        // Should not trigger in Help mode
        set_mode(&mut shortcuts, AppMode::Help);
        let msg = shortcuts.on(Event::Keyboard(key_event));
        assert!(msg.is_none());
    }
    
    #[test]
    fn test_ctrl_t_truncation() {
        let mut shortcuts = create_global_shortcuts();
        set_mode(&mut shortcuts, AppMode::Search);
        
        let key_event = KeyEvent {
            code: Key::Char('t'),
            modifiers: KeyModifiers::CONTROL,
        };
        
        let msg = shortcuts.on(Event::Keyboard(key_event));
        assert!(matches!(msg, Some(AppMessage::ToggleTruncation)));
    }
    
    #[test]
    fn test_vim_navigation_search_mode() {
        let mut shortcuts = create_global_shortcuts();
        set_mode(&mut shortcuts, AppMode::Search);
        
        // Test j key
        let msg = shortcuts.on(Event::Keyboard(KeyEvent {
            code: Key::Char('j'),
            modifiers: KeyModifiers::empty(),
        }));
        assert!(matches!(msg, Some(AppMessage::ResultDown)));
        
        // Test k key
        let msg = shortcuts.on(Event::Keyboard(KeyEvent {
            code: Key::Char('k'),
            modifiers: KeyModifiers::empty(),
        }));
        assert!(matches!(msg, Some(AppMessage::ResultUp)));
    }
    
    #[test]
    fn test_vim_navigation_detail_mode() {
        let mut shortcuts = create_global_shortcuts();
        set_mode(&mut shortcuts, AppMode::ResultDetail);
        
        // Test j key
        let msg = shortcuts.on(Event::Keyboard(KeyEvent {
            code: Key::Char('j'),
            modifiers: KeyModifiers::empty(),
        }));
        assert!(matches!(msg, Some(AppMessage::DetailScrollDown)));
        
        // Test k key
        let msg = shortcuts.on(Event::Keyboard(KeyEvent {
            code: Key::Char('k'),
            modifiers: KeyModifiers::empty(),
        }));
        assert!(matches!(msg, Some(AppMessage::DetailScrollUp)));
    }
    
    #[test]
    fn test_vim_navigation_session_mode() {
        let mut shortcuts = create_global_shortcuts();
        set_mode(&mut shortcuts, AppMode::SessionViewer);
        
        // Test j key
        let msg = shortcuts.on(Event::Keyboard(KeyEvent {
            code: Key::Char('j'),
            modifiers: KeyModifiers::empty(),
        }));
        assert!(matches!(msg, Some(AppMessage::SessionScrollDown)));
        
        // Test k key
        let msg = shortcuts.on(Event::Keyboard(KeyEvent {
            code: Key::Char('k'),
            modifiers: KeyModifiers::empty(),
        }));
        assert!(matches!(msg, Some(AppMessage::SessionScrollUp)));
    }
    
    #[test]
    fn test_copy_shortcuts() {
        let mut shortcuts = create_global_shortcuts();
        
        let key_event = KeyEvent {
            code: Key::Char('y'),
            modifiers: KeyModifiers::CONTROL,
        };
        
        // Test in different modes
        for mode in &[AppMode::Search, AppMode::ResultDetail, AppMode::SessionViewer] {
            set_mode(&mut shortcuts, mode.clone());
            let msg = shortcuts.on(Event::Keyboard(key_event.clone()));
            assert!(matches!(msg, Some(AppMessage::CopyRawJson)));
        }
    }
    
    #[test]
    fn test_help_mode_exit() {
        let mut shortcuts = create_global_shortcuts();
        set_mode(&mut shortcuts, AppMode::Help);
        
        // Test Esc key
        let msg = shortcuts.on(Event::Keyboard(KeyEvent {
            code: Key::Esc,
            modifiers: KeyModifiers::empty(),
        }));
        assert!(matches!(msg, Some(AppMessage::ExitHelp)));
        
        // Test q key
        let msg = shortcuts.on(Event::Keyboard(KeyEvent {
            code: Key::Char('q'),
            modifiers: KeyModifiers::empty(),
        }));
        assert!(matches!(msg, Some(AppMessage::ExitHelp)));
    }
    
    #[test]
    fn test_unhandled_keys() {
        let mut shortcuts = create_global_shortcuts();
        set_mode(&mut shortcuts, AppMode::Search);
        
        // Test random key that should not be handled
        let msg = shortcuts.on(Event::Keyboard(KeyEvent {
            code: Key::Char('x'),
            modifiers: KeyModifiers::empty(),
        }));
        assert!(msg.is_none());
        
        // Test F1 key
        let msg = shortcuts.on(Event::Keyboard(KeyEvent {
            code: Key::Function(1),
            modifiers: KeyModifiers::empty(),
        }));
        assert!(msg.is_none());
    }
}