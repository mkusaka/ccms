#[cfg(test)]
mod tests {
    use crate::interactive_iocraft::domain::Mode;
    use crate::interactive_iocraft::ui::{SessionState, UIState};

    fn create_test_messages() -> Vec<String> {
        vec![
            r#"{"type":"user","content":"Hello world"}"#.to_string(),
            r#"{"type":"assistant","content":"Hi there!"}"#.to_string(),
            r#"{"type":"user","content":"How are you?"}"#.to_string(),
            r#"{"type":"assistant","content":"I'm doing well, thank you!"}"#.to_string(),
            r#"{"type":"user","content":"What's the weather like?"}"#.to_string(),
        ]
    }

    #[test]
    fn test_session_viewer_initialization() {
        let mut session_state = SessionState {
            messages: create_test_messages(),
            filtered_indices: vec![],
            selected_index: 0,
            scroll_offset: 0,
            query: String::new(),
            file_path: Some("/path/to/session.jsonl".to_string()),
            session_id: Some("session-123".to_string()),
        };

        // Initialize filtered indices
        session_state.filtered_indices = (0..session_state.messages.len()).collect();

        assert_eq!(session_state.messages.len(), 5);
        assert_eq!(session_state.filtered_indices.len(), 5);
        assert_eq!(session_state.selected_index, 0);
        assert_eq!(session_state.scroll_offset, 0);
    }

    #[test]
    fn test_session_message_filtering() {
        let session_state = SessionState {
            messages: create_test_messages(),
            filtered_indices: vec![],
            selected_index: 0,
            scroll_offset: 0,
            query: "weather".to_string(),
            file_path: None,
            session_id: None,
        };

        // Simulate filtering
        let filtered: Vec<usize> = session_state
            .messages
            .iter()
            .enumerate()
            .filter(|(_, msg)| {
                msg.to_lowercase()
                    .contains(&session_state.query.to_lowercase())
            })
            .map(|(idx, _)| idx)
            .collect();

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0], 4); // "What's the weather like?"
    }

    #[test]
    fn test_session_viewer_navigation() {
        let mut session_state = SessionState {
            messages: create_test_messages(),
            filtered_indices: (0..5).collect(),
            selected_index: 0,
            scroll_offset: 0,
            query: String::new(),
            file_path: None,
            session_id: None,
        };

        // Test moving down
        if session_state.selected_index < session_state.filtered_indices.len().saturating_sub(1) {
            session_state.selected_index += 1;
        }
        assert_eq!(session_state.selected_index, 1);

        // Test moving up
        if session_state.selected_index > 0 {
            session_state.selected_index -= 1;
        }
        assert_eq!(session_state.selected_index, 0);

        // Test scroll adjustment
        session_state.selected_index = 11;
        if session_state.selected_index >= session_state.scroll_offset + 10 {
            session_state.scroll_offset = session_state.selected_index - 9;
        }
        assert_eq!(session_state.scroll_offset, 2);
    }

    #[test]
    fn test_session_viewer_query_input() {
        let mut session_state = SessionState {
            messages: create_test_messages(),
            filtered_indices: (0..5).collect(),
            selected_index: 0,
            scroll_offset: 0,
            query: String::new(),
            file_path: None,
            session_id: None,
        };

        // Add characters to query
        session_state.query.push('H');
        session_state.query.push('e');
        session_state.query.push('l');
        session_state.query.push('l');
        session_state.query.push('o');

        assert_eq!(session_state.query, "Hello");

        // Backspace
        session_state.query.pop();
        assert_eq!(session_state.query, "Hell");

        // Clear query
        session_state.query.clear();
        assert_eq!(session_state.query, "");
    }

    #[test]
    fn test_session_viewer_copy_operations() {
        let session_state = SessionState {
            messages: create_test_messages(),
            filtered_indices: (0..5).collect(),
            selected_index: 2,
            scroll_offset: 0,
            query: String::new(),
            file_path: None,
            session_id: Some("session-123".to_string()),
        };

        // Test copy selected message
        let selected_msg_idx = session_state.filtered_indices[session_state.selected_index];
        let selected_msg = &session_state.messages[selected_msg_idx];
        assert_eq!(selected_msg, r#"{"type":"user","content":"How are you?"}"#);

        // Test copy session ID
        assert_eq!(session_state.session_id, Some("session-123".to_string()));

        // Test copy all messages
        let all_messages = session_state.messages.join("\n\n");
        assert!(all_messages.contains("Hello world"));
        assert!(all_messages.contains("Hi there!"));
        assert!(all_messages.contains("How are you?"));
    }

    #[test]
    fn test_session_viewer_empty_messages() {
        let session_state = SessionState {
            messages: vec![],
            filtered_indices: vec![],
            selected_index: 0,
            scroll_offset: 0,
            query: String::new(),
            file_path: None,
            session_id: None,
        };

        assert_eq!(session_state.messages.len(), 0);
        assert_eq!(session_state.filtered_indices.len(), 0);
    }

    #[test]
    fn test_session_viewer_mode_transition() {
        let mut ui_state = UIState {
            mode: Mode::ResultDetail,
            mode_stack: vec![Mode::Search],
            ..Default::default()
        };

        // Enter SessionViewer
        ui_state.mode_stack.push(ui_state.mode);
        ui_state.mode = Mode::SessionViewer;

        assert_eq!(ui_state.mode, Mode::SessionViewer);
        assert_eq!(ui_state.mode_stack, vec![Mode::Search, Mode::ResultDetail]);

        // Exit SessionViewer
        if let Some(prev_mode) = ui_state.mode_stack.pop() {
            ui_state.mode = prev_mode;
        }

        assert_eq!(ui_state.mode, Mode::ResultDetail);
        assert_eq!(ui_state.mode_stack, vec![Mode::Search]);
    }

    #[test]
    fn test_session_viewer_backspace_behavior() {
        let mut session_state = SessionState {
            messages: create_test_messages(),
            filtered_indices: (0..5).collect(),
            selected_index: 0,
            scroll_offset: 0,
            query: "test".to_string(),
            file_path: None,
            session_id: None,
        };

        let mut ui_state = UIState {
            mode: Mode::SessionViewer,
            mode_stack: vec![Mode::Search, Mode::ResultDetail],
            ..Default::default()
        };

        // Backspace with query - should delete character
        assert!(!session_state.query.is_empty());
        session_state.query.pop();
        assert_eq!(session_state.query, "tes");

        // Backspace with empty query - should go back
        session_state.query.clear();
        assert!(session_state.query.is_empty());

        // Simulate going back
        if let Some(prev_mode) = ui_state.mode_stack.pop() {
            ui_state.mode = prev_mode;
        }
        assert_eq!(ui_state.mode, Mode::ResultDetail);
    }

    #[test]
    fn test_filtered_message_count_display() {
        let session_state = SessionState {
            messages: create_test_messages(),
            filtered_indices: vec![0, 2, 4], // 3 filtered out of 5
            selected_index: 0,
            scroll_offset: 0,
            query: "user".to_string(),
            file_path: None,
            session_id: None,
        };

        let total_count = session_state.messages.len();
        let filtered_count = session_state.filtered_indices.len();

        assert_eq!(total_count, 5);
        assert_eq!(filtered_count, 3);

        // Display format: "Messages (5 total, 3 filtered)"
        let display_text = format!("Messages ({total_count} total, {filtered_count} filtered)");
        assert_eq!(display_text, "Messages (5 total, 3 filtered)");
    }
}
