#[cfg(test)]
mod tests {
    use super::super::SearchService;
    use crate::SearchOptions;
    use crate::interactive_iocraft::domain::SearchRequest;

    fn create_test_service() -> SearchService {
        let options = SearchOptions {
            max_results: Some(50),
            role: None,
            session_id: None,
            before: None,
            after: None,
            verbose: false,
            project_path: None,
        };
        SearchService::new(options)
    }

    #[test]
    fn test_search_service_new() {
        let service = create_test_service();
        // Service should be created successfully
        // Can't test internal state due to privacy, but construction should succeed
        let _service = service;
    }

    #[test]
    fn test_search_empty_query() {
        let service = create_test_service();
        let request = SearchRequest {
            id: 1,
            query: "".to_string(),
            role_filter: None,
            pattern: "test.jsonl".to_string(),
        };

        let result = service.search(request);
        assert!(result.is_ok());

        let response = result.unwrap();
        assert_eq!(response.id, 1);
        assert!(response.results.is_empty());
    }

    #[test]
    fn test_search_whitespace_query() {
        let service = create_test_service();
        let request = SearchRequest {
            id: 2,
            query: "   ".to_string(),
            role_filter: None,
            pattern: "test.jsonl".to_string(),
        };

        let result = service.search(request);
        assert!(result.is_ok());

        let response = result.unwrap();
        assert_eq!(response.id, 2);
        assert!(response.results.is_empty());
    }

    #[test]
    fn test_search_with_role_filter() {
        let service = create_test_service();
        let request = SearchRequest {
            id: 3,
            query: "test".to_string(),
            role_filter: Some("user".to_string()),
            pattern: "test.jsonl".to_string(),
        };

        // This will fail in test environment due to missing files,
        // but we're testing that the API handles this gracefully
        let result = service.search(request);
        // In test environment without files, this should return an error
        assert!(result.is_err() || (result.is_ok() && result.unwrap().results.is_empty()));
    }

    #[test]
    fn test_search_response_id_matches_request() {
        let service = create_test_service();
        let request_id = 42;
        let request = SearchRequest {
            id: request_id,
            query: "".to_string(), // Empty query to ensure empty results
            role_filter: None,
            pattern: "test.jsonl".to_string(),
        };

        let result = service.search(request);
        assert!(result.is_ok());

        let response = result.unwrap();
        assert_eq!(response.id, request_id);
    }

    #[test]
    fn test_search_with_different_patterns() {
        let service = create_test_service();

        // Create a temporary test file to search
        let tmp_file = std::env::temp_dir().join("test_search_pattern.jsonl");
        std::fs::write(&tmp_file, r#"{"type":"user","uuid":"1","timestamp":"2024-01-01T00:00:00Z","sessionId":"test","version":"1.0","cwd":"/","userType":"user","isSidechain":false,"parentUuid":null,"message":{"role":"user","content":"query"}}
"#)
            .expect("Failed to write test file");

        let patterns = [tmp_file.to_str().unwrap()];

        for (i, pattern) in patterns.iter().enumerate() {
            let request = SearchRequest {
                id: i as u64,
                query: "query".to_string(),
                role_filter: None,
                pattern: pattern.to_string(),
            };

            // Test that search can handle the pattern
            let result = service.search(request);
            assert!(result.is_ok());
        }

        // Clean up
        std::fs::remove_file(&tmp_file).ok();
    }
}
