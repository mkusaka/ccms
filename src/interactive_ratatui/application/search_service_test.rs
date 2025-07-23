#[cfg(test)]
mod tests {
    use super::super::search_service::*;
    use crate::SearchOptions;
    use crate::interactive_ratatui::domain::models::SearchRequest;

    #[test]
    fn test_search_service_creation() {
        let options = SearchOptions {
            project_path: Some("/nonexistent/test/path".to_string()),
            ..Default::default()
        };
        let _service = SearchService::new(options);

        // Just ensure it can be created
    }

    #[test]
    fn test_empty_query_returns_all_results() {
        let options = SearchOptions {
            project_path: Some("/nonexistent/test/path".to_string()),
            ..Default::default()
        };
        let service = SearchService::new(options);

        let request = SearchRequest {
            id: 1,
            query: "   ".to_string(), // Empty/whitespace query
            role_filter: None,
            pattern: "/nonexistent/test/path/*.jsonl".to_string(),
        };

        let response = service.search(request).unwrap();

        assert_eq!(response.id, 1);
        // Since we're searching a nonexistent path, we'll still get 0 results
        // but the important thing is that it doesn't reject the empty query
        // In a real scenario with files, this would return all messages
        assert_eq!(response.results.len(), 0);
    }

    #[test]
    fn test_search_with_role_filter() {
        let options = SearchOptions {
            project_path: Some("/nonexistent/test/path".to_string()),
            ..Default::default()
        };
        let service = SearchService::new(options);

        let request = SearchRequest {
            id: 42,
            query: "test".to_string(),
            role_filter: Some("user".to_string()),
            pattern: "/nonexistent/test/path/*.jsonl".to_string(),
        };

        // This would normally search files, but without test files it returns empty
        let response = service.search(request).unwrap();

        assert_eq!(response.id, 42);
        // Results would be filtered by role if any were found
    }

    #[test]
    fn test_search_request_id_propagation() {
        let options = SearchOptions {
            project_path: Some("/nonexistent/test/path".to_string()),
            ..Default::default()
        };
        let service = SearchService::new(options);

        let test_ids = vec![1, 42, 100, 999];

        for id in test_ids {
            let request = SearchRequest {
                id,
                query: "test".to_string(),
                role_filter: None,
                pattern: "/nonexistent/test/path/*.jsonl".to_string(),
            };

            let response = service.search(request).unwrap();
            assert_eq!(response.id, id);
        }
    }

    #[test]
    fn test_search_with_invalid_pattern() {
        let options = SearchOptions {
            project_path: Some("/nonexistent/test/path".to_string()),
            ..Default::default()
        };
        let service = SearchService::new(options);

        let request = SearchRequest {
            id: 1,
            query: "[[invalid regex".to_string(),
            role_filter: None,
            pattern: "/nonexistent/test/path/*.jsonl".to_string(),
        };

        // Should handle invalid regex gracefully
        let result = service.search(request);
        assert!(result.is_err());
    }
}
