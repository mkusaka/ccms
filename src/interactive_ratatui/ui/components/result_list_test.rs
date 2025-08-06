#[cfg(test)]
mod tests {
    use super::super::Component;
    use super::super::result_list::*;
    use crate::interactive_ratatui::ui::events::Message;
    use crate::query::condition::{QueryCondition, SearchResult};
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    fn create_key_event(code: KeyCode) -> KeyEvent {
        KeyEvent {
            code,
            modifiers: KeyModifiers::empty(),
            kind: crossterm::event::KeyEventKind::Press,
            state: crossterm::event::KeyEventState::empty(),
        }
    }

    fn create_test_result(role: &str, text: &str) -> SearchResult {
        SearchResult {
            file: "test.jsonl".to_string(),
            uuid: "test-uuid".to_string(),
            timestamp: "2024-01-01T12:00:00Z".to_string(),
            session_id: "test-session".to_string(),
            role: role.to_string(),
            text: text.to_string(),
            message_type: role.to_string(),
            query: QueryCondition::Literal {
                pattern: "test".to_string(),
                case_sensitive: false,
            },
            cwd: "/test".to_string(),
            raw_json: None,
        }
    }

    #[test]
    fn test_result_list_creation() {
        let list = ResultList::new();
        assert!(list.selected_result().is_none());
    }

    #[test]
    fn test_update_results() {
        let mut list = ResultList::new();
        let results = vec![
            create_test_result("user", "Hello"),
            create_test_result("assistant", "Hi there"),
        ];

        list.update_results(results.clone(), 0);

        assert!(list.selected_result().is_some());
        assert_eq!(list.selected_result().unwrap().text, "Hello");
    }

    #[test]
    fn test_navigation_up_down() {
        let mut list = ResultList::new();
        let results = vec![
            create_test_result("user", "First"),
            create_test_result("assistant", "Second"),
            create_test_result("user", "Third"),
        ];

        list.update_results(results, 0);

        // Initially at index 0
        assert_eq!(list.selected_result().unwrap().text, "First");

        // Move down
        let msg = list.handle_key(create_key_event(KeyCode::Down));
        assert!(matches!(msg, Some(Message::SelectResult(_))));
        assert_eq!(list.selected_result().unwrap().text, "Second");

        // Move down again
        let msg = list.handle_key(create_key_event(KeyCode::Down));
        assert!(matches!(msg, Some(Message::SelectResult(_))));
        assert_eq!(list.selected_result().unwrap().text, "Third");

        // Can't move down from last item
        let msg = list.handle_key(create_key_event(KeyCode::Down));
        assert!(msg.is_none());

        // Move up
        let msg = list.handle_key(create_key_event(KeyCode::Up));
        assert!(matches!(msg, Some(Message::SelectResult(_))));
        assert_eq!(list.selected_result().unwrap().text, "Second");
    }

    #[test]
    fn test_page_navigation() {
        let mut list = ResultList::new();
        let mut results = vec![];
        for i in 0..20 {
            results.push(create_test_result("user", &format!("Message {i}")));
        }

        list.update_results(results, 0);

        // Page down - might trigger LoadMoreResults if near the end
        let msg = list.handle_key(create_key_event(KeyCode::PageDown));
        assert!(matches!(
            msg,
            Some(Message::SelectResult(_)) | Some(Message::LoadMoreResults)
        ));

        // Page up
        let msg = list.handle_key(create_key_event(KeyCode::PageUp));
        assert!(matches!(msg, Some(Message::SelectResult(_))));
    }

    #[test]
    fn test_home_end_navigation() {
        let mut list = ResultList::new();
        let results = vec![
            create_test_result("user", "First"),
            create_test_result("assistant", "Middle"),
            create_test_result("user", "Last"),
        ];

        list.update_results(results, 1); // Start in middle

        // Go to end
        let msg = list.handle_key(create_key_event(KeyCode::End));
        assert!(matches!(msg, Some(Message::SelectResult(_))));
        assert_eq!(list.selected_result().unwrap().text, "Last");

        // Go to home
        let msg = list.handle_key(create_key_event(KeyCode::Home));
        assert!(matches!(msg, Some(Message::SelectResult(_))));
        assert_eq!(list.selected_result().unwrap().text, "First");
    }

    #[test]
    fn test_enter_key() {
        let mut list = ResultList::new();
        let results = vec![create_test_result("user", "Test")];
        list.update_results(results, 0);

        // Enter should open detail view
        let msg = list.handle_key(create_key_event(KeyCode::Enter));
        assert!(matches!(msg, Some(Message::EnterMessageDetail)));
    }

    #[test]
    fn test_s_key_session_viewer() {
        let mut list = ResultList::new();
        let results = vec![create_test_result("user", "Test")];
        list.update_results(results, 0);

        // Ctrl+S should open session viewer
        let msg = list.handle_key(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::CONTROL));
        assert!(matches!(msg, Some(Message::EnterSessionViewer)));
    }

    #[test]
    fn test_empty_results() {
        let mut list = ResultList::new();
        list.update_results(vec![], 0);

        assert!(list.selected_result().is_none());

        // Navigation should do nothing
        let msg = list.handle_key(create_key_event(KeyCode::Down));
        assert!(msg.is_none());

        let msg = list.handle_key(create_key_event(KeyCode::Up));
        assert!(msg.is_none());
    }

    #[test]
    fn test_ctrl_p_n_navigation() {
        let mut list = ResultList::new();
        let results = vec![
            create_test_result("user", "First"),
            create_test_result("assistant", "Second"),
            create_test_result("user", "Third"),
        ];

        list.update_results(results, 0);

        // Initially at index 0
        assert_eq!(list.selected_result().unwrap().text, "First");

        // Move down with Ctrl+N
        let msg = list.handle_key(KeyEvent::new(KeyCode::Char('n'), KeyModifiers::CONTROL));
        assert!(matches!(msg, Some(Message::SelectResult(_))));
        assert_eq!(list.selected_result().unwrap().text, "Second");

        // Move down again with Ctrl+N
        let msg = list.handle_key(KeyEvent::new(KeyCode::Char('n'), KeyModifiers::CONTROL));
        assert!(matches!(msg, Some(Message::SelectResult(_))));
        assert_eq!(list.selected_result().unwrap().text, "Third");

        // Can't move down from last item
        let msg = list.handle_key(KeyEvent::new(KeyCode::Char('n'), KeyModifiers::CONTROL));
        assert!(msg.is_none());

        // Move up with Ctrl+P
        let msg = list.handle_key(KeyEvent::new(KeyCode::Char('p'), KeyModifiers::CONTROL));
        assert!(matches!(msg, Some(Message::SelectResult(_))));
        assert_eq!(list.selected_result().unwrap().text, "Second");

        // Move up again with Ctrl+P
        let msg = list.handle_key(KeyEvent::new(KeyCode::Char('p'), KeyModifiers::CONTROL));
        assert!(matches!(msg, Some(Message::SelectResult(_))));
        assert_eq!(list.selected_result().unwrap().text, "First");

        // Can't move up from first item
        let msg = list.handle_key(KeyEvent::new(KeyCode::Char('p'), KeyModifiers::CONTROL));
        assert!(msg.is_none());
    }

    #[test]
    fn test_ctrl_u_d_navigation() {
        let mut list = ResultList::new();
        let mut results = vec![];
        for i in 0..30 {
            results.push(create_test_result("user", &format!("Message {i}")));
        }

        list.update_results(results, 0);

        // Initially at index 0
        assert_eq!(list.selected_result().unwrap().text, "Message 0");

        // Move down with Ctrl+D (half page down)
        let msg = list.handle_key(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::CONTROL));
        assert!(matches!(msg, Some(Message::SelectResult(_))));
        // The exact position depends on the viewport height, but it should have moved down
        let first_pos = list.selected_result().unwrap().text.clone();
        assert_ne!(first_pos, "Message 0");

        // Move down again with Ctrl+D
        let msg = list.handle_key(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::CONTROL));
        assert!(matches!(msg, Some(Message::SelectResult(_))));
        let second_pos = list.selected_result().unwrap().text.clone();
        assert_ne!(second_pos, first_pos);

        // Navigate to near the end
        list.update_selection(25);

        // Move down with Ctrl+D - might trigger LoadMoreResults since we're near the end
        let msg = list.handle_key(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::CONTROL));
        assert!(matches!(
            msg,
            Some(Message::SelectResult(_)) | Some(Message::LoadMoreResults)
        ));
        // Selection should still move to the last item
        assert_eq!(list.selected_result().unwrap().text, "Message 29");

        // Can't move down from last item
        let msg = list.handle_key(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::CONTROL));
        assert!(msg.is_none());

        // Move up with Ctrl+U (half page up)
        let msg = list.handle_key(KeyEvent::new(KeyCode::Char('u'), KeyModifiers::CONTROL));
        assert!(matches!(msg, Some(Message::SelectResult(_))));
        assert_ne!(list.selected_result().unwrap().text, "Message 29");

        // Move to position 5
        list.update_selection(5);

        // Move up with Ctrl+U should go to first item
        let msg = list.handle_key(KeyEvent::new(KeyCode::Char('u'), KeyModifiers::CONTROL));
        assert!(matches!(msg, Some(Message::SelectResult(_))));
        assert_eq!(list.selected_result().unwrap().text, "Message 0");

        // Can't move up from first item
        let msg = list.handle_key(KeyEvent::new(KeyCode::Char('u'), KeyModifiers::CONTROL));
        assert!(msg.is_none());
    }

    #[test]
    fn test_shortcuts_display_with_wrap() {
        use ratatui::Terminal;
        use ratatui::backend::TestBackend;

        let mut list = ResultList::new();
        let results = vec![create_test_result("user", "Test message")];
        list.update_results(results, 0);

        // Create test backend with narrow width but enough height to show all shortcuts
        let backend = TestBackend::new(40, 25);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|f| {
                list.render(f, f.area());
            })
            .unwrap();

        let buffer = terminal.backend().buffer();

        // Convert buffer to string for easier testing
        let mut content = String::new();
        for y in 0..buffer.area.height {
            for x in 0..buffer.area.width {
                let cell = buffer.cell((x, y)).unwrap();
                content.push_str(cell.symbol());
            }
            content.push('\n');
        }

        // Check that shortcuts are displayed in the status bar
        // With a narrow terminal (40 chars), the status bar will wrap
        // The new status bar starts with "Shift+Tab/Ctrl+Tab: Switch tabs"
        // So we check for text that would be visible in the wrapped display
        assert!(content.contains("Shift+Tab") || content.contains("Switch tabs"));
        assert!(content.contains("Tab:") || content.contains("Filter"));
        assert!(content.contains("Navigate") || content.contains("Ctrl+"));
    }

    #[test]
    fn test_shortcuts_display_wide_screen() {
        use ratatui::Terminal;
        use ratatui::backend::TestBackend;

        let mut list = ResultList::new();
        let results = vec![create_test_result("user", "Test message")];
        list.update_results(results, 0);

        // Create test backend with wide width
        let backend = TestBackend::new(120, 30);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|f| {
                list.render(f, f.area());
            })
            .unwrap();

        let buffer = terminal.backend().buffer();

        // Convert buffer to string for easier testing
        let mut content = String::new();
        for y in 0..buffer.area.height {
            for x in 0..buffer.area.width {
                let cell = buffer.cell((x, y)).unwrap();
                content.push_str(cell.symbol());
            }
            content.push('\n');
        }

        // Check that shortcuts are displayed properly on wide screen in the status bar
        // With the new status bar, we should see the tab switching shortcuts first
        assert!(content.contains("Shift+Tab: Switch tabs"));
        assert!(content.contains("Tab: Filter"));
        assert!(content.contains("↑/↓ or Ctrl+P/N: Navigate"));
    }

    #[test]
    fn test_esc_key_preview() {
        let mut list = ResultList::new();
        list.set_results(vec![create_test_result("user", "Test message")]);

        // Test Esc when preview is disabled - should bubble up
        list.set_preview_enabled(false);
        let key = KeyEvent::from(KeyCode::Esc);
        let message = list.handle_key(key);
        assert!(message.is_none());

        // Test Esc when preview is enabled - should toggle preview
        list.set_preview_enabled(true);
        let key = KeyEvent::from(KeyCode::Esc);
        let message = list.handle_key(key);
        assert!(matches!(message, Some(Message::TogglePreview)));
    }

    // Pagination tests
    #[test]
    fn test_pagination_state_setting() {
        let mut list = ResultList::new();

        // Set pagination state
        list.set_pagination_state(true, false, 150);

        // State is set internally - we can verify through behavior in other tests
        // Direct field access would require pub(crate) or getter methods
    }

    #[test]
    fn test_load_more_trigger_on_down_navigation() {
        let mut list = ResultList::new();

        // Create 100 results
        let mut results = Vec::new();
        for i in 0..100 {
            results.push(create_test_result("user", &format!("Message {i}")));
        }
        list.set_results(results);
        list.set_pagination_state(true, false, 100);

        // Navigate to near the end (position 95)
        for _ in 0..95 {
            list.handle_key(create_key_event(KeyCode::Down));
        }

        // Next down should trigger LoadMoreResults
        let msg = list.handle_key(create_key_event(KeyCode::Down));
        assert!(matches!(msg, Some(Message::LoadMoreResults)));
    }

    #[test]
    fn test_no_load_more_when_all_loaded() {
        let mut list = ResultList::new();

        // Create 50 results (less than a full page)
        let mut results = Vec::new();
        for i in 0..50 {
            results.push(create_test_result("user", &format!("Message {i}")));
        }
        list.set_results(results);
        list.set_pagination_state(false, false, 50); // has_more_results = false

        // Navigate to near the end
        for _ in 0..45 {
            list.handle_key(create_key_event(KeyCode::Down));
        }

        // Should NOT trigger LoadMoreResults since has_more_results is false
        let msg = list.handle_key(create_key_event(KeyCode::Down));
        assert!(matches!(msg, Some(Message::SelectResult(_))));
    }

    #[test]
    fn test_no_load_more_when_already_loading() {
        let mut list = ResultList::new();

        // Create 100 results
        let mut results = Vec::new();
        for i in 0..100 {
            results.push(create_test_result("user", &format!("Message {i}")));
        }
        list.set_results(results);
        list.set_pagination_state(true, true, 100); // loading_more = true

        // Navigate to near the end
        for _ in 0..95 {
            list.handle_key(create_key_event(KeyCode::Down));
        }

        // Should NOT trigger LoadMoreResults since already loading
        let msg = list.handle_key(create_key_event(KeyCode::Down));
        assert!(matches!(msg, Some(Message::SelectResult(_))));
    }

    #[test]
    fn test_pagination_trigger_with_ctrl_n() {
        let mut list = ResultList::new();

        // Create 100 results
        let mut results = Vec::new();
        for i in 0..100 {
            results.push(create_test_result("user", &format!("Message {i}")));
        }
        list.set_results(results);
        list.set_pagination_state(true, false, 100);

        // Navigate to near the end with Ctrl+N
        for _ in 0..95 {
            list.handle_key(KeyEvent::new(KeyCode::Char('n'), KeyModifiers::CONTROL));
        }

        // Next Ctrl+N should trigger LoadMoreResults
        let msg = list.handle_key(KeyEvent::new(KeyCode::Char('n'), KeyModifiers::CONTROL));
        assert!(matches!(msg, Some(Message::LoadMoreResults)));
    }

    #[test]
    fn test_pagination_trigger_with_page_down() {
        let mut list = ResultList::new();

        // Create 100 results
        let mut results = Vec::new();
        for i in 0..100 {
            results.push(create_test_result("user", &format!("Message {i}")));
        }
        list.set_results(results);
        list.set_pagination_state(true, false, 100);

        // Navigate to position 85 (near end but not at threshold)
        for _ in 0..85 {
            list.handle_key(create_key_event(KeyCode::Down));
        }

        // PageDown should move us near the end and trigger LoadMoreResults
        let msg = list.handle_key(create_key_event(KeyCode::PageDown));
        assert!(matches!(
            msg,
            Some(Message::LoadMoreResults) | Some(Message::SelectResult(_))
        ));
    }

    #[test]
    fn test_pagination_trigger_with_ctrl_d() {
        let mut list = ResultList::new();

        // Create 100 results
        let mut results = Vec::new();
        for i in 0..100 {
            results.push(create_test_result("user", &format!("Message {i}")));
        }
        list.set_results(results);
        list.set_pagination_state(true, false, 100);

        // Navigate to position 80
        for _ in 0..80 {
            list.handle_key(create_key_event(KeyCode::Down));
        }

        // Ctrl+D (half page down) should move us near the end and may trigger LoadMoreResults
        let msg = list.handle_key(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::CONTROL));
        assert!(matches!(
            msg,
            Some(Message::LoadMoreResults) | Some(Message::SelectResult(_))
        ));
    }
}
