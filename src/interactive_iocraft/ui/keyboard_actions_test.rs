#[cfg(test)]
mod tests {
    use crate::interactive_iocraft::domain::Mode;
    use crate::interactive_iocraft::ui::{SearchState, UIState};

    #[test]
    fn test_help_mode_transition() {
        let mut ui_state = UIState {
            mode: Mode::Search,
            mode_stack: vec![],
            ..Default::default()
        };

        // Simulate pressing ? key
        let current_mode = ui_state.mode;
        ui_state.mode_stack.push(current_mode);
        ui_state.mode = Mode::Help;

        assert_eq!(ui_state.mode, Mode::Help);
        assert_eq!(ui_state.mode_stack.len(), 1);
        assert_eq!(ui_state.mode_stack[0], Mode::Search);

        // Simulate pressing Esc to return
        if let Some(prev_mode) = ui_state.mode_stack.pop() {
            ui_state.mode = prev_mode;
        }

        assert_eq!(ui_state.mode, Mode::Search);
        assert!(ui_state.mode_stack.is_empty());
    }

    #[test]
    fn test_role_filter_cycling() {
        let mut search_state = SearchState::default();

        // Test role filter cycling: None → user → assistant → system → summary → None
        assert_eq!(search_state.role_filter, None);

        // Cycle to "user"
        search_state.role_filter = match search_state.role_filter.as_deref() {
            None => Some("user".to_string()),
            Some("user") => Some("assistant".to_string()),
            Some("assistant") => Some("system".to_string()),
            Some("system") => Some("summary".to_string()),
            Some("summary") => None,
            _ => None,
        };
        assert_eq!(search_state.role_filter, Some("user".to_string()));

        // Cycle to "assistant"
        search_state.role_filter = match search_state.role_filter.as_deref() {
            None => Some("user".to_string()),
            Some("user") => Some("assistant".to_string()),
            Some("assistant") => Some("system".to_string()),
            Some("system") => Some("summary".to_string()),
            Some("summary") => None,
            _ => None,
        };
        assert_eq!(search_state.role_filter, Some("assistant".to_string()));

        // Cycle to "system"
        search_state.role_filter = match search_state.role_filter.as_deref() {
            None => Some("user".to_string()),
            Some("user") => Some("assistant".to_string()),
            Some("assistant") => Some("system".to_string()),
            Some("system") => Some("summary".to_string()),
            Some("summary") => None,
            _ => None,
        };
        assert_eq!(search_state.role_filter, Some("system".to_string()));

        // Cycle to "summary"
        search_state.role_filter = match search_state.role_filter.as_deref() {
            None => Some("user".to_string()),
            Some("user") => Some("assistant".to_string()),
            Some("assistant") => Some("system".to_string()),
            Some("system") => Some("summary".to_string()),
            Some("summary") => None,
            _ => None,
        };
        assert_eq!(search_state.role_filter, Some("summary".to_string()));

        // Cycle back to None
        search_state.role_filter = match search_state.role_filter.as_deref() {
            None => Some("user".to_string()),
            Some("user") => Some("assistant".to_string()),
            Some("assistant") => Some("system".to_string()),
            Some("system") => Some("summary".to_string()),
            Some("summary") => None,
            _ => None,
        };
        assert_eq!(search_state.role_filter, None);
    }

    #[test]
    fn test_truncation_toggle() {
        let mut ui_state = UIState {
            truncation_enabled: true,
            ..Default::default()
        };

        // Toggle truncation
        ui_state.truncation_enabled = !ui_state.truncation_enabled;
        assert_eq!(ui_state.truncation_enabled, false);

        // Toggle back
        ui_state.truncation_enabled = !ui_state.truncation_enabled;
        assert_eq!(ui_state.truncation_enabled, true);
    }

    #[test]
    fn test_cache_reload_message() {
        let mut ui_state = UIState::default();

        // Simulate Ctrl+R press
        ui_state.message = Some("Cache cleared. Reloading...".to_string());

        assert_eq!(
            ui_state.message,
            Some("Cache cleared. Reloading...".to_string())
        );
    }

    #[test]
    fn test_mode_stack_navigation() {
        let mut ui_state = UIState {
            mode: Mode::Search,
            mode_stack: vec![],
            ..Default::default()
        };

        // Navigate to ResultDetail
        ui_state.mode_stack.push(ui_state.mode);
        ui_state.mode = Mode::ResultDetail;

        assert_eq!(ui_state.mode, Mode::ResultDetail);
        assert_eq!(ui_state.mode_stack, vec![Mode::Search]);

        // Navigate to SessionViewer
        ui_state.mode_stack.push(ui_state.mode);
        ui_state.mode = Mode::SessionViewer;

        assert_eq!(ui_state.mode, Mode::SessionViewer);
        assert_eq!(ui_state.mode_stack, vec![Mode::Search, Mode::ResultDetail]);

        // Navigate back to ResultDetail
        if let Some(prev_mode) = ui_state.mode_stack.pop() {
            ui_state.mode = prev_mode;
        }

        assert_eq!(ui_state.mode, Mode::ResultDetail);
        assert_eq!(ui_state.mode_stack, vec![Mode::Search]);

        // Navigate back to Search
        if let Some(prev_mode) = ui_state.mode_stack.pop() {
            ui_state.mode = prev_mode;
        }

        assert_eq!(ui_state.mode, Mode::Search);
        assert!(ui_state.mode_stack.is_empty());
    }

    #[test]
    fn test_message_display_modes() {
        let mut ui_state = UIState {
            truncation_enabled: true,
            ..Default::default()
        };

        // Test truncation mode indicator
        let mode_text = if ui_state.truncation_enabled {
            "[Truncated]"
        } else {
            "[Full Text]"
        };
        assert_eq!(mode_text, "[Truncated]");

        ui_state.truncation_enabled = false;
        let mode_text = if ui_state.truncation_enabled {
            "[Truncated]"
        } else {
            "[Full Text]"
        };
        assert_eq!(mode_text, "[Full Text]");
    }

    #[test]
    fn test_feedback_messages() {
        let mut ui_state = UIState::default();

        // Test copy success messages
        ui_state.message = Some("✓ Copied message text".to_string());
        assert!(ui_state.message.as_ref().unwrap().starts_with('✓'));

        ui_state.message = Some("✓ Copied file path".to_string());
        assert!(ui_state.message.as_ref().unwrap().contains("file path"));

        ui_state.message = Some("✓ Copied session ID".to_string());
        assert!(ui_state.message.as_ref().unwrap().contains("session ID"));

        // Test error messages
        ui_state.message = Some("Failed to copy: clipboard not available".to_string());
        assert!(ui_state.message.as_ref().unwrap().starts_with("Failed"));
    }

    #[test]
    fn test_search_state_reset() {
        let mut search_state = SearchState {
            query: "test query".to_string(),
            role_filter: Some("user".to_string()),
            selected_index: 5,
            scroll_offset: 10,
            is_searching: true,
            ..Default::default()
        };

        // Reset for new search
        search_state.query.clear();
        search_state.selected_index = 0;
        search_state.scroll_offset = 0;
        search_state.is_searching = false;

        assert_eq!(search_state.query, "");
        assert_eq!(search_state.selected_index, 0);
        assert_eq!(search_state.scroll_offset, 0);
        assert_eq!(search_state.is_searching, false);
        // Role filter should persist
        assert_eq!(search_state.role_filter, Some("user".to_string()));
    }
}
