#[cfg(test)]
mod tests {
    use crate::interactive_ratatui::domain::models::Mode;
    use crate::interactive_ratatui::ui::app_state::AppState;
    use crate::interactive_ratatui::ui::events::Message;
    use crate::interactive_ratatui::ui::renderer::Renderer;
    use crate::query::condition::{QueryCondition, SearchResult};
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    fn create_mock_search_result() -> SearchResult {
        SearchResult {
            file: "test.jsonl".to_string(),
            uuid: "test-uuid".to_string(),
            timestamp: "2024-08-04".to_string(),
            session_id: "test-session".to_string(),
            role: "user".to_string(),
            text: "test message".to_string(),
            message_type: "text".to_string(),
            query: QueryCondition::Literal {
                pattern: "test".to_string(),
                case_sensitive: false,
            },
            cwd: "/test".to_string(),
            raw_json: None,
        }
    }

    #[test]
    fn test_help_overlay_from_search_mode() {
        let mut state = AppState::new();

        // Start in Search mode
        assert_eq!(state.mode, Mode::Search);
        assert!(!state.ui.show_help);

        // Open Help overlay
        state.update(Message::ShowHelp);
        assert_eq!(state.mode, Mode::Search); // Mode should not change
        assert!(state.ui.show_help);

        // Close Help overlay
        state.update(Message::CloseHelp);
        assert_eq!(state.mode, Mode::Search); // Mode should remain the same
        assert!(!state.ui.show_help);
    }

    #[test]
    fn test_help_overlay_from_message_detail() {
        let mut state = AppState::new();
        let mut renderer = Renderer::new();

        // Start in Search mode, then go to MessageDetail
        state.mode = Mode::MessageDetail;
        state.ui.selected_result = Some(create_mock_search_result());
        assert!(!state.ui.show_help);

        // Open Help overlay from MessageDetail
        state.update(Message::ShowHelp);
        assert_eq!(state.mode, Mode::MessageDetail); // Mode should not change
        assert!(state.ui.show_help);

        // Test rendering - MessageDetail should be rendered in background
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|f| {
                renderer.render(f, &state);
            })
            .unwrap();

        // Close Help overlay
        state.update(Message::CloseHelp);
        assert_eq!(state.mode, Mode::MessageDetail); // Should return to MessageDetail
        assert!(!state.ui.show_help);
    }

    #[test]
    fn test_help_overlay_from_session_viewer() {
        let mut state = AppState::new();

        // Start in SessionViewer mode
        state.mode = Mode::SessionViewer;
        assert!(!state.ui.show_help);

        // Open Help overlay from SessionViewer
        state.update(Message::ShowHelp);
        assert_eq!(state.mode, Mode::SessionViewer); // Mode should not change
        assert!(state.ui.show_help);

        // Close Help overlay
        state.update(Message::CloseHelp);
        assert_eq!(state.mode, Mode::SessionViewer); // Should remain in SessionViewer
        assert!(!state.ui.show_help);
    }

    #[test]
    fn test_help_overlay_rendering() {
        let mut state = AppState::new();
        let mut renderer = Renderer::new();

        // Test that help overlay is rendered on top of current mode
        state.ui.show_help = true;

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        // Render with help overlay active
        terminal
            .draw(|f| {
                renderer.render(f, &state);
            })
            .unwrap();

        // The help dialog should be rendered on top
        // (Actual content verification would require parsing the buffer)
    }
}
