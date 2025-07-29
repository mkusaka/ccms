#[cfg(test)]
mod tests {
    use super::super::list_item::{ListItem, truncate_message, wrap_text};
    use super::super::list_viewer::ListViewer;
    use ratatui::text::Line;

    // Mock implementation of ListItem for testing
    #[derive(Clone)]
    struct MockListItem {
        role: String,
        timestamp: String,
        content: String,
    }

    impl ListItem for MockListItem {
        fn get_role(&self) -> &str {
            &self.role
        }

        fn get_timestamp(&self) -> &str {
            &self.timestamp
        }

        fn get_content(&self) -> &str {
            &self.content
        }

        fn create_truncated_line(&self, max_width: usize, _query: &str) -> Line<'static> {
            let content = truncate_message(self.get_content(), max_width);
            Line::from(content)
        }

        fn create_full_lines(&self, max_width: usize, _query: &str) -> Vec<Line<'static>> {
            let wrapped_lines = wrap_text(self.get_content(), max_width);
            wrapped_lines
                .into_iter()
                .map(|s| Line::from(s.to_string()))
                .collect()
        }
    }

    fn create_mock_items(count: usize) -> Vec<MockListItem> {
        (0..count)
            .map(|i| MockListItem {
                role: if i % 2 == 0 { "user" } else { "assistant" }.to_string(),
                timestamp: format!("2024-01-{:02}T12:00:00Z", i + 1),
                content: format!("Message content #{}", i + 1),
            })
            .collect()
    }

    #[test]
    fn test_scrolling_after_filtering_reproduction() {
        // This test reproduces the issue described in issue #58
        let mut viewer = ListViewer::<MockListItem>::new("Test".to_string(), "Empty".to_string());

        // Create items with longer content to ensure they take multiple lines in full text mode
        let items: Vec<MockListItem> = (0..20)
            .map(|i| MockListItem {
                role: if i % 2 == 0 { "user" } else { "assistant" }.to_string(),
                timestamp: format!("2024-01-{:02}T12:00:00Z", i + 1),
                content: format!("This is a longer message content #{} that will wrap to multiple lines when displayed in full text mode. This ensures that each message takes up more vertical space.", i + 1),
            })
            .collect();
        viewer.set_items(items);

        // Apply a filter that shows items 0, 10, 15 (indices in the filtered list: 0, 1, 2)
        viewer.set_filtered_indices(vec![0, 10, 15]);

        // Test scrolling in full text mode (non-truncated) - this is where the bug occurs
        viewer.truncation_enabled = false;
        viewer.scroll_offset = 0;

        // Select the last filtered item (index 2, which is item 15)
        viewer.selected_index = 2;

        // With small available height, not all items can be visible at once
        let available_height = 4; // Small height to force scrolling
        let available_width = 80;

        // Store initial scroll offset
        let initial_scroll_offset = viewer.scroll_offset;

        // This is where the bug occurs - adjust_scroll_offset should update scroll_offset
        // but due to the bug, it won't properly adjust
        viewer.adjust_scroll_offset(available_height, available_width);

        // The scroll offset should be adjusted to make item at index 2 visible
        // With the bug, this assertion will fail because scroll_offset remains 0
        assert!(
            viewer.scroll_offset > initial_scroll_offset,
            "Scroll offset should be adjusted from {} to make selected item visible, but it remained at {}",
            initial_scroll_offset,
            viewer.scroll_offset
        );
    }

    #[test]
    fn test_basic_scrolling_truncated_mode() {
        let mut viewer = ListViewer::<MockListItem>::new("Test".to_string(), "Empty".to_string());
        let items = create_mock_items(10);
        viewer.set_items(items);
        viewer.truncation_enabled = true;

        let available_height = 5;
        let available_width = 80;

        // Test scrolling down
        viewer.selected_index = 7;
        viewer.adjust_scroll_offset(available_height, available_width);
        assert_eq!(
            viewer.scroll_offset, 3,
            "Scroll offset should be 3 to show item 7 at the bottom"
        );

        // Test scrolling up
        viewer.selected_index = 1;
        viewer.adjust_scroll_offset(available_height, available_width);
        assert_eq!(
            viewer.scroll_offset, 1,
            "Scroll offset should be 1 to show item 1 at the top"
        );
    }

    #[test]
    fn test_move_operations() {
        let mut viewer = ListViewer::<MockListItem>::new("Test".to_string(), "Empty".to_string());
        let items = create_mock_items(5);
        viewer.set_items(items);

        // Test move_down
        assert_eq!(viewer.selected_index, 0);
        assert!(viewer.move_down());
        assert_eq!(viewer.selected_index, 1);

        // Test move_up
        assert!(viewer.move_up());
        assert_eq!(viewer.selected_index, 0);

        // Test move_up at start
        assert!(!viewer.move_up());
        assert_eq!(viewer.selected_index, 0);

        // Test move_to_end
        assert!(viewer.move_to_end());
        assert_eq!(viewer.selected_index, 4);

        // Test move_down at end
        assert!(!viewer.move_down());
        assert_eq!(viewer.selected_index, 4);

        // Test move_to_start
        assert!(viewer.move_to_start());
        assert_eq!(viewer.selected_index, 0);
        assert_eq!(viewer.scroll_offset, 0);
    }

    #[test]
    fn test_page_navigation() {
        let mut viewer = ListViewer::<MockListItem>::new("Test".to_string(), "Empty".to_string());
        let items = create_mock_items(25);
        viewer.set_items(items);

        // Test page_down
        assert!(viewer.page_down());
        assert_eq!(viewer.selected_index, 10);

        assert!(viewer.page_down());
        assert_eq!(viewer.selected_index, 20);

        assert!(viewer.page_down());
        assert_eq!(viewer.selected_index, 24); // Last item

        // Test page_up
        assert!(viewer.page_up());
        assert_eq!(viewer.selected_index, 14);

        viewer.selected_index = 5;
        assert!(viewer.page_up());
        assert_eq!(viewer.selected_index, 0);
    }

    #[test]
    fn test_half_page_navigation() {
        let mut viewer = ListViewer::<MockListItem>::new("Test".to_string(), "Empty".to_string());
        let items = create_mock_items(30);
        viewer.set_items(items);

        // Set a viewport height for testing
        viewer.set_last_viewport_height(10);

        // Test half_page_down
        assert_eq!(viewer.selected_index, 0);
        assert!(viewer.half_page_down());
        assert_eq!(viewer.selected_index, 5); // Half of 10

        assert!(viewer.half_page_down());
        assert_eq!(viewer.selected_index, 10);

        // Navigate to near the end
        viewer.selected_index = 25;
        assert!(viewer.half_page_down());
        assert_eq!(viewer.selected_index, 29); // Can't go past last item

        // Test that we can't scroll past the end
        assert!(!viewer.half_page_down());
        assert_eq!(viewer.selected_index, 29);

        // Test half_page_up
        assert!(viewer.half_page_up());
        assert_eq!(viewer.selected_index, 24); // 29 - 5

        viewer.selected_index = 3;
        assert!(viewer.half_page_up());
        assert_eq!(viewer.selected_index, 0); // Can't go below 0

        // Test that we can't scroll past the start
        assert!(!viewer.half_page_up());
        assert_eq!(viewer.selected_index, 0);
    }

    #[test]
    fn test_half_page_navigation_with_different_viewport_sizes() {
        let mut viewer = ListViewer::<MockListItem>::new("Test".to_string(), "Empty".to_string());
        let items = create_mock_items(50);
        viewer.set_items(items);

        // Test with viewport height of 20
        viewer.set_last_viewport_height(20);
        assert!(viewer.half_page_down());
        assert_eq!(viewer.selected_index, 10); // Half of 20

        // Test with viewport height of 7 (odd number)
        viewer.selected_index = 0;
        viewer.set_last_viewport_height(7);
        assert!(viewer.half_page_down());
        assert_eq!(viewer.selected_index, 3); // Floor of 7/2

        // Test with very small viewport
        viewer.selected_index = 0;
        viewer.set_last_viewport_height(2);
        assert!(viewer.half_page_down());
        assert_eq!(viewer.selected_index, 1);
    }

    #[test]
    fn test_filtered_navigation() {
        let mut viewer = ListViewer::<MockListItem>::new("Test".to_string(), "Empty".to_string());
        let items = create_mock_items(10);
        viewer.set_items(items);

        // Apply filter showing only even indices
        viewer.set_filtered_indices(vec![0, 2, 4, 6, 8]);

        assert_eq!(viewer.selected_index, 0);
        assert_eq!(viewer.selected_index(), 0); // Actual item index

        viewer.move_down();
        assert_eq!(viewer.selected_index, 1);
        assert_eq!(viewer.selected_index(), 2); // Actual item index

        // Test set_selected_index
        viewer.set_selected_index(6); // Set to actual item index 6
        assert_eq!(viewer.selected_index, 3); // Position in filtered list
    }

    #[test]
    fn test_scroll_offset_full_text_mode_edge_cases() {
        let mut viewer = ListViewer::<MockListItem>::new("Test".to_string(), "Empty".to_string());

        // Create items with varying content lengths
        let items: Vec<MockListItem> = (0..10)
            .map(|i| MockListItem {
                role: if i % 2 == 0 { "user" } else { "assistant" }.to_string(),
                timestamp: format!("2024-01-{:02}T12:00:00Z", i + 1),
                content: match i {
                    0..=2 => "Short message".to_string(),
                    3..=5 => "This is a medium length message that might wrap to two lines depending on the width".to_string(),
                    _ => "This is a very long message that will definitely wrap to multiple lines when displayed. It contains a lot of text to ensure that it takes up significant vertical space in the viewer.".to_string(),
                },
            })
            .collect();
        viewer.set_items(items);
        viewer.truncation_enabled = false;

        // Test scrolling to the end
        viewer.selected_index = 9;
        viewer.adjust_scroll_offset(10, 80);

        // Scroll offset should be adjusted so that item 9 is visible
        assert!(
            viewer.scroll_offset > 0,
            "Scroll offset should be adjusted for last item"
        );

        // Test scrolling back to start
        viewer.selected_index = 0;
        viewer.adjust_scroll_offset(10, 80);
        assert_eq!(
            viewer.scroll_offset, 0,
            "Scroll offset should be 0 for first item"
        );
    }

    #[test]
    fn test_scroll_offset_with_filtering_and_movement() {
        let mut viewer = ListViewer::<MockListItem>::new("Test".to_string(), "Empty".to_string());

        // Create many items to ensure scrolling is needed
        let items: Vec<MockListItem> = (0..50)
            .map(|i| MockListItem {
                role: if i % 2 == 0 { "user" } else { "assistant" }.to_string(),
                timestamp: format!("2024-01-{:02}T12:00:00Z", (i % 30) + 1),
                content: format!("Message #{i} with some content"),
            })
            .collect();
        viewer.set_items(items);

        // Apply filter that shows every 5th item
        let filtered: Vec<usize> = (0..50).step_by(5).collect();
        viewer.set_filtered_indices(filtered);

        // Test scrolling in truncated mode
        viewer.truncation_enabled = true;
        let available_height = 5;

        // Move to middle
        viewer.selected_index = 5; // Item 25 in original list
        viewer.adjust_scroll_offset(available_height, 80);

        // Should scroll to show item around the middle
        assert!(viewer.scroll_offset > 0, "Should scroll for middle item");
        assert!(
            viewer.scroll_offset <= viewer.selected_index,
            "Scroll offset should not exceed selected index"
        );

        // Move to end
        viewer.move_to_end();
        viewer.adjust_scroll_offset(available_height, 80);

        // Should scroll to show last items
        let expected_offset = viewer
            .filtered_indices
            .len()
            .saturating_sub(available_height as usize);
        assert_eq!(
            viewer.scroll_offset, expected_offset,
            "Should scroll to show last items"
        );
    }

    #[test]
    fn test_scroll_behavior_consistency() {
        let mut viewer = ListViewer::<MockListItem>::new("Test".to_string(), "Empty".to_string());
        let items = create_mock_items(20);
        viewer.set_items(items);

        // Test that scroll offset is consistent between modes
        viewer.selected_index = 15;

        // Test in truncated mode
        viewer.truncation_enabled = true;
        viewer.adjust_scroll_offset(10, 80);
        let truncated_offset = viewer.scroll_offset;

        // Test in full text mode
        viewer.truncation_enabled = false;
        viewer.scroll_offset = 0; // Reset
        viewer.adjust_scroll_offset(10, 80);

        // Both modes should have scrolled to make item 15 visible
        assert!(truncated_offset > 0, "Truncated mode should have scrolled");
        assert!(
            viewer.scroll_offset > 0,
            "Full text mode should have scrolled"
        );
    }
}
