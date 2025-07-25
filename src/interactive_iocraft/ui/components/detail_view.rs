//! Detail view component for showing full message content

use crate::interactive_iocraft::ui::contexts::Theme;
use crate::interactive_iocraft::ui::hooks::{use_terminal_events, use_clipboard, copy_to_clipboard};
use crate::interactive_iocraft::SearchResult;
use iocraft::prelude::*;
use futures::StreamExt;
// crossterm::event::KeyCode import removed - using iocraft::KeyCode from prelude
use chrono::{Local, TimeZone};

#[derive(Props)]
pub struct DetailViewProps<'a> {
    pub result: SearchResult,
    pub on_view_session: Handler<'a, String>,
}

impl<'a> Default for DetailViewProps<'a> {
    fn default() -> Self {
        panic!("DetailViewProps cannot be default constructed")
    }
}

#[component]
pub fn DetailView<'a>(mut hooks: Hooks, props: &mut DetailViewProps<'a>) -> impl Into<AnyElement<'a>> {
    let theme = hooks.use_context::<Theme>();
    let clipboard = use_clipboard(&mut hooks);
    let mut events = use_terminal_events(&mut hooks);
    
    // State for Handler call
    let mut pending_session = hooks.use_state(|| None::<String>);
    
    // Scroll state
    let scroll_offset = hooks.use_state(|| 0usize);
    let mut total_lines = hooks.use_state(|| 0usize);
    
    // Format timestamp
    let timestamp = props.result.timestamp
        .parse::<i64>()
        .ok()
        .and_then(|ts| Local.timestamp_opt(ts, 0).single())
        .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
        .unwrap_or_else(|| props.result.timestamp.clone());
    
    // Handle keyboard events
    hooks.use_future({
        let result = props.result.clone();
        let file_path = props.result.file.clone();
        let mut pending_session = pending_session.clone();
        let mut scroll_offset = scroll_offset.clone();
        let total_lines = total_lines.clone();
        
        async move {
            while let Some(event) = events.next().await {
                if let TerminalEvent::Key(key) = event {
                    match key.code {
                    // Scroll navigation
                    KeyCode::Up | KeyCode::Char('k') => {
                        let current = *scroll_offset.read();
                        if current > 0 {
                            scroll_offset.set(current - 1);
                        }
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        let current = *scroll_offset.read();
                        let max_scroll = total_lines.read().saturating_sub(10); // Approximate visible lines
                        if current < max_scroll {
                            scroll_offset.set(current + 1);
                        }
                    }
                    KeyCode::PageUp => {
                        let current = *scroll_offset.read();
                        scroll_offset.set(current.saturating_sub(10));
                    }
                    KeyCode::PageDown => {
                        let current = *scroll_offset.read();
                        let max_scroll = total_lines.read().saturating_sub(10);
                        scroll_offset.set((current + 10).min(max_scroll));
                    }
                    KeyCode::Home => {
                        scroll_offset.set(0);
                    }
                    KeyCode::End => {
                        let max_scroll = total_lines.read().saturating_sub(10);
                        scroll_offset.set(max_scroll);
                    }
                    // Copy message content
                    KeyCode::Char('c') | KeyCode::Char('C') => {
                        let _ = copy_to_clipboard(&result.text);
                    }
                    // Copy file path
                    KeyCode::Char('f') | KeyCode::Char('F') => {
                        let _ = copy_to_clipboard(&result.file);
                    }
                    // Copy session ID
                    KeyCode::Char('i') | KeyCode::Char('I') => {
                        let _ = copy_to_clipboard(&result.session_id);
                    }
                    // Copy project path
                    KeyCode::Char('p') | KeyCode::Char('P') => {
                        let _ = copy_to_clipboard(&result.project_path);
                    }
                    // Copy message text (same as 'c')
                    KeyCode::Char('m') | KeyCode::Char('M') => {
                        let _ = copy_to_clipboard(&result.text);
                    }
                    // Copy raw JSON
                    KeyCode::Char('r') | KeyCode::Char('R') => {
                        if let Some(raw_json) = &result.raw_json {
                            let _ = copy_to_clipboard(raw_json);
                        } else {
                            // Fallback: create formatted output
                            let formatted = format!(
                                "File: {}\nUUID: {}\nTimestamp: {}\nSession ID: {}\nRole: {}\nText: {}\nProject: {}",
                                result.file,
                                result.uuid,
                                result.timestamp,
                                result.session_id,
                                result.role,
                                result.text,
                                result.project_path
                            );
                            let _ = copy_to_clipboard(&formatted);
                        }
                    }
                    // Copy URL
                    KeyCode::Char('u') | KeyCode::Char('U') => {
                        if !result.session_id.is_empty() {
                            let url = format!("https://claude.ai/chat/{}", result.session_id);
                            let _ = copy_to_clipboard(&url);
                        }
                    }
                    // View full session
                    KeyCode::Char('s') | KeyCode::Char('S') => {
                        pending_session.set(Some(file_path.clone()));
                    }
                    _ => {}
                    }
                }
            }
        }
    });
    
    // Handle pending session view
    let path_to_view = pending_session.read().clone();
    if let Some(path) = path_to_view {
        pending_session.set(None);
        (props.on_view_session)(path);
    }
    
    element! {
        Box(
            flex_direction: FlexDirection::Column,
            width: 100pct,
            height: 100pct,
            padding: 1,
        ) {
            // Header
            Box(
                flex_direction: FlexDirection::Column,
                border_style: theme.default_border,
                border_color: theme.primary_color,
                padding: 1,
                margin_bottom: 1,
            ) {
                // File path
                Box(flex_direction: FlexDirection::Row) {
                    Text(
                        content: "File: ".to_string(),
                        weight: Weight::Bold,
                        color: theme.primary_color,
                    )
                    Text(
                        content: props.result.file.clone(),
                        color: Color::Reset,
                    )
                }
                
                // Role and timestamp
                Box(flex_direction: FlexDirection::Row, margin_top: 1) {
                    Text(
                        content: "Role: ".to_string(),
                        weight: Weight::Bold,
                        color: theme.primary_color,
                    )
                    Text(
                        content: props.result.role.clone(),
                        color: theme.role_color(&props.result.role),
                        weight: Weight::Bold,
                    )
                    Box(margin_left: 2, margin_right: 2) {
                        Text(
                            content: " | ".to_string(),
                            color: Color::DarkGrey,
                        )
                    }
                    Text(
                        content: "Time: ".to_string(),
                        weight: Weight::Bold,
                        color: theme.primary_color,
                    )
                    Text(
                        content: timestamp,
                        color: Color::Reset,
                    )
                }
                
                // Project path
                Box(flex_direction: FlexDirection::Row, margin_top: 1) {
                    Text(
                        content: "Project: ".to_string(),
                        weight: Weight::Bold,
                        color: theme.primary_color,
                    )
                    Text(
                        content: props.result.project_path.clone(),
                        color: Color::Reset,
                    )
                }
                
                // UUID
                Box(flex_direction: FlexDirection::Row, margin_top: 1) {
                    Text(
                        content: "UUID: ".to_string(),
                        weight: Weight::Bold,
                        color: theme.primary_color,
                    )
                    Text(
                        content: props.result.uuid.clone(),
                        color: Color::Reset,
                    )
                }
                
                // Session ID if available
                #(if !props.result.session_id.is_empty() {
                    element! {
                        Box(flex_direction: FlexDirection::Row, margin_top: 1) {
                            Text(
                                content: "Session: ".to_string(),
                                weight: Weight::Bold,
                                color: theme.primary_color,
                            )
                            Text(
                                content: props.result.session_id.clone(),
                                color: theme.accent_color,
                            )
                        }
                    }
                } else {
                    element! { Box() }
                })
            }
            
            // Content with scroll support
            Box(
                flex_grow: 1.0,
                border_style: theme.default_border,
                border_color: theme.primary_color,
                padding: 1,
            ) {
                // Calculate total lines and visible content
                #({
                    // Count lines in the text
                    let text = &props.result.text;
                    let line_count = text.lines().count().max(1);
                    total_lines.set(line_count);
                    
                    // Get visible lines
                    let offset = *scroll_offset.read();
                    let visible_lines: Vec<&str> = text
                        .lines()
                        .skip(offset)
                        .take(20) // Approximate visible height
                        .collect();
                    
                    let visible_text = visible_lines.join("\n");
                    
                    element! {
                        Box(flex_direction: FlexDirection::Column) {
                            // Scroll position indicator
                            #(if line_count > 1 {
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
                                            content: format!("Line {}/{}", offset + 1, line_count),
                                            color: Color::White,
                                            weight: Weight::Bold,
                                        )
                                    }
                                }
                            } else {
                                element! { Box() }
                            })
                            
                            Text(
                                content: visible_text,
                                color: Color::Reset,
                            )
                        }
                    }
                })
            }
            
            // Footer with keyboard shortcuts
            Box(
                flex_direction: FlexDirection::Column,
                margin_top: 1,
                padding: 1,
            ) {
                Box(flex_direction: FlexDirection::Row) {
                    Text(content: "Keys: ", color: Color::DarkGrey)
                    Text(content: "c", color: theme.accent_color, weight: Weight::Bold)
                    Text(content: " copy content | ", color: Color::DarkGrey)
                    Text(content: "f", color: theme.accent_color, weight: Weight::Bold)
                    Text(content: " file path | ", color: Color::DarkGrey)
                    Text(content: "i", color: theme.accent_color, weight: Weight::Bold)
                    Text(content: " session ID | ", color: Color::DarkGrey)
                    Text(content: "p", color: theme.accent_color, weight: Weight::Bold)
                    Text(content: " project path", color: Color::DarkGrey)
                }
                Box(flex_direction: FlexDirection::Row, margin_top: 1) {
                    Text(content: "      ", color: Color::DarkGrey)
                    Text(content: "m", color: theme.accent_color, weight: Weight::Bold)
                    Text(content: " message text | ", color: Color::DarkGrey)
                    Text(content: "r", color: theme.accent_color, weight: Weight::Bold)
                    Text(content: " raw JSON | ", color: Color::DarkGrey)
                    Text(content: "u", color: theme.accent_color, weight: Weight::Bold)
                    Text(content: " URL | ", color: Color::DarkGrey)
                    Text(content: "s", color: theme.accent_color, weight: Weight::Bold)
                    Text(content: " view session", color: Color::DarkGrey)
                }
                Box(flex_direction: FlexDirection::Row, margin_top: 1) {
                    Text(content: "      ", color: Color::DarkGrey)
                    Text(content: "↑/↓ j/k", color: theme.accent_color, weight: Weight::Bold)
                    Text(content: " scroll | ", color: Color::DarkGrey)
                    Text(content: "PgUp/PgDn", color: theme.accent_color, weight: Weight::Bold)
                    Text(content: " page | ", color: Color::DarkGrey)
                    Text(content: "Home/End", color: theme.accent_color, weight: Weight::Bold)
                    Text(content: " top/bottom | ", color: Color::DarkGrey)
                    Text(content: "ESC", color: theme.accent_color, weight: Weight::Bold)
                    Text(content: " back", color: Color::DarkGrey)
                }
            }
            
            // Clipboard message
            #(if let Some(message) = clipboard.message {
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
                            content: message,
                            color: Color::White,
                            weight: Weight::Bold,
                        )
                    }
                }
            } else {
                element! { Box() }
            })
        }
    }
}