//! Session view component for browsing full session

use crate::interactive_iocraft::application::{SessionService, CacheService};
use crate::interactive_iocraft::domain::models::SessionOrder;
use crate::interactive_iocraft::domain::session_list_item::SessionListItem;
use crate::interactive_iocraft::ui::contexts::Theme;
use crate::interactive_iocraft::ui::hooks::{use_terminal_events, use_clipboard, copy_to_clipboard};
use crate::interactive_iocraft::ui::components::shared::SearchBar;
use iocraft::prelude::*;
use futures::StreamExt;
use std::sync::{Arc, Mutex};

#[derive(Props)]
pub struct SessionViewProps {
    pub file_path: String,
}

impl Default for SessionViewProps {
    fn default() -> Self {
        panic!("SessionViewProps cannot be default constructed")
    }
}

#[component]
pub fn SessionView(mut hooks: Hooks, props: &SessionViewProps) -> impl Into<AnyElement<'static>> {
    let theme = hooks.use_context::<Theme>();
    let session_service = hooks.use_context::<Arc<SessionService>>();
    let cache_service = hooks.use_context::<Arc<Mutex<CacheService>>>();
    let clipboard = use_clipboard(&mut hooks);
    
    // State
    let mut messages = hooks.use_state(Vec::<SessionListItem>::new);
    let mut raw_messages = hooks.use_state(Vec::<String>::new);
    let mut search_query = hooks.use_state(String::new);
    let mut filtered_indices = hooks.use_state(Vec::<usize>::new);
    let mut selected_index = hooks.use_state(|| 0usize);
    let mut scroll_offset = hooks.use_state(|| 0usize);
    let mut order = hooks.use_state(|| None::<SessionOrder>);
    let mut is_searching = hooks.use_state(|| false);
    let mut truncate = hooks.use_state(|| true);
    let mut session_id = hooks.use_state(|| None::<String>);
    let mut loading = hooks.use_state(|| false);
    let mut error = hooks.use_state(|| None::<String>);
    
    // Load session messages on mount
    hooks.use_future({
        let file_path = props.file_path.clone();
        let session_service = session_service.clone();
        let cache_service = cache_service.clone();
        let mut messages = messages.clone();
        let mut raw_messages = raw_messages.clone();
        let mut session_id = session_id.clone();
        let mut loading = loading.clone();
        let mut error = error.clone();
        
        async move {
            loading.set(true);
            error.set(None);
            
            // Try to get from cache first
            let cache_result = match cache_service.lock() {
                Ok(cache) => cache.get(&file_path).cloned(),
                Err(e) => {
                    error.set(Some(format!("Failed to acquire cache lock: {}", e)));
                    loading.set(false);
                    return;
                }
            };
            
            let session_data = if let Some(data) = cache_result {
                data
            } else {
                // Load from file
                match session_service.get_raw_lines(&file_path) {
                    Ok(messages) => {
                        // Cache the result
                        match cache_service.lock() {
                            Ok(mut cache) => {
                                cache.put(file_path.clone(), messages.clone());
                                messages
                            }
                            Err(e) => {
                                error.set(Some(format!("Failed to acquire cache lock: {}", e)));
                                loading.set(false);
                                return;
                            }
                        }
                    }
                    Err(e) => {
                        error.set(Some(format!("Failed to load session: {}", e)));
                        loading.set(false);
                        return;
                    }
                }
            };
            
            // Extract session ID from messages if possible
            if let Some(first_msg) = session_data.first() {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(first_msg) {
                    if let Some(id) = json.get("sessionId").and_then(|v| v.as_str()) {
                        session_id.set(Some(id.to_string()));
                    }
                }
            }
            
            // Convert to SessionListItems
            let items: Vec<SessionListItem> = session_data
                .iter()
                .enumerate()
                .filter_map(|(idx, line)| SessionListItem::from_json_line(idx, line))
                .collect();
            
            raw_messages.set(session_data);
            messages.set(items);
            loading.set(false);
        }
    });
    
    // Apply search filter immediately based on current state
    {
        let query = search_query.read().clone();
        let all_messages = messages.read();
        let current_filtered = filtered_indices.read();
        
        if query.is_empty() && current_filtered.len() != all_messages.len() {
            // No filter - show all messages
            let indices: Vec<usize> = (0..all_messages.len()).collect();
            drop(current_filtered);
            filtered_indices.set(indices);
        } else if !query.is_empty() {
            // Filter messages containing the query (case-insensitive)
            let query_lower = query.to_lowercase();
            let indices: Vec<usize> = all_messages
                .iter()
                .enumerate()
                .filter(|(_, msg)| {
                    msg.text.to_lowercase().contains(&query_lower) ||
                    msg.role.to_lowercase().contains(&query_lower)
                })
                .map(|(idx, _)| idx)
                .collect();
            
            if indices != *current_filtered {
                drop(current_filtered);
                filtered_indices.set(indices);
                // Reset selection and scroll when filter changes
                selected_index.set(0);
                scroll_offset.set(0);
            }
        }
    }
    
    // Handle terminal events
    let mut events = use_terminal_events(&mut hooks);
    
    hooks.use_future({
        let mut is_searching = is_searching.clone();
        let mut search_query = search_query.clone();
        let mut order = order.clone();
        let mut truncate = truncate.clone();
        let messages = messages.clone();
        let raw_messages = raw_messages.clone();
        let mut selected_index = selected_index.clone();
        let mut scroll_offset = scroll_offset.clone();
        let clipboard_message = clipboard.message.clone();
        let filtered_indices = filtered_indices.clone();
        let file_path = props.file_path.clone();
        
        async move {
            while let Some(event) = events.next().await {
                if let TerminalEvent::Key(key) = event {
                    if *is_searching.read() {
                        match key.code {
                            KeyCode::Esc => {
                                is_searching.set(false);
                                search_query.set(String::new());
                            }
                            KeyCode::Enter => {
                                is_searching.set(false);
                                // Filter is already applied by use_effect
                            }
                            _ => {}
                        }
                    } else {
                        match key.code {
                            // Navigation
                            KeyCode::Up | KeyCode::Char('k') => {
                                let current = *selected_index.read();
                                if current > 0 {
                                    selected_index.set(current - 1);
                                    
                                    // Adjust scroll if needed
                                    if current - 1 < *scroll_offset.read() {
                                        scroll_offset.set(current - 1);
                                    }
                                }
                            }
                            KeyCode::Down | KeyCode::Char('j') => {
                                let current = *selected_index.read();
                                let max = filtered_indices.read().len().saturating_sub(1);
                                if current < max {
                                    selected_index.set(current + 1);
                                    
                                    // Adjust scroll if needed
                                    let visible_height = 20; // Approximate
                                    if current + 1 >= *scroll_offset.read() + visible_height {
                                        scroll_offset.set((current + 1).saturating_sub(visible_height - 1));
                                    }
                                }
                            }
                            
                            // Search
                            KeyCode::Char('/') => {
                                is_searching.set(true);
                                search_query.set(String::new());
                            }
                            
                            // Copy JSON of selected message
                            KeyCode::Char('c') => {
                                let selected_idx = *selected_index.read();
                                let indices = filtered_indices.read();
                                if selected_idx < indices.len() {
                                    let actual_idx = indices[selected_idx];
                                    if actual_idx < raw_messages.read().len() {
                                        let _ = copy_to_clipboard(&raw_messages.read()[actual_idx]);
                                    }
                                }
                            }
                            
                            // Copy all messages (filtered if search active)
                            KeyCode::Char('C') => {
                                let indices = filtered_indices.read();
                                let raws = raw_messages.read();
                                if indices.is_empty() || search_query.read().is_empty() {
                                    // Copy all messages
                                    let _ = copy_to_clipboard(&raws.join("\n\n"));
                                } else {
                                    // Copy only filtered messages
                                    let filtered: Vec<String> = indices
                                        .iter()
                                        .filter_map(|&idx| raws.get(idx).cloned())
                                        .collect();
                                    let _ = copy_to_clipboard(&filtered.join("\n\n"));
                                }
                            }
                            
                            // Copy session ID
                            KeyCode::Char('i') | KeyCode::Char('I') => {
                                if let Some(id) = session_id.read().clone() {
                                    let _ = copy_to_clipboard(&id);
                                }
                            }
                            
                            // Copy file path
                            KeyCode::Char('f') | KeyCode::Char('F') => {
                                let _ = copy_to_clipboard(&file_path);
                            }
                            
                            // Copy message text
                            KeyCode::Char('m') | KeyCode::Char('M') => {
                                let selected_idx = *selected_index.read();
                                let messages = messages.read();
                                let indices = filtered_indices.read();
                                
                                // Find the actual message based on filtered index
                                if !indices.is_empty() && selected_idx < indices.len() {
                                    let actual_idx = indices[selected_idx];
                                    if let Some(msg) = messages.get(actual_idx) {
                                        let _ = copy_to_clipboard(&msg.text);
                                    }
                                } else if indices.is_empty() && selected_idx < messages.len() {
                                    // No filter, use direct index
                                    if let Some(msg) = messages.get(selected_idx) {
                                        let _ = copy_to_clipboard(&msg.text);
                                    }
                                }
                            }
                            
                            // Sort order
                            KeyCode::Char('o') => {
                                let current = order.read().clone();
                                let next = match current {
                                    None => Some(SessionOrder::Ascending),
                                    Some(SessionOrder::Ascending) => Some(SessionOrder::Descending),
                                    Some(SessionOrder::Descending) => Some(SessionOrder::Original),
                                    Some(SessionOrder::Original) => None,
                                };
                                order.set(next);
                            }
                            
                            // Toggle truncation
                            KeyCode::Char('t') if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                                let current = *truncate.read();
                                truncate.set(!current);
                            }
                            
                            // Also support Ctrl+T for consistency
                            KeyCode::Char('t') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                                let current = *truncate.read();
                                truncate.set(!current);
                            }
                            
                            _ => {}
                        }
                    }
                }
            }
        }
    });
    
    // Apply order and filter to messages
    let displayed_messages = {
        let all_messages = messages.read();
        let indices = filtered_indices.read();
        
        if indices.is_empty() && !search_query.read().is_empty() {
            // Search active but no results
            vec![]
        } else if indices.is_empty() {
            // No search, show all messages with ordering
            let mut msgs = all_messages.clone();
            match order.read().clone() {
                Some(SessionOrder::Ascending) => msgs.sort_by_key(|m| m.timestamp.clone()),
                Some(SessionOrder::Descending) => {
                    msgs.sort_by_key(|m| m.timestamp.clone());
                    msgs.reverse();
                }
                Some(SessionOrder::Original) | None => {}
            }
            msgs
        } else {
            // Apply filter
            let mut msgs: Vec<SessionListItem> = indices
                .iter()
                .filter_map(|&idx| all_messages.get(idx).cloned())
                .collect();
            
            // Then apply ordering
            match order.read().clone() {
                Some(SessionOrder::Ascending) => msgs.sort_by_key(|m| m.timestamp.clone()),
                Some(SessionOrder::Descending) => {
                    msgs.sort_by_key(|m| m.timestamp.clone());
                    msgs.reverse();
                }
                Some(SessionOrder::Original) | None => {}
            }
            msgs
        }
    };
    
    element! {
        Box(
            flex_direction: FlexDirection::Column,
            width: 100pct,
            height: 100pct,
        ) {
            // Title bar
            Box(
                flex_direction: FlexDirection::Column,
                border_style: theme.default_border,
                border_color: theme.primary_color,
                padding: 1,
            ) {
                Text(
                    content: "Session Viewer".to_string(),
                    weight: Weight::Bold,
                    color: theme.primary_color,
                )
                Box(margin_top: 1) {
                    Text(
                        content: if let Some(id) = session_id.read().clone() {
                            format!("Session: {} | File: {}", id, props.file_path)
                        } else {
                            format!("File: {}", props.file_path)
                        },
                        color: Color::DarkGrey,
                    )
                }
                #(if !search_query.read().is_empty() {
                    element! {
                        Box(margin_top: 1) {
                            Text(
                                content: format!("Searching: '{}'", &*search_query.read()),
                                color: theme.accent_color,
                            )
                        }
                    }
                } else {
                    element! { Box() }
                })
            }
            
            // Search bar or info bar
            #(if *is_searching.read() {
                element! {
                    SearchBar(
                        value: search_query.read().clone(),
                        on_change: {
                            let mut search_query = search_query.clone();
                            move |new_value: String| {
                                search_query.set(new_value);
                            }
                        },
                        role_filter: None,
                        on_role_filter_toggle: None,
                        status: Some("Search in session (Esc to cancel)".to_string()),
                        message: None,
                        focused: true,
                    )
                }.into_any()
            } else {
                element! {
                    Box(
                        border_style: theme.default_border,
                        border_color: theme.primary_color,
                        padding: 1,
                    ) {
                        Text(
                            content: format!(
                                "Messages: {} (filtered: {}) | Order: {} | Press '/' to search",
                                messages.read().len(),
                                displayed_messages.len(),
                                match order.read().clone() {
                                    Some(SessionOrder::Ascending) => "Ascending",
                                    Some(SessionOrder::Descending) => "Descending",
                                    Some(SessionOrder::Original) => "Original",
                                    None => "Default",
                                }
                            ),
                            color: Color::Reset,
                        )
                    }
                }.into_any()
            })
            
            // Messages list
            Box(
                flex_grow: 1.0,
                border_style: theme.default_border,
                border_color: theme.primary_color,
                padding: 1,
                position: Position::Relative,
            ) {
                #(if *loading.read() {
                    element! {
                        Box(
                            align_items: AlignItems::Center,
                            justify_content: JustifyContent::Center,
                            width: 100pct,
                            height: 100pct,
                        ) {
                            Text(
                                content: "Loading session...".to_string(),
                                color: theme.accent_color,
                            )
                        }
                    }
                } else if let Some(err) = error.read().clone() {
                    element! {
                        Box(
                            align_items: AlignItems::Center,
                            justify_content: JustifyContent::Center,
                            width: 100pct,
                            height: 100pct,
                        ) {
                            Text(
                                content: err,
                                color: theme.error_color,
                            )
                        }
                    }
                } else if displayed_messages.is_empty() {
                    element! {
                        Box(
                            align_items: AlignItems::Center,
                            justify_content: JustifyContent::Center,
                            width: 100pct,
                            height: 100pct,
                        ) {
                            Text(
                                content: if !search_query.read().is_empty() {
                                    format!("No messages matching '{}'", &*search_query.read())
                                } else {
                                    "No messages in session".to_string()
                                },
                                color: Color::DarkGrey,
                            )
                        }
                    }
                } else {
                    element! {
                        Box(flex_direction: FlexDirection::Column) {
                            // Scroll position indicator
                            #(if displayed_messages.len() > 20 {
                                let visible_end = (*scroll_offset.read() + 20).min(displayed_messages.len());
                                element! {
                                    Box(
                                        position: Position::Absolute,
                                        top: 0,
                                        right: 0,
                                        background_color: theme.primary_color,
                                        padding_left: 1,
                                        padding_right: 1,
                                    ) {
                                        Text(
                                            content: format!("{}-{} of {}", 
                                                *scroll_offset.read() + 1, 
                                                visible_end,
                                                displayed_messages.len()
                                            ),
                                            color: Color::White,
                                            weight: Weight::Bold,
                                        )
                                    }
                                }
                            } else {
                                element! { Box() }
                            })
                            
                            #(displayed_messages.iter().enumerate().skip(*scroll_offset.read()).take(20).map(|(idx, msg)| {
                                let is_selected = idx == *selected_index.read();
                                element! {
                                    SessionMessage(
                                        message: msg.clone(),
                                        index: idx,
                                        selected: is_selected,
                                        truncate: *truncate.read(),
                                    )
                                }
                            }))
                        }
                    }
                })
            }
            
            // Status bar
            Box(
                flex_direction: FlexDirection::Column,
                padding: 1,
                background_color: Color::DarkGrey,
            ) {
                Box(flex_direction: FlexDirection::Row) {
                    Text(content: "↑/↓ j/k: Navigate | ", color: Color::White)
                    Text(content: "o: Sort | ", color: Color::White)
                    Text(content: "c: Copy JSON | ", color: Color::White)
                    Text(content: "C: Copy all | ", color: Color::White)
                    Text(content: "m: Message text | ", color: Color::White)
                    Text(content: "i: Session ID", color: Color::White)
                }
                Box(flex_direction: FlexDirection::Row) {
                    Text(content: "f: File path | ", color: Color::White)
                    Text(content: "t: Toggle truncate | ", color: Color::White)
                    Text(content: "/: Search | ", color: Color::White)
                    Text(content: "Ctrl+T: Truncate | ", color: Color::White)
                    Text(content: "Esc: Back", color: Color::White)
                }
            }
            
            // Clipboard message
            #(if let Some(msg) = clipboard.message {
                element! {
                    Box(
                        position: Position::Absolute,
                        top: 40pct,
                        left: 40pct,
                        background_color: theme.success_color,
                        padding: 2,
                        border_style: theme.default_border,
                        border_color: Color::White,
                    ) {
                        Text(
                            content: msg,
                            color: Color::White,
                            weight: Weight::Bold,
                        )
                    }
                }.into_any()
            } else {
                element! { Box() }.into_any()
            })
        }
    }
}

// Session message component
#[derive(Props)]
struct SessionMessageProps {
    message: SessionListItem,
    index: usize,
    selected: bool,
    truncate: bool,
}

impl Default for SessionMessageProps {
    fn default() -> Self {
        panic!("SessionMessageProps cannot be default constructed")
    }
}

#[component]
fn SessionMessage(mut hooks: Hooks, props: &SessionMessageProps) -> impl Into<AnyElement<'static>> {
    let theme = hooks.use_context::<Theme>();
    
    let background_color = if props.selected {
        theme.highlight_color
    } else {
        Color::Reset
    };
    
    let text_color = theme.role_color(&props.message.role);
    
    // Format timestamp
    let timestamp = props.message.timestamp
        .chars()
        .take(19)
        .collect::<String>()
        .replace('T', " ");
    
    // Truncate text if needed
    let text = if props.truncate && props.message.text.len() > 100 {
        format!("{}...", props.message.text.chars().take(97).collect::<String>())
    } else {
        props.message.text.clone()
    };
    
    element! {
        Box(
            flex_direction: FlexDirection::Row,
            background_color: background_color,
            padding_left: 1,
            padding_right: 1,
        ) {
            // Index
            Box(width: 5) {
                Text(
                    content: format!("{:4}", props.index),
                    color: Color::DarkGrey,
                )
            }
            
            // Timestamp
            Box(width: 20, margin_right: 1) {
                Text(
                    content: timestamp,
                    color: Color::DarkGrey,
                )
            }
            
            // Role
            Box(width: 12, margin_right: 1) {
                Text(
                    content: format!("{:10}", props.message.role),
                    color: text_color,
                    weight: Weight::Bold,
                )
            }
            
            // Text
            Box(flex_grow: 1.0) {
                Text(
                    content: text,
                    color: Color::Reset,
                )
            }
        }
    }
}