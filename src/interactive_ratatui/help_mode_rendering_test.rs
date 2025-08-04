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
    fn test_help_mode_renders_previous_mode() {
        let mut state = AppState::new();
        let mut renderer = Renderer::new();

        // Start in Search mode
        assert_eq!(state.mode, Mode::Search);

        // Transition to MessageDetail mode
        state.mode = Mode::MessageDetail;
        state.ui.selected_result = Some(create_mock_search_result());

        // Open Help from MessageDetail
        state.update(Message::ShowHelp);
        assert_eq!(state.mode, Mode::Help);
        assert_eq!(state.ui.mode_before_help, Some(Mode::MessageDetail));

        // Test rendering - it should render MessageDetail in background
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|f| {
                renderer.render(f, &state);
            })
            .unwrap();

        // Close Help
        state.update(Message::CloseHelp);
        assert_eq!(state.mode, Mode::MessageDetail);
        assert_eq!(state.ui.mode_before_help, None);
    }

    #[test]
    fn test_help_mode_from_session_viewer() {
        let mut state = AppState::new();

        // Start in Search mode, then go to SessionViewer
        state.mode = Mode::SessionViewer;

        // Open Help from SessionViewer
        state.update(Message::ShowHelp);
        assert_eq!(state.mode, Mode::Help);
        assert_eq!(state.ui.mode_before_help, Some(Mode::SessionViewer));

        // Close Help
        state.update(Message::CloseHelp);
        assert_eq!(state.mode, Mode::SessionViewer);
        assert_eq!(state.ui.mode_before_help, None);
    }

    #[test]
    fn test_help_mode_defaults_to_search() {
        let mut state = AppState::new();

        // Open Help from Search mode
        state.update(Message::ShowHelp);
        assert_eq!(state.mode, Mode::Help);
        assert_eq!(state.ui.mode_before_help, Some(Mode::Search));

        // Close Help
        state.update(Message::CloseHelp);
        assert_eq!(state.mode, Mode::Search);
        assert_eq!(state.ui.mode_before_help, None);
    }
}
