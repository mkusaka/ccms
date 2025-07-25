#[cfg(test)]
mod search_input_tests {
    use super::super::*;
    use crate::interactive_ratatui::tuirealm_v3::messages::AppMessage;
    use tuirealm::command::{Cmd, CmdResult};
    use tuirealm::event::{Event, Key, KeyEvent, KeyModifiers};
    use tuirealm::props::{AttrValue, Attribute};
    use tuirealm::{Component, MockComponent};

    fn create_search_input() -> SearchInput {
        SearchInput::new()
    }

    #[test]
    fn test_search_input_new() {
        let input = create_search_input();
        
        // Check initial state
        assert_eq!(input.props.get(Attribute::Text), Some(AttrValue::String(String::new())));
    }

    #[test]
    fn test_search_input_text_attribute() {
        let mut input = create_search_input();
        
        // Set text attribute
        input.attr(Attribute::Text, AttrValue::String("test query".to_string()));
        
        // Verify it's stored
        assert_eq!(
            input.query(Attribute::Text),
            Some(AttrValue::String("test query".to_string()))
        );
    }

    #[test]
    fn test_search_input_role_filter_attribute() {
        let mut input = create_search_input();
        
        // Set role filter
        input.attr(
            Attribute::Custom("role_filter"),
            AttrValue::String("User".to_string())
        );
        
        // Verify it's stored
        assert_eq!(
            input.props.get(Attribute::Custom("role_filter")),
            Some(AttrValue::String("User".to_string()))
        );
    }

    #[test]
    fn test_search_input_is_searching_attribute() {
        let mut input = create_search_input();
        
        // Set searching state
        input.attr(Attribute::Custom("is_searching"), AttrValue::Flag(true));
        
        // Verify it's stored
        assert_eq!(
            input.props.get(Attribute::Custom("is_searching")),
            Some(AttrValue::Flag(true))
        );
    }

    #[test]
    fn test_search_input_message_attribute() {
        let mut input = create_search_input();
        
        // Set message
        input.attr(
            Attribute::Custom("message"),
            AttrValue::String("Test message".to_string())
        );
        
        // Verify it's stored
        assert_eq!(
            input.props.get(Attribute::Custom("message")),
            Some(AttrValue::String("Test message".to_string()))
        );
    }

    #[test]
    fn test_search_input_char_input() {
        let mut input = create_search_input();
        input.attr(Attribute::Text, AttrValue::String("test".to_string()));
        
        // Type a character
        let msg = input.on(Event::Keyboard(KeyEvent {
            code: Key::Char('s'),
            modifiers: KeyModifiers::empty(),
        }));
        
        assert_eq!(msg, Some(AppMessage::SearchQueryChanged("tests".to_string())));
    }

    #[test]
    fn test_search_input_backspace() {
        let mut input = create_search_input();
        input.attr(Attribute::Text, AttrValue::String("test".to_string()));
        
        // Press backspace
        let msg = input.on(Event::Keyboard(KeyEvent {
            code: Key::Backspace,
            modifiers: KeyModifiers::empty(),
        }));
        
        assert_eq!(msg, Some(AppMessage::SearchQueryChanged("tes".to_string())));
    }

    #[test]
    fn test_search_input_backspace_empty() {
        let mut input = create_search_input();
        input.attr(Attribute::Text, AttrValue::String("".to_string()));
        
        // Press backspace on empty query
        let msg = input.on(Event::Keyboard(KeyEvent {
            code: Key::Backspace,
            modifiers: KeyModifiers::empty(),
        }));
        
        // Should return None when there's nothing to delete
        assert_eq!(msg, None);
    }

    #[test]
    fn test_search_input_enter() {
        let mut input = create_search_input();
        input.attr(Attribute::Text, AttrValue::String("query".to_string()));
        
        // Press Enter
        let msg = input.on(Event::Keyboard(KeyEvent {
            code: Key::Enter,
            modifiers: KeyModifiers::empty(),
        }));
        
        assert_eq!(msg, Some(AppMessage::SearchRequested));
    }

    #[test]
    fn test_search_input_tab() {
        let mut input = create_search_input();
        
        // Press Tab
        let msg = input.on(Event::Keyboard(KeyEvent {
            code: Key::Tab,
            modifiers: KeyModifiers::empty(),
        }));
        
        assert_eq!(msg, Some(AppMessage::ToggleRoleFilter));
    }

    #[test]
    fn test_search_input_help() {
        let mut input = create_search_input();
        
        // Press ? - This will be treated as a regular character
        let msg = input.on(Event::Keyboard(KeyEvent {
            code: Key::Char('?'),
            modifiers: KeyModifiers::empty(),
        }));
        
        assert_eq!(msg, Some(AppMessage::SearchQueryChanged("?".to_string())));
    }

    #[test]
    fn test_search_input_quit() {
        let mut input = create_search_input();
        
        // Press q - This will be treated as a regular character
        let msg = input.on(Event::Keyboard(KeyEvent {
            code: Key::Char('q'),
            modifiers: KeyModifiers::empty(),
        }));
        
        assert_eq!(msg, Some(AppMessage::SearchQueryChanged("q".to_string())));
    }

    #[test]
    fn test_search_input_esc() {
        let mut input = create_search_input();
        
        // Press Esc - SearchInput doesn't handle Esc
        let msg = input.on(Event::Keyboard(KeyEvent {
            code: Key::Esc,
            modifiers: KeyModifiers::empty(),
        }));
        
        assert_eq!(msg, None);
    }

    #[test]
    fn test_search_input_ctrl_c() {
        let mut input = create_search_input();
        
        // Press Ctrl+C - SearchInput doesn't handle Ctrl+C
        let msg = input.on(Event::Keyboard(KeyEvent {
            code: Key::Char('c'),
            modifiers: KeyModifiers::CONTROL,
        }));
        
        assert_eq!(msg, None);
    }

    #[test]
    fn test_search_input_ctrl_t() {
        let mut input = create_search_input();
        
        // Press Ctrl+T - SearchInput doesn't handle Ctrl+T
        let msg = input.on(Event::Keyboard(KeyEvent {
            code: Key::Char('t'),
            modifiers: KeyModifiers::CONTROL,
        }));
        
        assert_eq!(msg, None);
    }

    #[test]
    fn test_search_input_navigation_keys() {
        let mut input = create_search_input();
        
        // Arrow keys - SearchInput handles these for cursor movement
        assert_eq!(
            input.on(Event::Keyboard(KeyEvent {
                code: Key::Up,
                modifiers: KeyModifiers::empty(),
            })),
            None
        );
        
        assert_eq!(
            input.on(Event::Keyboard(KeyEvent {
                code: Key::Down,
                modifiers: KeyModifiers::empty(),
            })),
            None
        );
        
        // Vim keys - k and j are treated as regular characters
        input.attr(Attribute::Text, AttrValue::String("".to_string()));
        assert_eq!(
            input.on(Event::Keyboard(KeyEvent {
                code: Key::Char('k'),
                modifiers: KeyModifiers::empty(),
            })),
            Some(AppMessage::SearchQueryChanged("k".to_string()))
        );
        
        input.attr(Attribute::Text, AttrValue::String("".to_string()));
        assert_eq!(
            input.on(Event::Keyboard(KeyEvent {
                code: Key::Char('j'),
                modifiers: KeyModifiers::empty(),
            })),
            Some(AppMessage::SearchQueryChanged("j".to_string()))
        );
    }

    #[test]
    fn test_search_input_page_navigation() {
        let mut input = create_search_input();
        
        // SearchInput handles Home/End for cursor movement
        assert_eq!(
            input.on(Event::Keyboard(KeyEvent {
                code: Key::PageUp,
                modifiers: KeyModifiers::empty(),
            })),
            None
        );
        
        assert_eq!(
            input.on(Event::Keyboard(KeyEvent {
                code: Key::PageDown,
                modifiers: KeyModifiers::empty(),
            })),
            None
        );
        
        assert_eq!(
            input.on(Event::Keyboard(KeyEvent {
                code: Key::Home,
                modifiers: KeyModifiers::empty(),
            })),
            None
        );
        
        assert_eq!(
            input.on(Event::Keyboard(KeyEvent {
                code: Key::End,
                modifiers: KeyModifiers::empty(),
            })),
            None
        );
    }

    #[test]
    fn test_search_input_vim_page_navigation() {
        let mut input = create_search_input();
        
        // SearchInput uses Ctrl+B/F for cursor movement
        assert_eq!(
            input.on(Event::Keyboard(KeyEvent {
                code: Key::Char('b'),
                modifiers: KeyModifiers::CONTROL,
            })),
            None
        );
        
        assert_eq!(
            input.on(Event::Keyboard(KeyEvent {
                code: Key::Char('f'),
                modifiers: KeyModifiers::CONTROL,
            })),
            None
        );
        
        // g and G are treated as regular characters
        input.attr(Attribute::Text, AttrValue::String("".to_string()));
        assert_eq!(
            input.on(Event::Keyboard(KeyEvent {
                code: Key::Char('g'),
                modifiers: KeyModifiers::empty(),
            })),
            Some(AppMessage::SearchQueryChanged("g".to_string()))
        );
        
        input.attr(Attribute::Text, AttrValue::String("".to_string()));
        // Shift+G doesn't match the condition due to SHIFT modifier
        assert_eq!(
            input.on(Event::Keyboard(KeyEvent {
                code: Key::Char('G'),
                modifiers: KeyModifiers::SHIFT,
            })),
            None
        );
    }

    #[test]
    fn test_search_input_unknown_key() {
        let mut input = create_search_input();
        
        // Unknown function key
        let msg = input.on(Event::Keyboard(KeyEvent {
            code: Key::Function(1),
            modifiers: KeyModifiers::empty(),
        }));
        
        assert_eq!(msg, None);
    }

    #[test]
    fn test_search_input_perform() {
        let mut input = create_search_input();
        
        // Test various commands
        // Move commands should work
        assert_eq!(input.perform(Cmd::Move(tuirealm::command::Direction::Left)), CmdResult::None);
        
        // Other commands return None
        assert_eq!(input.perform(Cmd::Cancel), CmdResult::None);
        assert_eq!(input.perform(Cmd::Submit), CmdResult::None);
    }

    #[test]
    fn test_search_input_state() {
        let input = create_search_input();
        
        // State should contain cursor position
        assert_eq!(input.state(), tuirealm::State::One(tuirealm::StateValue::Usize(0)));
    }

    #[test]
    fn test_search_input_multibyte_characters() {
        let mut input = create_search_input();
        input.attr(Attribute::Text, AttrValue::String("検索".to_string()));
        
        // Add another character
        let msg = input.on(Event::Keyboard(KeyEvent {
            code: Key::Char('中'),
            modifiers: KeyModifiers::empty(),
        }));
        
        assert_eq!(msg, Some(AppMessage::SearchQueryChanged("検索中".to_string())));
    }

    #[test]
    fn test_search_input_backspace_multibyte() {
        let mut input = create_search_input();
        input.attr(Attribute::Text, AttrValue::String("検索中".to_string()));
        
        // Backspace should remove one character
        let msg = input.on(Event::Keyboard(KeyEvent {
            code: Key::Backspace,
            modifiers: KeyModifiers::empty(),
        }));
        
        assert_eq!(msg, Some(AppMessage::SearchQueryChanged("検索".to_string())));
    }
}