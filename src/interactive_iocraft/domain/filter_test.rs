#[cfg(test)]
mod tests {
    use super::super::{SearchFilter, SessionFilter};
    use crate::query::condition::SearchResult;

    #[test]
    fn test_filter_messages_empty_query() {
        let messages = vec![
            "First message".to_string(),
            "Second message".to_string(),
            "Third message".to_string(),
        ];

        let filtered = SessionFilter::filter_messages(&messages, "");
        assert_eq!(filtered, vec![0, 1, 2]);
    }

    #[test]
    fn test_filter_messages_with_query() {
        let messages = vec![
            "Hello world".to_string(),
            "Goodbye world".to_string(),
            "Hello again".to_string(),
            "Something else".to_string(),
        ];

        let filtered = SessionFilter::filter_messages(&messages, "hello");
        assert_eq!(filtered, vec![0, 2]);
    }

    #[test]
    fn test_filter_messages_case_insensitive() {
        let messages = vec![
            "HELLO world".to_string(),
            "hello WORLD".to_string(),
            "HeLLo Again".to_string(),
            "goodbye".to_string(),
        ];

        let filtered = SessionFilter::filter_messages(&messages, "HELLO");
        assert_eq!(filtered, vec![0, 1, 2]);
    }

    #[test]
    fn test_filter_messages_no_matches() {
        let messages = vec![
            "First message".to_string(),
            "Second message".to_string(),
            "Third message".to_string(),
        ];

        let filtered = SessionFilter::filter_messages(&messages, "xyz");
        assert!(filtered.is_empty());
    }

    #[test]
    fn test_filter_messages_partial_match() {
        let messages = vec![
            "The quick brown fox".to_string(),
            "jumps over the lazy dog".to_string(),
            "The fox runs fast".to_string(),
        ];

        let filtered = SessionFilter::filter_messages(&messages, "fox");
        assert_eq!(filtered, vec![0, 2]);
    }

    #[test]
    fn test_filter_messages_with_whitespace() {
        let messages = vec![
            "  Hello world  ".to_string(),
            "Hello\tworld".to_string(),
            "Hello\nworld".to_string(),
            "No hello here".to_string(),
        ];

        let filtered = SessionFilter::filter_messages(&messages, "hello world");
        // Should match the first one with spaces
        assert_eq!(filtered, vec![0]);
    }

    #[test]
    fn test_filter_messages_empty_messages() {
        let messages: Vec<String> = vec![];
        let filtered = SessionFilter::filter_messages(&messages, "query");
        assert!(filtered.is_empty());
    }

    #[test]
    fn test_filter_messages_unicode() {
        let messages = vec![
            "こんにちは世界".to_string(),
            "Hello 世界".to_string(),
            "世界 World".to_string(),
            "Bonjour le monde".to_string(),
        ];

        let filtered = SessionFilter::filter_messages(&messages, "世界");
        assert_eq!(filtered, vec![0, 1, 2]);
    }

    #[test]
    fn test_filter_messages_special_characters() {
        let messages = vec![
            "user@example.com".to_string(),
            "admin@test.org".to_string(),
            "support@example.com".to_string(),
        ];

        let filtered = SessionFilter::filter_messages(&messages, "@example.com");
        assert_eq!(filtered, vec![0, 2]);
    }

    // SearchFilter tests
    #[test]
    fn test_search_filter_no_role_filter() {
        let filter = SearchFilter::new(None);
        let mut results = vec![
            create_test_result("user"),
            create_test_result("assistant"),
            create_test_result("system"),
        ];

        let original_len = results.len();
        let result = filter.apply(&mut results);

        assert!(result.is_ok());
        assert_eq!(results.len(), original_len);
    }

    #[test]
    fn test_search_filter_with_role_filter() {
        let filter = SearchFilter::new(Some("user".to_string()));
        let mut results = vec![
            create_test_result("user"),
            create_test_result("assistant"),
            create_test_result("user"),
            create_test_result("system"),
        ];

        let result = filter.apply(&mut results);

        assert!(result.is_ok());
        assert_eq!(results.len(), 2);
        assert!(results.iter().all(|r| r.role == "user"));
    }

    #[test]
    fn test_search_filter_case_insensitive() {
        let filter = SearchFilter::new(Some("USER".to_string()));
        let mut results = vec![
            create_test_result("user"),
            create_test_result("User"),
            create_test_result("USER"),
            create_test_result("assistant"),
        ];

        let result = filter.apply(&mut results);

        assert!(result.is_ok());
        assert_eq!(results.len(), 3);
    }

    #[test]
    fn test_search_filter_no_matches() {
        let filter = SearchFilter::new(Some("admin".to_string()));
        let mut results = vec![
            create_test_result("user"),
            create_test_result("assistant"),
            create_test_result("system"),
        ];

        let result = filter.apply(&mut results);

        assert!(result.is_ok());
        assert!(results.is_empty());
    }

    #[test]
    fn test_search_filter_empty_results() {
        let filter = SearchFilter::new(Some("user".to_string()));
        let mut results: Vec<SearchResult> = vec![];

        let result = filter.apply(&mut results);

        assert!(result.is_ok());
        assert!(results.is_empty());
    }

    // Helper function to create test SearchResult
    fn create_test_result(role: &str) -> SearchResult {
        SearchResult {
            file: "test.jsonl".to_string(),
            uuid: "test-uuid".to_string(),
            timestamp: "2024-01-01T00:00:00Z".to_string(),
            session_id: "test-session".to_string(),
            role: role.to_string(),
            text: "Test message".to_string(),
            has_tools: false,
            has_thinking: false,
            message_type: "text".to_string(),
            query: crate::query::condition::QueryCondition::Literal {
                pattern: "test".to_string(),
                case_sensitive: false,
            },
            project_path: "test-project".to_string(),
            raw_json: None,
        }
    }
}
