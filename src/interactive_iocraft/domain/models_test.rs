#[cfg(test)]
mod tests {
    use super::super::*;

    #[test]
    fn test_mode_default() {
        let mode = Mode::default();
        assert!(matches!(mode, Mode::Search));
    }

    #[test]
    fn test_search_request_creation() {
        let request = SearchRequest {
            id: 1,
            query: "test query".to_string(),
            role_filter: Some("user".to_string()),
            pattern: "/path/to/files".to_string(),
        };

        assert_eq!(request.id, 1);
        assert_eq!(request.query, "test query");
        assert_eq!(request.role_filter, Some("user".to_string()));
        assert_eq!(request.pattern, "/path/to/files");
    }

    #[test]
    fn test_search_response_creation() {
        let results = vec![];
        let response = SearchResponse { id: 1, results };

        assert_eq!(response.id, 1);
        assert!(response.results.is_empty());
    }

    #[test]
    fn test_session_order_variants() {
        // Test all SessionOrder variants exist
        let _ = SessionOrder::Ascending;
        let _ = SessionOrder::Descending;
        let _ = SessionOrder::Original;
    }

    #[test]
    fn test_mode_variants() {
        // Test all Mode variants exist
        let _ = Mode::Search;
        let _ = Mode::ResultDetail;
        let _ = Mode::SessionViewer;
        let _ = Mode::Help;
    }

    #[test]
    fn test_search_request_with_no_role_filter() {
        let request = SearchRequest {
            id: 42,
            query: "search".to_string(),
            role_filter: None,
            pattern: "*.jsonl".to_string(),
        };

        assert_eq!(request.id, 42);
        assert_eq!(request.query, "search");
        assert!(request.role_filter.is_none());
        assert_eq!(request.pattern, "*.jsonl");
    }

    #[test]
    fn test_mode_equality() {
        assert_eq!(Mode::Search, Mode::Search);
        assert_eq!(Mode::Help, Mode::Help);
        assert_ne!(Mode::Search, Mode::Help);
        assert_ne!(Mode::ResultDetail, Mode::SessionViewer);
    }

    #[test]
    fn test_session_order_equality() {
        assert_eq!(SessionOrder::Ascending, SessionOrder::Ascending);
        assert_eq!(SessionOrder::Descending, SessionOrder::Descending);
        assert_ne!(SessionOrder::Ascending, SessionOrder::Descending);
    }
}
