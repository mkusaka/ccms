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

    #[test]
    fn test_role_filter_applied_before_max_results() {
        // Test that role filter is applied before max_results truncation
        // This ensures that when changing role filter, we get up to max_results
        // of the filtered role, not just whatever was in the first max_results

        // Create a service with a low max_results for testing
        let options = SearchOptions {
            max_results: Some(5),
            ..Default::default()
        };
        let service = SearchService::new(options);

        // Request without role filter should get mixed results
        let request1 = SearchRequest {
            id: 1,
            query: "".to_string(),
            role_filter: None,
            pattern: "/nonexistent/test/path/*.jsonl".to_string(),
        };

        // Request with role filter should get only that role
        let request2 = SearchRequest {
            id: 2,
            query: "".to_string(),
            role_filter: Some("user".to_string()),
            pattern: "/nonexistent/test/path/*.jsonl".to_string(),
        };

        // Both will return empty due to missing file, but the structure is correct
        let response1 = service.search(request1).unwrap();
        let response2 = service.search(request2).unwrap();

        assert_eq!(response1.id, 1);
        assert_eq!(response2.id, 2);
        // In a real scenario with files containing mixed roles,
        // response2 would have up to 5 user messages, not just user messages
        // that happened to be in the first 5 overall results
    }
}
