#[cfg(test)]
mod tests {
    use crate::interactive_iocraft::ui::{SearchState, UIState};
    use crate::query::condition::{QueryCondition, SearchResult};

    fn create_test_results(count: usize) -> Vec<SearchResult> {
        (0..count)
            .map(|i| SearchResult {
                text: format!("Result {}", i),
                file: "test.jsonl".to_string(),
                uuid: format!("uuid-{}", i),
                project_path: "/test".to_string(),
                timestamp: "2024-01-01T00:00:00Z".to_string(),
                role: "user".to_string(),
                session_id: "test-session".to_string(),
                has_thinking: false,
                has_tools: false,
                message_type: "message".to_string(),
                query: QueryCondition::Literal {
                    pattern: "test".to_string(),
                    case_sensitive: false,
                },
                raw_json: None,
            })
            .collect()
    }

    #[test]
    fn test_navigation_up_down() {
        let mut search_state = SearchState {
            results: create_test_results(10),
            selected_index: 5,
            ..Default::default()
        };

        // Test moving up
        let initial_index = search_state.selected_index;
        assert_eq!(initial_index, 5);

        // Simulate Up key - should decrease index
        if search_state.selected_index > 0 {
            search_state.selected_index -= 1;
        }
        assert_eq!(search_state.selected_index, 4);

        // Test moving down
        if search_state.selected_index < search_state.results.len().saturating_sub(1) {
            search_state.selected_index += 1;
        }
        assert_eq!(search_state.selected_index, 5);
    }

    #[test]
    fn test_navigation_boundaries() {
        let mut search_state = SearchState {
            results: create_test_results(5),
            selected_index: 0,
            ..Default::default()
        };

        // Test can't go above 0
        if search_state.selected_index > 0 {
            search_state.selected_index -= 1;
        }
        assert_eq!(search_state.selected_index, 0);

        // Test can't go beyond last item
        search_state.selected_index = 4;
        if search_state.selected_index < search_state.results.len().saturating_sub(1) {
            search_state.selected_index += 1;
        }
        assert_eq!(search_state.selected_index, 4);
    }

    #[test]
    fn test_home_end_navigation() {
        let mut search_state = SearchState {
            results: create_test_results(20),
            selected_index: 10,
            ..Default::default()
        };

        // Test Home key (Shift+Home for result list)
        search_state.selected_index = 0;
        search_state.scroll_offset = 0;
        assert_eq!(search_state.selected_index, 0);
        assert_eq!(search_state.scroll_offset, 0);

        // Test End key (Shift+End for result list)
        search_state.selected_index = search_state.results.len() - 1;
        assert_eq!(search_state.selected_index, 19);
    }

    #[test]
    fn test_page_navigation() {
        let mut search_state = SearchState {
            results: create_test_results(50),
            selected_index: 25,
            ..Default::default()
        };

        let ui_state = UIState {
            terminal_height: 30,
            ..Default::default()
        };

        // Calculate visible height
        let visible_height = ui_state.terminal_height.saturating_sub(10).max(1);
        assert_eq!(visible_height, 20);

        // Test PageUp
        let page_size = visible_height;
        search_state.selected_index = search_state.selected_index.saturating_sub(page_size);
        assert_eq!(search_state.selected_index, 5);

        // Test PageDown
        let max_index = search_state.results.len().saturating_sub(1);
        search_state.selected_index = (search_state.selected_index + page_size).min(max_index);
        assert_eq!(search_state.selected_index, 25);
    }

    #[test]
    fn test_dynamic_scroll_adjustment() {
        let search_state = SearchState {
            results: create_test_results(100),
            selected_index: 50,
            ..Default::default()
        };

        // Test scroll offset calculation
        let visible_height = 20;
        let scroll_offset = search_state.calculate_scroll_offset(visible_height);

        // Selected item should be centered when possible
        assert_eq!(scroll_offset, 40); // 50 - (20/2) = 40

        // Verify selected item is within visible range
        assert!(search_state.selected_index >= scroll_offset);
        assert!(search_state.selected_index < scroll_offset + visible_height);
    }

    #[test]
    fn test_navigation_with_empty_results() {
        let search_state = SearchState {
            results: vec![],
            selected_index: 0,
            ..Default::default()
        };

        // Should handle empty results gracefully
        assert_eq!(search_state.results.len(), 0);
        assert_eq!(search_state.selected_index, 0);
    }

    #[test]
    fn test_navigation_with_single_result() {
        let mut search_state = SearchState {
            results: create_test_results(1),
            selected_index: 0,
            ..Default::default()
        };

        // Can't move up from 0
        if search_state.selected_index > 0 {
            search_state.selected_index -= 1;
        }
        assert_eq!(search_state.selected_index, 0);

        // Can't move down from last (only) item
        if search_state.selected_index < search_state.results.len().saturating_sub(1) {
            search_state.selected_index += 1;
        }
        assert_eq!(search_state.selected_index, 0);
    }

    #[test]
    fn test_scroll_offset_at_boundaries() {
        let mut search_state = SearchState {
            results: create_test_results(30),
            selected_index: 0,
            ..Default::default()
        };

        let visible_height = 10;

        // At top, scroll offset should be 0
        let offset = search_state.calculate_scroll_offset(visible_height);
        assert_eq!(offset, 0);

        // At bottom, should show last page
        search_state.selected_index = 29;
        let offset = search_state.calculate_scroll_offset(visible_height);
        assert_eq!(offset, 20); // 30 - 10 = 20

        // In middle, should center
        search_state.selected_index = 15;
        let offset = search_state.calculate_scroll_offset(visible_height);
        assert_eq!(offset, 10); // 15 - 5 = 10
    }
}
