#[cfg(test)]
mod tests {
    use crate::interactive_ratatui::ui::components::list_item::ListItem;
    use crate::query::condition::SearchResult;

    #[test]
    fn test_search_result_content_no_extra_characters() {
        // Create a SearchResult with content similar to what's shown in the issue
        let result = SearchResult {
            file: "/test/path".to_string(),
            uuid: "test-uuid".to_string(),
            timestamp: "2024-07-25T18:22:00Z".to_string(),
            session_id: "test-session".to_string(),
            role: "system".to_string(),
            text: "PostToolUse:Edit [ccth --debug] completed successfully: [dotenv@17.2.1] injecting env (0) from .env".to_string(),
            has_tools: false,
            has_thinking: false,
            message_type: "normal".to_string(),
            query: crate::query::condition::QueryCondition::Literal { 
                pattern: "test".to_string(), 
                case_sensitive: false 
            },
            project_path: "/test/project".to_string(),
            raw_json: Some("{}".to_string()),
        };

        // Check that get_content doesn't contain unexpected characters
        let content = result.get_content();
        println!("Content: '{}'", content);
        println!("Content length: {}", content.len());
        println!("Content bytes: {:?}", content.as_bytes());

        // Ensure there are no trailing spaces or special characters
        assert!(
            !content.contains("│"),
            "Content should not contain border characters"
        );
        assert!(
            !content.ends_with(" "),
            "Content should not end with spaces"
        );
        assert!(!content.contains("\t"), "Content should not contain tabs");

        // Test another example
        let result2 = SearchResult {
            file: "/test/path".to_string(),
            uuid: "test-uuid-2".to_string(),
            timestamp: "2024-07-25T18:22:00Z".to_string(),
            session_id: "test-session".to_string(),
            role: "system".to_string(),
            text: "Running PostToolUse:Edit...".to_string(),
            has_tools: false,
            has_thinking: false,
            message_type: "normal".to_string(),
            query: crate::query::condition::QueryCondition::Literal {
                pattern: "test".to_string(),
                case_sensitive: false,
            },
            project_path: "/test/project".to_string(),
            raw_json: Some("{}".to_string()),
        };

        let content2 = result2.get_content();
        println!("\nContent2: '{}'", content2);
        println!("Content2 length: {}", content2.len());

        assert!(
            !content2.contains("│"),
            "Content should not contain border characters"
        );
        assert!(
            !content2.ends_with(" "),
            "Content should not end with spaces"
        );
    }
}
