#[cfg(test)]
mod tests {
    use crate::interactive_iocraft::ui::components::shared::{ResultList, ResultListProps};
    use crate::interactive_iocraft::ui::contexts::Theme;
    use crate::interactive_iocraft::SearchResult;
    use iocraft::prelude::*;
    use futures::stream::{self, StreamExt};
    use smol_macros::test;
    use macro_rules_attribute::apply;

    fn create_test_results() -> Vec<SearchResult> {
        use crate::query::condition::QueryCondition;
        
        vec![
            SearchResult {
                file: "/path/to/file1.jsonl".to_string(),
                uuid: "uuid1".to_string(),
                session_id: "session1".to_string(),
                timestamp: "1234567890".to_string(),
                role: "user".to_string(),
                text: "This is a test query".to_string(),
                has_tools: false,
                has_thinking: false,
                message_type: "message".to_string(),
                query: QueryCondition::All,
            },
            SearchResult {
                file: "/path/to/file2.jsonl".to_string(),
                uuid: "uuid2".to_string(),
                session_id: "session2".to_string(),
                timestamp: "1234567891".to_string(),
                role: "assistant".to_string(),
                text: "This is a response to the query".to_string(),
                has_tools: false,
                has_thinking: false,
                message_type: "message".to_string(),
                query: QueryCondition::All,
            },
        ]
    }

    #[component]
    fn TestResultList(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
        let results = create_test_results();
        let selected = hooks.use_state(|| 0usize);
        
        element! {
            ResultList(
                results: results,
                selected: *selected.read(),
                scroll_offset: 0,
                on_select: move |idx| {
                    selected.set(idx);
                },
                truncate: true,
                max_width: 80,
            )
        }
    }

    #[apply(test!)]
    async fn test_result_list_renders() {
        let actual = element!(TestResultList)
            .mock_terminal_render_loop(MockTerminalConfig::default())
            .map(|c| c.to_string())
            .take(1)
            .collect::<Vec<_>>()
            .await;
        
        assert!(!actual.is_empty());
        let output = &actual[0];
        
        // Check that results are rendered
        assert!(output.contains("user"));
        assert!(output.contains("assistant"));
        assert!(output.contains("This is a test query"));
        assert!(output.contains("This is a response to the query"));
    }

    #[apply(test!)]
    async fn test_result_list_empty() {
        #[component]
        fn TestEmptyList(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
            element! {
                ResultList(
                    results: Vec::new(),
                    selected: 0,
                    scroll_offset: 0,
                    on_select: |_| {},
                    truncate: true,
                    max_width: 80,
                )
            }
        }
        
        let actual = element!(TestEmptyList)
            .mock_terminal_render_loop(MockTerminalConfig::default())
            .map(|c| c.to_string())
            .take(1)
            .collect::<Vec<_>>()
            .await;
        
        assert!(!actual.is_empty());
        let output = &actual[0];
        
        // Check that "No results found" is displayed
        assert!(output.contains("No results found"));
    }

    #[apply(test!)]
    async fn test_result_list_selection() {
        #[component]
        fn TestSelection(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
            let results = create_test_results();
            
            element! {
                ResultList(
                    results: results,
                    selected: 1, // Second item selected
                    scroll_offset: 0,
                    on_select: |_| {},
                    truncate: true,
                    max_width: 80,
                )
            }
        }
        
        let actual = element!(TestSelection)
            .mock_terminal_render_loop(MockTerminalConfig::default())
            .map(|c| c.to_string())
            .take(1)
            .collect::<Vec<_>>()
            .await;
        
        assert!(!actual.is_empty());
        // The selected item should be highlighted differently
        // (actual validation would depend on terminal output format)
    }

    #[apply(test!)]
    async fn test_result_list_truncation() {
        #[component]
        fn TestTruncation(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
            let mut results = create_test_results();
            results[0].text = "This is a very long text that should be truncated when displayed in the result list because it exceeds the maximum width".to_string();
            
            element! {
                ResultList(
                    results: results,
                    selected: 0,
                    scroll_offset: 0,
                    on_select: |_| {},
                    truncate: true,
                    max_width: 50, // Small width to force truncation
                )
            }
        }
        
        let actual = element!(TestTruncation)
            .mock_terminal_render_loop(MockTerminalConfig::default())
            .map(|c| c.to_string())
            .take(1)
            .collect::<Vec<_>>()
            .await;
        
        assert!(!actual.is_empty());
        let output = &actual[0];
        
        // Check that truncation indicator is present
        assert!(output.contains("..."));
    }
}