#[cfg(test)]
mod result_list_tests {
    use super::super::*;
    use crate::interactive_ratatui::tuirealm_v3::messages::AppMessage;
    use crate::interactive_ratatui::tuirealm_v3::type_safe_wrapper::{SearchResults, TypeSafeAttr};
    use crate::query::condition::{SearchResult, QueryCondition};
    use tuirealm::command::{Cmd, CmdResult};
    use tuirealm::event::{Event, Key, KeyEvent, KeyModifiers};
    use tuirealm::props::{AttrValue, Attribute};
    use tuirealm::{Component, MockComponent};

    fn create_result_list() -> ResultList {
        ResultList::new()
    }

    fn setup_result_list_with_data(list: &mut ResultList) {
        // Set up sample data
        let results = vec![
            SearchResult {
                file: "test.json".to_string(),
                uuid: "uuid1".to_string(),
                timestamp: "2024-01-01T10:30:45Z".to_string(),
                session_id: "session1".to_string(),
                role: "User".to_string(),
                text: "First message".to_string(),
                has_tools: false,
                has_thinking: false,
                message_type: "message".to_string(),
                query: QueryCondition::Literal { pattern: "test".to_string(), case_sensitive: false },
                project_path: "/test".to_string(),
                raw_json: None,
            },
            SearchResult {
                file: "test.json".to_string(),
                uuid: "uuid2".to_string(),
                timestamp: "2024-01-01T10:31:20Z".to_string(),
                session_id: "session1".to_string(),
                role: "Assistant".to_string(),
                text: "Second message".to_string(),
                has_tools: false,
                has_thinking: false,
                message_type: "message".to_string(),
                query: QueryCondition::Literal { pattern: "test".to_string(), case_sensitive: false },
                project_path: "/test".to_string(),
                raw_json: None,
            },
            SearchResult {
                file: "test2.json".to_string(),
                uuid: "uuid3".to_string(),
                timestamp: "2024-01-01T10:32:15Z".to_string(),
                session_id: "session2".to_string(),
                role: "System".to_string(),
                text: "Third message".to_string(),
                has_tools: false,
                has_thinking: false,
                message_type: "message".to_string(),
                query: QueryCondition::Literal { pattern: "test".to_string(), case_sensitive: false },
                project_path: "/test".to_string(),
                raw_json: None,
            },
        ];
        
        list.attr(
            Attribute::Custom("search_results"),
            SearchResults(results).to_attr_value()
        );
        list.attr(
            Attribute::Custom("result_count"),
            AttrValue::String("3".to_string())
        );
        list.attr(
            Attribute::Value,
            AttrValue::Length(1)
        );
    }

    #[test]
    fn test_result_list_new() {
        let list = create_result_list();
        
        // Check borders are set
        assert!(list.props.get(Attribute::Borders).is_some());
    }

    #[test]
    fn test_result_list_attributes() {
        let mut list = create_result_list();
        
        // Test setting various attributes
        list.attr(Attribute::Value, AttrValue::Length(5));
        assert_eq!(list.query(Attribute::Value), Some(AttrValue::Length(5)));
        
        list.attr(Attribute::Custom("truncate"), AttrValue::Flag(true));
        assert_eq!(
            list.props.get(Attribute::Custom("truncate")),
            Some(AttrValue::Flag(true))
        );
    }

    #[test]
    fn test_result_list_enter_key() {
        let mut list = create_result_list();
        setup_result_list_with_data(&mut list);
        
        // Press Enter with selected index 1
        let msg = list.on(Event::Keyboard(KeyEvent {
            code: Key::Enter,
            modifiers: KeyModifiers::empty(),
        }));
        
        assert_eq!(msg, Some(AppMessage::EnterResultDetail(1)));
    }

    #[test]
    fn test_result_list_navigation_keys() {
        let mut list = create_result_list();
        setup_result_list_with_data(&mut list);
        
        // Up arrow
        assert_eq!(
            list.on(Event::Keyboard(KeyEvent {
                code: Key::Up,
                modifiers: KeyModifiers::empty(),
            })),
            Some(AppMessage::ResultUp)
        );
        
        // Down arrow
        assert_eq!(
            list.on(Event::Keyboard(KeyEvent {
                code: Key::Down,
                modifiers: KeyModifiers::empty(),
            })),
            Some(AppMessage::ResultDown)
        );
        
        // Vim keys
        assert_eq!(
            list.on(Event::Keyboard(KeyEvent {
                code: Key::Char('k'),
                modifiers: KeyModifiers::empty(),
            })),
            Some(AppMessage::ResultUp)
        );
        
        assert_eq!(
            list.on(Event::Keyboard(KeyEvent {
                code: Key::Char('j'),
                modifiers: KeyModifiers::empty(),
            })),
            Some(AppMessage::ResultDown)
        );
    }

    #[test]
    fn test_result_list_page_navigation() {
        let mut list = create_result_list();
        
        assert_eq!(
            list.on(Event::Keyboard(KeyEvent {
                code: Key::PageUp,
                modifiers: KeyModifiers::empty(),
            })),
            Some(AppMessage::ResultPageUp)
        );
        
        assert_eq!(
            list.on(Event::Keyboard(KeyEvent {
                code: Key::PageDown,
                modifiers: KeyModifiers::empty(),
            })),
            Some(AppMessage::ResultPageDown)
        );
        
        assert_eq!(
            list.on(Event::Keyboard(KeyEvent {
                code: Key::Home,
                modifiers: KeyModifiers::empty(),
            })),
            Some(AppMessage::ResultHome)
        );
        
        assert_eq!(
            list.on(Event::Keyboard(KeyEvent {
                code: Key::End,
                modifiers: KeyModifiers::empty(),
            })),
            Some(AppMessage::ResultEnd)
        );
    }

    #[test]
    fn test_result_list_tab_no_action() {
        let mut list = create_result_list();
        
        let msg = list.on(Event::Keyboard(KeyEvent {
            code: Key::Tab,
            modifiers: KeyModifiers::empty(),
        }));
        
        // ResultList doesn't handle Tab
        assert_eq!(msg, None);
    }

    #[test]
    fn test_result_list_t_truncation() {
        let mut list = create_result_list();
        
        // ResultList handles 't' without modifiers for truncation
        let msg = list.on(Event::Keyboard(KeyEvent {
            code: Key::Char('t'),
            modifiers: KeyModifiers::empty(),
        }));
        
        assert_eq!(msg, Some(AppMessage::ToggleTruncation));
    }

    #[test]
    fn test_result_list_quit_keys_not_handled() {
        let mut list = create_result_list();
        
        // ResultList doesn't handle quit keys - these should return None
        assert_eq!(
            list.on(Event::Keyboard(KeyEvent {
                code: Key::Esc,
                modifiers: KeyModifiers::empty(),
            })),
            None
        );
        
        assert_eq!(
            list.on(Event::Keyboard(KeyEvent {
                code: Key::Char('q'),
                modifiers: KeyModifiers::empty(),
            })),
            None
        );
        
        assert_eq!(
            list.on(Event::Keyboard(KeyEvent {
                code: Key::Char('c'),
                modifiers: KeyModifiers::CONTROL,
            })),
            None
        );
    }

    #[test]
    fn test_result_list_help_key_not_handled() {
        let mut list = create_result_list();
        
        // ResultList doesn't handle help key
        let msg = list.on(Event::Keyboard(KeyEvent {
            code: Key::Char('?'),
            modifiers: KeyModifiers::empty(),
        }));
        
        assert_eq!(msg, None);
    }

    #[test]
    fn test_result_list_parse_results() {
        let mut list = create_result_list();
        setup_result_list_with_data(&mut list);
        
        // The component should be able to parse the search_results
        let attr = list.props.get(Attribute::Custom("search_results"));
        assert!(attr.is_some());
        
        if let Some(attr_value) = attr {
            if let Some(results) = SearchResults::from_attr_value(&attr_value) {
                assert_eq!(results.0.len(), 3);
                assert_eq!(results.0[0].text, "First message");
                assert_eq!(results.0[1].text, "Second message");
                assert_eq!(results.0[2].text, "Third message");
            } else {
                panic!("Failed to parse SearchResults from AttrValue");
            }
        }
    }

    #[test]
    fn test_result_list_empty_results() {
        let mut list = create_result_list();
        
        // Set empty results
        list.attr(
            Attribute::Custom("search_results"),
            SearchResults(vec![]).to_attr_value()
        );
        list.attr(
            Attribute::Custom("result_count"),
            AttrValue::String("0".to_string())
        );
        
        // Should still handle key events without panic
        let msg = list.on(Event::Keyboard(KeyEvent {
            code: Key::Enter,
            modifiers: KeyModifiers::empty(),
        }));
        
        // Enter with index 0 on empty list
        assert_eq!(msg, Some(AppMessage::EnterResultDetail(0)));
    }

    #[test]
    fn test_result_list_perform() {
        let mut list = create_result_list();
        
        // perform should always return None
        assert_eq!(list.perform(Cmd::Cancel), CmdResult::None);
        assert_eq!(list.perform(Cmd::Submit), CmdResult::None);
    }

    #[test]
    fn test_result_list_state() {
        let list = create_result_list();
        
        // State returns default
        let state = list.state();
        // Just check that state() doesn't panic
        let _ = state;
    }

    #[test]
    fn test_result_list_unknown_key() {
        let mut list = create_result_list();
        
        // Unknown function key
        let msg = list.on(Event::Keyboard(KeyEvent {
            code: Key::Function(1),
            modifiers: KeyModifiers::empty(),
        }));
        
        assert_eq!(msg, None);
    }

    #[test]
    fn test_result_list_vim_page_navigation() {
        let mut list = create_result_list();
        
        // Ctrl+B for page up
        assert_eq!(
            list.on(Event::Keyboard(KeyEvent {
                code: Key::Char('b'),
                modifiers: KeyModifiers::CONTROL,
            })),
            Some(AppMessage::ResultPageUp)
        );
        
        // Ctrl+F for page down
        assert_eq!(
            list.on(Event::Keyboard(KeyEvent {
                code: Key::Char('f'),
                modifiers: KeyModifiers::CONTROL,
            })),
            Some(AppMessage::ResultPageDown)
        );
        
        // g for home
        assert_eq!(
            list.on(Event::Keyboard(KeyEvent {
                code: Key::Char('g'),
                modifiers: KeyModifiers::empty(),
            })),
            Some(AppMessage::ResultHome)
        );
        
        // G for end
        assert_eq!(
            list.on(Event::Keyboard(KeyEvent {
                code: Key::Char('G'),
                modifiers: KeyModifiers::SHIFT,
            })),
            Some(AppMessage::ResultEnd)
        );
    }

    #[test]
    fn test_result_list_with_large_index() {
        let mut list = create_result_list();
        
        // Set a large selected index
        list.attr(Attribute::Value, AttrValue::Length(999));
        
        // Should handle Enter without panic
        let msg = list.on(Event::Keyboard(KeyEvent {
            code: Key::Enter,
            modifiers: KeyModifiers::empty(),
        }));
        
        assert_eq!(msg, Some(AppMessage::EnterResultDetail(999)));
    }
}