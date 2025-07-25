//! Main search view component

use crate::interactive_iocraft::ui::components::shared::{SearchBar, ResultList};
use crate::interactive_iocraft::ui::hooks::{use_search, use_terminal_events, use_debounced_search};
use crate::interactive_iocraft::ui::contexts::Theme;
use crate::interactive_iocraft::SearchResult;
use iocraft::prelude::*;
use futures::StreamExt;

#[derive(Default, Props)]
pub struct SearchViewProps<'a> {
    pub initial_query: Option<String>,
    pub file_pattern: String,
    pub on_select_result: Handler<'a, SearchResult>,
    pub on_show_help: Handler<'a, ()>,
}

#[component]
pub fn SearchView<'a>(mut hooks: Hooks, props: &mut SearchViewProps<'a>) -> impl Into<AnyElement<'a>> {
    let _theme = hooks.use_context::<Theme>();
    
    // State
    let role_filter = hooks.use_state(|| None::<String>);
    let truncate = hooks.use_state(|| true);
    let focused = hooks.use_state(|| true);
    let selected_index = hooks.use_state(|| 0usize);
    let scroll_offset = hooks.use_state(|| 0usize);
    
    // Use debounced search
    let (query, search_query, is_typing) = use_debounced_search(
        &mut hooks,
        props.initial_query.clone().unwrap_or_default(),
        300, // 300ms debounce delay
    );
    
    // Search hook with debounced query
    let search_results = use_search(
        &mut hooks,
        &search_query.read().clone(),
        &props.file_pattern,
        role_filter.read().clone(),
    );
    
    // States for Handler calls
    let mut pending_select = hooks.use_state(|| None::<SearchResult>);
    let mut pending_help = hooks.use_state(|| false);
    
    // Handle keyboard events
    let mut events = use_terminal_events(&mut hooks);
    
    hooks.use_future({
        let mut role_filter = role_filter.clone();
        let mut truncate = truncate.clone();
        let results = search_results.results.clone();
        let mut selected_index = selected_index.clone();
        let mut scroll_offset = scroll_offset.clone();
        let mut pending_select = pending_select.clone();
        let mut pending_help = pending_help.clone();
        let mut focused = focused.clone();
        
        async move {
            while let Some(event) = events.next().await {
                if let TerminalEvent::Key(key) = event {
                    let is_focused = *focused.read();
                    
                    match key.code {
                    // Special keys that work regardless of focus
                    KeyCode::Esc => {
                        if is_focused {
                            focused.set(false);
                        }
                    }
                    
                    KeyCode::Tab => {
                        let current = role_filter.read().clone();
                        let next = match current.as_deref() {
                            None => Some("user".to_string()),
                            Some("user") => Some("assistant".to_string()),
                            Some("assistant") => Some("system".to_string()),
                            Some("system") => None,
                            _ => None,
                        };
                        role_filter.set(next);
                    }
                    
                    // Toggle truncation
                    KeyCode::Char('t') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        let current = *truncate.read();
                        truncate.set(!current);
                    }
                    
                    // Keys that only work when not focused
                    _ if !is_focused => {
                        match key.code {
                        // Focus search bar
                        KeyCode::Char('/') => {
                            focused.set(true);
                        }
                        
                        // Navigation keys
                        KeyCode::Up | KeyCode::Char('k') => {
                            let current = *selected_index.read();
                            if current > 0 {
                                let new_selected = current - 1;
                                selected_index.set(new_selected);
                                
                                // Adjust scroll if needed
                                if new_selected < *scroll_offset.read() {
                                    scroll_offset.set(new_selected);
                                }
                            }
                        }
                        KeyCode::Down | KeyCode::Char('j') => {
                            let current = *selected_index.read();
                            let max = results.len().saturating_sub(1);
                            if current < max {
                                let new_selected = current + 1;
                                selected_index.set(new_selected);
                                
                                // Adjust scroll if needed
                                let visible_items = 20;
                                if new_selected >= *scroll_offset.read() + visible_items {
                                    scroll_offset.set(new_selected.saturating_sub(visible_items - 1));
                                }
                            }
                        }
                        
                        // Toggle truncation
                        KeyCode::Char('t') => {
                            let current = *truncate.read();
                            truncate.set(!current);
                        }
                        
                        // Select result
                        KeyCode::Enter => {
                            let selected = *selected_index.read();
                            if selected < results.len() {
                                pending_select.set(Some(results[selected].clone()));
                            }
                        }
                        
                        // Show help
                        KeyCode::Char('?') => {
                            pending_help.set(true);
                        }
                        
                        _ => {}
                        }
                    }
                    
                    // Keys that only work when focused
                    _ if is_focused => {
                        match key.code {
                        // Enter to select first result or unfocus
                        KeyCode::Enter => {
                            let selected = *selected_index.read();
                            if selected < results.len() {
                                pending_select.set(Some(results[selected].clone()));
                            }
                            focused.set(false);
                        }
                        
                        _ => {
                            // All other keys are handled by the TextInput
                        }
                        }
                    }
                    
                    _ => {}
                    }
                }
            }
        }
    });
    
    // Handle pending actions outside of async block
    let result_to_select = pending_select.read().clone();
    let should_show_help = *pending_help.read();
    
    if let Some(result) = result_to_select {
        pending_select.set(None);
        props.on_select_result.take()(result);
    }
    
    if should_show_help {
        pending_help.set(false);
        props.on_show_help.take()(());
    }
    
    // Determine status
    let status = if *is_typing.read() {
        Some("typing...".to_string())
    } else if search_results.loading {
        Some("searching...".to_string())
    } else if !query.read().is_empty() && search_results.results.is_empty() {
        Some("no results".to_string())
    } else {
        let truncation_status = if *truncate.read() {
            "[Truncated]"
        } else {
            "[Full Text]"
        };
        Some(format!("{} results {}", search_results.results.len(), truncation_status))
    };
    
    element! {
        Box(
            flex_direction: FlexDirection::Column,
            width: 100pct,
            height: 100pct,
        ) {
            // Search bar
            SearchBar(
                value: query.read().clone(),
                on_change: {
                    let mut query = query.clone();
                    let mut focused = focused.clone();
                    move |new_value: String| {
                        query.set(new_value);
                        // Keep focus when typing
                        if !*focused.read() {
                            focused.set(true);
                        }
                    }
                },
                role_filter: role_filter.read().clone(),
                on_role_filter_toggle: None,  // Toggle is handled in the keyboard event handler
                status: status,
                message: search_results.error.clone(),
                focused: *focused.read(),
            )
            
            // Results list
            ResultList(
                results: search_results.results.clone(),
                selected: *selected_index.read(),
                scroll_offset: *scroll_offset.read(),
                on_select: move |_idx| {
                    // Selection is handled by keyboard navigation
                },
                truncate: *truncate.read(),
            )
        }
    }
}