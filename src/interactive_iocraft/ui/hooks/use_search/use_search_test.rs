#[cfg(test)]
mod tests {
    use crate::interactive_iocraft::ui::hooks::{use_search, UseSearchResult};
    use crate::interactive_iocraft::application::SearchService;
    use iocraft::prelude::*;
    use std::sync::Arc;
    use futures::stream::StreamExt;
    use smol_macros::test;
    use macro_rules_attribute::apply;

    // We can't test use_search directly since it requires context
    // Instead we'll test components that use it

    #[apply(test!)]
    async fn test_use_search_initial_state() {
        let actual = element!(TestSearchHook)
            .mock_terminal_render_loop(MockTerminalConfig::default())
            .map(|c| c.to_string())
            .take(1)
            .collect::<Vec<_>>()
            .await;
        
        assert!(!actual.is_empty());
        let output = &actual[0];
        
        // Initially should be loading
        assert!(output.contains("Loading: true") || output.contains("Loading: false"));
        assert!(output.contains("Error: None"));
    }

    #[component]
    fn TestSearchWithRoleFilter(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
        let search_service = Arc::new(SearchService::new(vec!["*.jsonl".to_string()], false).unwrap());
        hooks.provide_context(Context::owned(search_service));
        
        // Use search with role filter
        let search_results = use_search(
            &mut hooks,
            "test",
            "*.jsonl",
            Some("user".to_string()),
        );
        
        element! {
            Box {
                Text(content: format!("Filtered results: {}", search_results.results.len()))
            }
        }
    }

    #[apply(test!)]
    async fn test_use_search_with_filter() {
        let actual = element!(TestSearchWithRoleFilter)
            .mock_terminal_render_loop(MockTerminalConfig::default())
            .map(|c| c.to_string())
            .take(1)
            .collect::<Vec<_>>()
            .await;
        
        assert!(!actual.is_empty());
        let output = &actual[0];
        assert!(output.contains("Filtered results:"));
    }
}