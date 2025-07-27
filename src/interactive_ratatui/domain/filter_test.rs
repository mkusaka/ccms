#[cfg(test)]
mod tests {
    use super::super::filter::*;
    use crate::query::condition::{QueryCondition, SearchResult};

    fn create_test_result(role: &str, text: &str) -> SearchResult {
        SearchResult {
            file: "test.jsonl".to_string(),
            uuid: "test-uuid".to_string(),
            timestamp: "2024-01-01T00:00:00Z".to_string(),
            session_id: "test-session".to_string(),
            role: role.to_string(),
            text: text.to_string(),
            has_tools: false,
            has_thinking: false,
            message_type: role.to_string(),
            query: QueryCondition::Literal {
                pattern: "test".to_string(),
                case_sensitive: false,
            },
            project_path: "/test".to_string(),
            raw_json: None,
        }
    }

    #[test]
    fn test_search_filter_no_filter() {
        let mut results = vec![
            create_test_result("user", "Hello"),
            create_test_result("assistant", "Hi"),
            create_test_result("system", "Welcome"),
        ];

        let filter = SearchFilter::new(None);
        filter.apply(&mut results).unwrap();

        assert_eq!(results.len(), 3);
    }

    #[test]
    fn test_search_filter_user_role() {
        let mut results = vec![
            create_test_result("user", "Hello"),
            create_test_result("assistant", "Hi"),
            create_test_result("user", "How are you?"),
            create_test_result("system", "Welcome"),
        ];

        let filter = SearchFilter::new(Some("user".to_string()));
        filter.apply(&mut results).unwrap();

        assert_eq!(results.len(), 2);
        assert_eq!(results[0].role, "user");
        assert_eq!(results[1].role, "user");
    }

    #[test]
    fn test_search_filter_assistant_role() {
        let mut results = vec![
            create_test_result("user", "Hello"),
            create_test_result("assistant", "Hi"),
            create_test_result("assistant", "I'm doing well"),
        ];

        let filter = SearchFilter::new(Some("assistant".to_string()));
        filter.apply(&mut results).unwrap();

        assert_eq!(results.len(), 2);
        assert!(results.iter().all(|r| r.role == "assistant"));
    }

    #[test]
    fn test_search_filter_empty_results() {
        let mut results: Vec<SearchResult> = vec![];

        let filter = SearchFilter::new(Some("user".to_string()));
        filter.apply(&mut results).unwrap();

        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_session_filter_empty_query() {
        use crate::interactive_ratatui::domain::session_list_item::SessionListItem;

        // Create test JSONL data
        let jsonl_data = [
            r#"{"type":"user","message":{"role":"user","content":"Hello world"},"uuid":"1","timestamp":"2024-12-25T14:30:00Z","sessionId":"session1","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}"#,
            r#"{"type":"assistant","message":{"role":"assistant","content":"How are you?"},"uuid":"2","timestamp":"2024-12-25T14:31:00Z","sessionId":"session1","parentUuid":"1","isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}"#,
            r#"{"type":"system","content":"I'm fine, thanks","uuid":"3","timestamp":"2024-12-25T14:32:00Z","sessionId":"session1","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}"#,
        ];

        let items: Vec<SessionListItem> = jsonl_data
            .iter()
            .enumerate()
            .filter_map(|(_idx, line)| SessionListItem::from_json_line(line))
            .collect();

        let indices = SessionFilter::filter_messages(&items, "", &None);

        // Empty query should return all messages
        assert_eq!(indices, vec![0, 1, 2]);
    }

    #[test]
    fn test_session_filter_case_insensitive() {
        use crate::interactive_ratatui::domain::session_list_item::SessionListItem;

        let jsonl_data = [
            r#"{"type":"user","message":{"role":"user","content":"Hello World"},"uuid":"1","timestamp":"2024-12-25T14:30:00Z","sessionId":"session1","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}"#,
            r#"{"type":"assistant","message":{"role":"assistant","content":"goodbye world"},"uuid":"2","timestamp":"2024-12-25T14:31:00Z","sessionId":"session1","parentUuid":"1","isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}"#,
            r#"{"type":"system","content":"WORLD champion","uuid":"3","timestamp":"2024-12-25T14:32:00Z","sessionId":"session1","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}"#,
            r#"{"type":"user","message":{"role":"user","content":"Something else"},"uuid":"4","timestamp":"2024-12-25T14:33:00Z","sessionId":"session1","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}"#,
        ];

        let items: Vec<SessionListItem> = jsonl_data
            .iter()
            .enumerate()
            .filter_map(|(_idx, line)| SessionListItem::from_json_line(line))
            .collect();

        let indices = SessionFilter::filter_messages(&items, "world", &None);

        assert_eq!(indices, vec![0, 1, 2]);
    }

    #[test]
    fn test_session_filter_partial_match() {
        use crate::interactive_ratatui::domain::session_list_item::SessionListItem;

        let jsonl_data = [
            r#"{"type":"user","message":{"role":"user","content":"The quick brown fox"},"uuid":"1","timestamp":"2024-12-25T14:30:00Z","sessionId":"session1","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}"#,
            r#"{"type":"assistant","message":{"role":"assistant","content":"jumps over the lazy dog"},"uuid":"2","timestamp":"2024-12-25T14:31:00Z","sessionId":"session1","parentUuid":"1","isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}"#,
            r#"{"type":"system","content":"The fox is quick","uuid":"3","timestamp":"2024-12-25T14:32:00Z","sessionId":"session1","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}"#,
        ];

        let items: Vec<SessionListItem> = jsonl_data
            .iter()
            .enumerate()
            .filter_map(|(_idx, line)| SessionListItem::from_json_line(line))
            .collect();

        let indices = SessionFilter::filter_messages(&items, "fox", &None);

        assert_eq!(indices, vec![0, 2]);
    }

    #[test]
    fn test_session_filter_unicode() {
        use crate::interactive_ratatui::domain::session_list_item::SessionListItem;

        let jsonl_data = [
            r#"{"type":"user","message":{"role":"user","content":"こんにちは世界"},"uuid":"1","timestamp":"2024-12-25T14:30:00Z","sessionId":"session1","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}"#,
            r#"{"type":"assistant","message":{"role":"assistant","content":"Hello 世界"},"uuid":"2","timestamp":"2024-12-25T14:31:00Z","sessionId":"session1","parentUuid":"1","isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}"#,
            r#"{"type":"system","content":"世界 is world","uuid":"3","timestamp":"2024-12-25T14:32:00Z","sessionId":"session1","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}"#,
            r#"{"type":"user","message":{"role":"user","content":"Something else"},"uuid":"4","timestamp":"2024-12-25T14:33:00Z","sessionId":"session1","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}"#,
        ];

        let items: Vec<SessionListItem> = jsonl_data
            .iter()
            .enumerate()
            .filter_map(|(_idx, line)| SessionListItem::from_json_line(line))
            .collect();

        let indices = SessionFilter::filter_messages(&items, "世界", &None);

        assert_eq!(indices, vec![0, 1, 2]);
    }

    #[test]
    fn test_session_filter_whitespace() {
        use crate::interactive_ratatui::domain::session_list_item::SessionListItem;

        let jsonl_data = [
            r#"{"type":"user","message":{"role":"user","content":"  Hello  World  "},"uuid":"1","timestamp":"2024-12-25T14:30:00Z","sessionId":"session1","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}"#,
            r#"{"type":"assistant","message":{"role":"assistant","content":"Hello\tWorld"},"uuid":"2","timestamp":"2024-12-25T14:31:00Z","sessionId":"session1","parentUuid":"1","isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}"#,
            r#"{"type":"system","content":"Hello\nWorld","uuid":"3","timestamp":"2024-12-25T14:32:00Z","sessionId":"session1","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}"#,
        ];

        let items: Vec<SessionListItem> = jsonl_data
            .iter()
            .enumerate()
            .filter_map(|(_idx, line)| SessionListItem::from_json_line(line))
            .collect();

        // Test various whitespace scenarios

        // Search for "Hello" should find all items
        let hello_indices = SessionFilter::filter_messages(&items, "Hello", &None);
        assert_eq!(hello_indices, vec![0, 1, 2]);

        // Search for "World" should find all items (all contain "World")
        let world_indices = SessionFilter::filter_messages(&items, "World", &None);
        assert_eq!(world_indices, vec![0, 1, 2]);

        // Search for exact "Hello World" (single space) won't match the multi-space version
        let exact_indices = SessionFilter::filter_messages(&items, "Hello World", &None);
        assert_eq!(exact_indices, Vec::<usize>::new());

        // Search for "Hello\t" should find the tab version
        let tab_indices = SessionFilter::filter_messages(&items, "Hello\t", &None);
        assert_eq!(tab_indices, vec![1]);

        // Search for "Hello\n" should find the newline version
        let newline_indices = SessionFilter::filter_messages(&items, "Hello\n", &None);
        assert_eq!(newline_indices, vec![2]);
    }

    #[test]
    fn test_session_filter_with_real_jsonl_data() {
        use crate::interactive_ratatui::domain::session_list_item::SessionListItem;

        // This test validates that search now works on display text, not raw JSON
        let jsonl_data = [
            r#"{"type":"user","message":{"role":"user","content":"Hello, how can I help you?"},"uuid":"1","timestamp":"2024-12-25T14:30:00Z","sessionId":"session1","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}"#,
            r#"{"type":"assistant","message":{"role":"assistant","content":"I'm here to assist you!"},"uuid":"2","timestamp":"2024-12-25T14:31:00Z","sessionId":"session1","parentUuid":"1","isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}"#,
            r#"{"type":"system","content":"System initialized","uuid":"3","timestamp":"2024-12-25T14:32:00Z","sessionId":"session1","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}"#,
        ];

        let items: Vec<SessionListItem> = jsonl_data
            .iter()
            .enumerate()
            .filter_map(|(_idx, line)| SessionListItem::from_json_line(line))
            .collect();

        // These searches now work on display text as expected

        // Should find user messages when searching for display role
        let user_search = SessionFilter::filter_messages(&items, "user", &None);
        assert_eq!(user_search.len(), 1); // Finds the user message

        // Should find messages with "12/25" in the formatted timestamp
        let date_search = SessionFilter::filter_messages(&items, "12/25", &None);
        assert_eq!(date_search.len(), 3); // Now finds all messages because they all have the same date

        // Should work because "Hello" is in the content
        let content_search = SessionFilter::filter_messages(&items, "Hello", &None);
        assert_eq!(content_search.len(), 1); // Finds the message with "Hello"

        // Should find assistant messages when searching for display role
        let assistant_search = SessionFilter::filter_messages(&items, "assistant", &None);
        assert_eq!(assistant_search.len(), 1); // Finds the assistant message

        // Should find system messages when searching for display role
        let system_search = SessionFilter::filter_messages(&items, "system", &None);
        assert_eq!(system_search.len(), 1); // Finds the system message
    }

    #[test]
    fn test_session_filter_with_role_filter() {
        use crate::interactive_ratatui::domain::session_list_item::SessionListItem;

        let jsonl_data = [
            r#"{"type":"user","message":{"role":"user","content":"Hello"},"uuid":"1","timestamp":"2024-12-25T14:30:00Z","sessionId":"session1","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}"#,
            r#"{"type":"assistant","message":{"role":"assistant","content":"Hi there"},"uuid":"2","timestamp":"2024-12-25T14:31:00Z","sessionId":"session1","parentUuid":"1","isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}"#,
            r#"{"type":"user","message":{"role":"user","content":"How are you?"},"uuid":"3","timestamp":"2024-12-25T14:32:00Z","sessionId":"session1","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}"#,
            r#"{"type":"system","content":"System message","uuid":"4","timestamp":"2024-12-25T14:33:00Z","sessionId":"session1","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}"#,
        ];

        let items: Vec<SessionListItem> = jsonl_data
            .iter()
            .enumerate()
            .filter_map(|(_idx, line)| SessionListItem::from_json_line(line))
            .collect();

        // Test with no role filter - should return all messages
        let indices = SessionFilter::filter_messages(&items, "", &None);
        assert_eq!(indices, vec![0, 1, 2, 3]);

        // Test with user role filter - should return only user messages
        let user_filter = Some("user".to_string());
        let indices = SessionFilter::filter_messages(&items, "", &user_filter);
        assert_eq!(indices, vec![0, 2]);

        // Test with assistant role filter
        let assistant_filter = Some("assistant".to_string());
        let indices = SessionFilter::filter_messages(&items, "", &assistant_filter);
        assert_eq!(indices, vec![1]);

        // Test with system role filter
        let system_filter = Some("system".to_string());
        let indices = SessionFilter::filter_messages(&items, "", &system_filter);
        assert_eq!(indices, vec![3]);
    }

    #[test]
    fn test_session_filter_with_role_and_text_filter() {
        use crate::interactive_ratatui::domain::session_list_item::SessionListItem;

        let jsonl_data = [
            r#"{"type":"user","message":{"role":"user","content":"Hello world"},"uuid":"1","timestamp":"2024-12-25T14:30:00Z","sessionId":"session1","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}"#,
            r#"{"type":"assistant","message":{"role":"assistant","content":"Hello there"},"uuid":"2","timestamp":"2024-12-25T14:31:00Z","sessionId":"session1","parentUuid":"1","isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}"#,
            r#"{"type":"user","message":{"role":"user","content":"Goodbye"},"uuid":"3","timestamp":"2024-12-25T14:32:00Z","sessionId":"session1","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}"#,
            r#"{"type":"system","content":"Hello system","uuid":"4","timestamp":"2024-12-25T14:33:00Z","sessionId":"session1","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}"#,
        ];

        let items: Vec<SessionListItem> = jsonl_data
            .iter()
            .enumerate()
            .filter_map(|(_idx, line)| SessionListItem::from_json_line(line))
            .collect();

        // Test with text filter "Hello" and user role filter
        let user_filter = Some("user".to_string());
        let indices = SessionFilter::filter_messages(&items, "Hello", &user_filter);
        assert_eq!(indices, vec![0]); // Only the first user message contains "Hello"

        // Test with text filter "Hello" and assistant role filter
        let assistant_filter = Some("assistant".to_string());
        let indices = SessionFilter::filter_messages(&items, "Hello", &assistant_filter);
        assert_eq!(indices, vec![1]); // Only the assistant message contains "Hello"

        // Test with text filter "Hello" and no role filter
        let indices = SessionFilter::filter_messages(&items, "Hello", &None);
        assert_eq!(indices, vec![0, 1, 3]); // All messages containing "Hello"
    }

    #[test]
    fn test_session_filter_case_insensitive_role() {
        use crate::interactive_ratatui::domain::session_list_item::SessionListItem;

        let jsonl_data = [
            r#"{"type":"USER","message":{"role":"USER","content":"Hello"},"uuid":"1","timestamp":"2024-12-25T14:30:00Z","sessionId":"session1","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}"#,
            r#"{"type":"User","message":{"role":"User","content":"Hi"},"uuid":"2","timestamp":"2024-12-25T14:31:00Z","sessionId":"session1","parentUuid":"1","isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}"#,
            r#"{"type":"user","message":{"role":"user","content":"Hey"},"uuid":"3","timestamp":"2024-12-25T14:32:00Z","sessionId":"session1","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}"#,
        ];

        let items: Vec<SessionListItem> = jsonl_data
            .iter()
            .enumerate()
            .filter_map(|(_idx, line)| SessionListItem::from_json_line(line))
            .collect();

        // Test with lowercase role filter - should match all case variations
        let user_filter = Some("user".to_string());
        let indices = SessionFilter::filter_messages(&items, "", &user_filter);
        assert_eq!(indices, vec![0, 1, 2]);
    }
}
