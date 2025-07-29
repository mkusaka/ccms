#[cfg(test)]
mod tests {
    use super::super::models::*;

    #[test]
    fn test_mode_equality() {
        assert_eq!(Mode::Search, Mode::Search);
        assert_ne!(Mode::Search, Mode::Help);
        assert_ne!(Mode::MessageDetail, Mode::SessionViewer);
    }

    #[test]
    fn test_session_order_variants() {
        // Ensure all variants are constructible
        let orders = vec![SessionOrder::Ascending, SessionOrder::Descending];

        for order in orders {
            match order {
                SessionOrder::Ascending => {}
                SessionOrder::Descending => {}
            }
        }
    }

    #[test]
    fn test_cached_file_creation() {
        use std::time::SystemTime;

        let cached_file = CachedFile {
            raw_lines: vec!["line1".to_string(), "line2".to_string()],
            messages: vec![],
            last_modified: SystemTime::now(),
        };

        assert_eq!(cached_file.raw_lines.len(), 2);
        assert_eq!(cached_file.messages.len(), 0);
    }

    #[test]
    fn test_search_request_creation() {
        let request = SearchRequest {
            id: 42,
            query: "test query".to_string(),
            role_filter: Some("user".to_string()),
            pattern: "*.jsonl".to_string(),
            order: SearchOrder::Descending,
        };

        assert_eq!(request.id, 42);
        assert_eq!(request.query, "test query");
        assert_eq!(request.role_filter, Some("user".to_string()));
        assert_eq!(request.pattern, "*.jsonl");
    }

    #[test]
    fn test_search_response_creation() {
        use crate::query::condition::{QueryCondition, SearchResult};

        let results = vec![SearchResult {
            file: "test.jsonl".to_string(),
            uuid: "uuid1".to_string(),
            timestamp: "2024-01-01T00:00:00Z".to_string(),
            session_id: "session1".to_string(),
            role: "user".to_string(),
            text: "Hello".to_string(),
            has_tools: false,
            has_thinking: false,
            message_type: "user".to_string(),
            query: QueryCondition::Literal {
                pattern: "test".to_string(),
                case_sensitive: false,
            },
            project_path: "/test".to_string(),
            raw_json: None,
        }];

        let response = SearchResponse {
            id: 42,
            results: results.clone(),
        };

        assert_eq!(response.id, 42);
        assert_eq!(response.results.len(), 1);
        assert_eq!(response.results[0].text, "Hello");
    }

    #[test]
    fn test_mode_debug_display() {
        // Ensure Mode can be printed for debugging
        let mode = Mode::Search;
        let debug_str = format!("{mode:?}");
        assert!(debug_str.contains("Search"));

        let mode = Mode::Help;
        let debug_str = format!("{mode:?}");
        assert!(debug_str.contains("Help"));
    }

    #[test]
    fn test_search_request_clone() {
        let original = SearchRequest {
            id: 1,
            query: "test".to_string(),
            role_filter: None,
            pattern: "*.jsonl".to_string(),
            order: SearchOrder::Ascending,
        };

        let cloned = original.clone();
        assert_eq!(cloned.id, original.id);
        assert_eq!(cloned.query, original.query);
        assert_eq!(cloned.role_filter, original.role_filter);
        assert_eq!(cloned.pattern, original.pattern);
        assert_eq!(cloned.order, original.order);
    }
}
