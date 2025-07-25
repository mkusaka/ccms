use crate::interactive_iocraft::domain::filter::SearchFilter;
use crate::interactive_iocraft::domain::models::{SearchRequest, SearchResponse};
use crate::interactive_iocraft::{QueryCondition, SearchResult, SearchEngine, SearchOptions, parse_query};
use anyhow::Result;
use std::sync::Arc;

pub struct SearchService {
    engine: Arc<SearchEngine>,
    file_patterns: Vec<String>,
    #[allow(dead_code)]
    base_options: SearchOptions,
}

impl SearchService {
    pub fn new(file_patterns: Vec<String>, verbose: bool) -> Result<Self> {
        let options = SearchOptions {
            verbose,
            ..Default::default()
        };
        let engine = Arc::new(SearchEngine::new(options.clone()));
        Ok(Self {
            engine,
            file_patterns,
            base_options: options,
        })
    }

    pub fn search(&self, request: SearchRequest) -> Result<SearchResponse> {
        let mut results = self.execute_search(&request.query, &request.pattern)?;

        // Apply filters
        let filter = SearchFilter::new(request.role_filter);
        filter.apply(&mut results)?;

        Ok(SearchResponse {
            id: request.id,
            results,
        })
    }

    fn execute_search(&self, query: &str, pattern: &str) -> Result<Vec<SearchResult>> {
        let query_condition = if query.trim().is_empty() {
            // Empty query means "match all" - use empty AND condition
            QueryCondition::And { conditions: vec![] }
        } else {
            parse_query(query)?
        };

        // Use the provided pattern, or fall back to stored patterns or default
        let search_pattern = if !pattern.is_empty() {
            pattern.to_string()
        } else if !self.file_patterns.is_empty() {
            self.file_patterns.join(",")
        } else {
            crate::interactive_iocraft::default_claude_pattern()
        };

        let (mut results, _, _) = self.engine.search(&search_pattern, query_condition)?;

        // Sort by timestamp descending
        results.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interactive_iocraft::domain::models::SearchRequest;
    use crate::interactive_iocraft::{default_claude_pattern, SearchResult};
    use std::fs;
    use std::io::Write;
    use tempfile::NamedTempFile;
    
    fn create_test_jsonl() -> (NamedTempFile, String) {
        let mut file = NamedTempFile::new().unwrap();
        let content = r#"{"uuid":"123","timestamp":"1700000000","sessionId":"abc","role":"user","text":"Hello world","projectPath":"/test"}
{"uuid":"124","timestamp":"1700000001","sessionId":"abc","role":"assistant","text":"Hi there","projectPath":"/test"}
{"uuid":"125","timestamp":"1700000002","sessionId":"abc","role":"system","text":"System message","projectPath":"/test"}"#;
        file.write_all(content.as_bytes()).unwrap();
        file.flush().unwrap();
        (file, content.to_string())
    }
    
    #[test]
    fn test_search_service_new() {
        let service = SearchService::new(vec![], false);
        assert!(service.is_ok());
        
        let service_with_patterns = SearchService::new(vec!["*.jsonl".to_string()], true);
        assert!(service_with_patterns.is_ok());
    }
    
    #[test]
    fn test_search_with_empty_query() {
        let service = SearchService::new(vec![], false).unwrap();
        let request = SearchRequest {
            id: 1,
            query: "".to_string(),
            pattern: default_claude_pattern(),
            role_filter: None,
        };
        
        let result = service.search(request);
        assert!(result.is_ok());
    }
    
    #[test]
    fn test_search_with_simple_query() {
        let service = SearchService::new(vec![], false).unwrap();
        let request = SearchRequest {
            id: 1,
            query: "test".to_string(),
            pattern: "/tmp/nonexistent.jsonl".to_string(),
            role_filter: None,
        };
        
        let result = service.search(request);
        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.results.len(), 0); // No files should match
    }
    
    #[test]
    fn test_search_with_role_filter() {
        let (file, _) = create_test_jsonl();
        let service = SearchService::new(vec![], false).unwrap();
        
        // Search with user role filter
        let request = SearchRequest {
            id: 1,
            query: "".to_string(),
            pattern: file.path().to_string_lossy().to_string(),
            role_filter: Some("user".to_string()),
        };
        
        let result = service.search(request);
        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.results.len(), 1);
        assert_eq!(response.results[0].role, "user");
    }
    
    #[test]
    fn test_search_with_complex_query() {
        let (file, _) = create_test_jsonl();
        let service = SearchService::new(vec![], false).unwrap();
        
        // Test AND query
        let request = SearchRequest {
            id: 1,
            query: "Hello AND world".to_string(),
            pattern: file.path().to_string_lossy().to_string(),
            role_filter: None,
        };
        
        let result = service.search(request);
        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.results.len(), 1);
        assert!(response.results[0].text.contains("Hello world"));
        
        // Test OR query
        let request = SearchRequest {
            id: 2,
            query: "Hello OR Hi".to_string(),
            pattern: file.path().to_string_lossy().to_string(),
            role_filter: None,
        };
        
        let result = service.search(request);
        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.results.len(), 2);
        
        // Test NOT query
        let request = SearchRequest {
            id: 3,
            query: "NOT System".to_string(),
            pattern: file.path().to_string_lossy().to_string(),
            role_filter: None,
        };
        
        let result = service.search(request);
        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.results.len(), 2);
        assert!(response.results.iter().all(|r| !r.text.contains("System")));
    }
    
    #[test]
    fn test_search_pattern_fallback() {
        let service = SearchService::new(vec!["*.jsonl".to_string()], false).unwrap();
        
        // Test with empty pattern - should use stored patterns
        let request = SearchRequest {
            id: 1,
            query: "test".to_string(),
            pattern: "".to_string(),
            role_filter: None,
        };
        
        let result = service.search(request);
        assert!(result.is_ok());
        
        // Test with provided pattern - should override stored patterns
        let request = SearchRequest {
            id: 2,
            query: "test".to_string(),
            pattern: "/specific/path/*.jsonl".to_string(),
            role_filter: None,
        };
        
        let result = service.search(request);
        assert!(result.is_ok());
    }
    
    #[test]
    fn test_search_results_sorting() {
        let (file, _) = create_test_jsonl();
        let service = SearchService::new(vec![], false).unwrap();
        
        let request = SearchRequest {
            id: 1,
            query: "".to_string(),
            pattern: file.path().to_string_lossy().to_string(),
            role_filter: None,
        };
        
        let result = service.search(request);
        assert!(result.is_ok());
        let response = result.unwrap();
        
        // Results should be sorted by timestamp descending
        assert_eq!(response.results.len(), 3);
        assert_eq!(response.results[0].timestamp, "1700000002");
        assert_eq!(response.results[1].timestamp, "1700000001");
        assert_eq!(response.results[2].timestamp, "1700000000");
    }
    
    #[test]
    fn test_search_with_invalid_query() {
        let service = SearchService::new(vec![], false).unwrap();
        
        let request = SearchRequest {
            id: 1,
            query: "AND AND".to_string(), // Invalid query syntax
            pattern: "/tmp/test.jsonl".to_string(),
            role_filter: None,
        };
        
        let result = service.search(request);
        assert!(result.is_err());
    }
    
    #[test]
    fn test_search_response_id_preservation() {
        let service = SearchService::new(vec![], false).unwrap();
        
        let request = SearchRequest {
            id: 42,
            query: "test".to_string(),
            pattern: "/tmp/nonexistent.jsonl".to_string(),
            role_filter: None,
        };
        
        let result = service.search(request);
        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.id, 42);
    }
    
    #[test]
    fn test_multiple_role_filters() {
        let (file, _) = create_test_jsonl();
        let service = SearchService::new(vec![], false).unwrap();
        
        // Test each role filter
        for (role, expected_count) in &[("user", 1), ("assistant", 1), ("system", 1)] {
            let request = SearchRequest {
                id: 1,
                query: "".to_string(),
                pattern: file.path().to_string_lossy().to_string(),
                role_filter: Some(role.to_string()),
            };
            
            let result = service.search(request);
            assert!(result.is_ok());
            let response = result.unwrap();
            assert_eq!(response.results.len(), *expected_count);
            assert!(response.results.iter().all(|r| r.role == *role));
        }
    }
}
