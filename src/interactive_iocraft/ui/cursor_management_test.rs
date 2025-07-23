#[cfg(test)]
mod tests {
    use crate::interactive_iocraft::ui::SearchState;

    #[test]
    fn test_initial_cursor_position() {
        let search_state = SearchState::default();
        // Cursor is initially at the end of the string
        assert_eq!(search_state.cursor_position, 0);
    }

    #[test]
    fn test_cursor_position_after_text_input() {
        let mut search_state = SearchState::default();
        // Input characters one by one
        search_state.insert_char_at_cursor('H');
        search_state.insert_char_at_cursor('e');
        search_state.insert_char_at_cursor('l');
        search_state.insert_char_at_cursor('l');
        search_state.insert_char_at_cursor('o');
        // After text input, cursor moves to the end of string
        assert_eq!(search_state.query, "Hello");
        assert_eq!(search_state.cursor_position, 5);
    }

    #[test]
    fn test_cursor_move_left() {
        let mut search_state = SearchState {
            query: "Hello".to_string(),
            cursor_position: 5,
            ..Default::default()
        };

        // Move cursor left
        search_state.move_cursor_left();
        assert_eq!(search_state.cursor_position, 4);

        // Move further left
        search_state.move_cursor_left();
        assert_eq!(search_state.cursor_position, 3);

        // Move to the beginning
        for _ in 0..3 {
            search_state.move_cursor_left();
        }
        assert_eq!(search_state.cursor_position, 0);

        // Cannot go left of 0
        search_state.move_cursor_left();
        assert_eq!(search_state.cursor_position, 0);
    }

    #[test]
    fn test_cursor_move_right() {
        let mut search_state = SearchState {
            query: "Hello".to_string(),
            cursor_position: 0,
            ..Default::default()
        };

        // Move cursor right
        search_state.move_cursor_right();
        assert_eq!(search_state.cursor_position, 1);

        // Move to the end
        for _ in 0..4 {
            search_state.move_cursor_right();
        }
        assert_eq!(search_state.cursor_position, 5);

        // Cannot go right beyond string length
        search_state.move_cursor_right();
        assert_eq!(search_state.cursor_position, 5);
    }

    #[test]
    fn test_cursor_move_with_multibyte_chars() {
        let mut search_state = SearchState {
            query: "こんにちは".to_string(),
            cursor_position: 5, // Managed by character count
            ..Default::default()
        };

        // Move by character even with multibyte chars
        search_state.move_cursor_left();
        assert_eq!(search_state.cursor_position, 4);

        search_state.move_cursor_left();
        assert_eq!(search_state.cursor_position, 3);
    }

    #[test]
    fn test_insert_char_at_cursor() {
        let mut search_state = SearchState {
            query: "Hello".to_string(),
            cursor_position: 2, // "He|llo"
            ..Default::default()
        };

        // Insert character at cursor position
        search_state.insert_char_at_cursor('X');
        assert_eq!(search_state.query, "HeXllo");
        assert_eq!(search_state.cursor_position, 3); // Cursor moves after insertion
    }

    #[test]
    fn test_insert_multibyte_char_at_cursor() {
        let mut search_state = SearchState {
            query: "Hello".to_string(),
            cursor_position: 2, // "He|llo"
            ..Default::default()
        };

        // Insert multibyte character
        search_state.insert_char_at_cursor('世');
        assert_eq!(search_state.query, "He世llo");
        assert_eq!(search_state.cursor_position, 3);
    }

    #[test]
    fn test_delete_char_at_cursor() {
        let mut search_state = SearchState {
            query: "Hello".to_string(),
            cursor_position: 2, // "He|llo"
            ..Default::default()
        };

        // Delete character before cursor (backspace)
        search_state.delete_char_before_cursor();
        assert_eq!(search_state.query, "Hllo");
        assert_eq!(search_state.cursor_position, 1);

        // Nothing is deleted at the beginning
        search_state.cursor_position = 0;
        search_state.delete_char_before_cursor();
        assert_eq!(search_state.query, "Hllo");
        assert_eq!(search_state.cursor_position, 0);
    }

    #[test]
    fn test_delete_char_after_cursor() {
        let mut search_state = SearchState {
            query: "Hello".to_string(),
            cursor_position: 2, // "He|llo"
            ..Default::default()
        };

        // Delete character at cursor (Delete key)
        search_state.delete_char_at_cursor();
        assert_eq!(search_state.query, "Helo");
        assert_eq!(search_state.cursor_position, 2); // Cursor position doesn't change

        // Nothing is deleted at the end
        search_state.cursor_position = 4;
        search_state.delete_char_at_cursor();
        assert_eq!(search_state.query, "Helo");
        assert_eq!(search_state.cursor_position, 4);
    }

    #[test]
    fn test_cursor_home_end() {
        let mut search_state = SearchState {
            query: "Hello World".to_string(),
            cursor_position: 5,
            ..Default::default()
        };

        // Home key
        search_state.move_cursor_to_start();
        assert_eq!(search_state.cursor_position, 0);

        // End key
        search_state.move_cursor_to_end();
        assert_eq!(search_state.cursor_position, 11);
    }

    #[test]
    fn test_cursor_word_movement() {
        let mut search_state = SearchState {
            query: "Hello World Test".to_string(),
            cursor_position: 7, // "Hello W|orld Test"
            ..Default::default()
        };

        // Ctrl+Left - Move to beginning of previous word
        search_state.move_cursor_word_left();
        assert_eq!(search_state.cursor_position, 6); // "Hello |World Test"

        search_state.move_cursor_word_left();
        assert_eq!(search_state.cursor_position, 0); // "|Hello World Test"

        // Ctrl+Right - Move to beginning of next word
        search_state.move_cursor_word_right();
        assert_eq!(search_state.cursor_position, 6); // "Hello |World Test"

        search_state.move_cursor_word_right();
        assert_eq!(search_state.cursor_position, 12); // "Hello World |Test"
    }
}
