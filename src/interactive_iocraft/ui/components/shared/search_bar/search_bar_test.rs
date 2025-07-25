#[cfg(test)]
mod tests {
    use crate::interactive_iocraft::ui::components::shared::{SearchBar, SearchBarProps};
    use crate::interactive_iocraft::ui::contexts::Theme;
    use iocraft::prelude::*;
    use futures::stream::{self, StreamExt};
    use smol_macros::test;
    use macro_rules_attribute::apply;

    #[component]
    fn TestSearchBar(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
        let query = hooks.use_state(|| "test query".to_string());
        let focused = hooks.use_state(|| true);
        let role_filter = hooks.use_state(|| None::<String>);
        
        element! {
            SearchBar(
                value: query.read().clone(),
                on_change: {
                    let mut query = query.clone();
                    move |new_value: String| {
                        query.set(new_value);
                    }
                },
                role_filter: role_filter.read().clone(),
                on_role_filter_toggle: None,
                status: Some("searching...".to_string()),
                message: Some("Error message".to_string()),
                focused: *focused.read(),
            )
        }
    }

    #[apply(test!)]
    async fn test_search_bar_renders() {
        let actual = element!(TestSearchBar)
            .mock_terminal_render_loop(MockTerminalConfig::default())
            .map(|c| c.to_string())
            .take(1)
            .collect::<Vec<_>>()
            .await;
        
        assert!(!actual.is_empty());
        let output = &actual[0];
        
        // Check that key elements are rendered
        assert!(output.contains("test query"));
        assert!(output.contains("searching..."));
        assert!(output.contains("Error message"));
        assert!(output.contains("Press") && output.contains("Tab"));
    }

    #[apply(test!)]
    async fn test_search_bar_with_role_filter() {
        #[component]
        fn TestWithRoleFilter(_hooks: Hooks) -> impl Into<AnyElement<'static>> {
            element! {
                SearchBar(
                    value: "query".to_string(),
                    on_change: |_| {},
                    role_filter: Some("user".to_string()),
                    on_role_filter_toggle: None,
                    status: None,
                    message: None,
                    focused: false,
                )
            }
        }
        
        let actual = element!(TestWithRoleFilter)
            .mock_terminal_render_loop(MockTerminalConfig::default())
            .map(|c| c.to_string())
            .take(1)
            .collect::<Vec<_>>()
            .await;
        
        assert!(!actual.is_empty());
        let output = &actual[0];
        
        // Check that role filter is displayed
        assert!(output.contains("[user]"));
    }

    #[apply(test!)]
    async fn test_search_bar_focused_state() {
        #[component]
        fn TestFocusedState(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
            let focused = hooks.use_state(|| true);
            
            element! {
                SearchBar(
                    value: "test".to_string(),
                    on_change: |_| {},
                    role_filter: None,
                    on_role_filter_toggle: None,
                    status: None,
                    message: None,
                    focused: *focused.read(),
                )
            }
        }
        
        let actual = element!(TestFocusedState)
            .mock_terminal_render_loop(MockTerminalConfig::default())
            .map(|c| c.to_string())
            .take(1)
            .collect::<Vec<_>>()
            .await;
        
        assert!(!actual.is_empty());
        // The focused state should affect border styling
        // (actual validation would depend on terminal output format)
    }
}