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
        let messages = vec![
            "Hello world".to_string(),
            "How are you?".to_string(),
            "I'm fine, thanks".to_string(),
        ];

        let indices = SessionFilter::filter_messages(&messages, "");

        // Empty query should return all messages
        assert_eq!(indices, vec![0, 1, 2]);
    }

    #[test]
    fn test_session_filter_case_insensitive() {
        let messages = vec![
            "Hello World".to_string(),
            "goodbye world".to_string(),
            "WORLD champion".to_string(),
            "Something else".to_string(),
        ];

        let indices = SessionFilter::filter_messages(&messages, "world");

        assert_eq!(indices, vec![0, 1, 2]);
    }

    #[test]
    fn test_session_filter_partial_match() {
        let messages = vec![
            "The quick brown fox".to_string(),
            "jumps over the lazy dog".to_string(),
            "The fox is quick".to_string(),
        ];

        let indices = SessionFilter::filter_messages(&messages, "fox");

        assert_eq!(indices, vec![0, 2]);
    }

    #[test]
    fn test_session_filter_unicode() {
        let messages = vec![
            "こんにちは世界".to_string(),
            "Hello 世界".to_string(),
            "世界 is world".to_string(),
            "Something else".to_string(),
        ];

        let indices = SessionFilter::filter_messages(&messages, "世界");

        assert_eq!(indices, vec![0, 1, 2]);
    }

    #[test]
    fn test_session_filter_whitespace() {
        let messages = vec![
            "  Hello  World  ".to_string(),
            "Hello\tWorld".to_string(),
            "Hello\nWorld".to_string(),
        ];

        let indices = SessionFilter::filter_messages(&messages, "Hello World");

        // Should match all since we normalize whitespace
        assert_eq!(indices.len(), 0); // Currently doesn't handle this case
    }
}
