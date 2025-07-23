use iocraft::prelude::*;
use crate::interactive_iocraft::ui::{SessionState, UIState};

#[derive(Default, Props)]
pub struct SessionViewerViewProps {
    pub session_state: SessionState,
    pub ui_state: UIState,
}

#[component]
pub fn SessionViewerView<'a>(props: &SessionViewerViewProps) -> impl Into<AnyElement<'a>> {
    let session_id = props.session_state.session_id.clone().unwrap_or_else(|| "Unknown".to_string());
    let file_name = props.session_state.file_path.as_ref()
        .and_then(|path| std::path::Path::new(path).file_name())
        .and_then(|name| name.to_str())
        .unwrap_or("Unknown");
    
    let total_messages = props.session_state.messages.len();
    let filtered_count = props.session_state.filtered_indices.len();
    let has_filter = !props.session_state.query.is_empty();
    
    element! {
        View(flex_direction: FlexDirection::Column) {
            // Header
            View(
                border_style: BorderStyle::Single,
                border_color: Color::Grey,
                padding: 1
            ) {
                View(flex_direction: FlexDirection::Column) {
                    Text(
                        content: "Session Viewer",
                        weight: Weight::Bold
                    )
                    View(flex_direction: FlexDirection::Row) {
                        Text(content: "Session: ")
                        Text(content: session_id, color: Color::Cyan)
                    }
                    View(flex_direction: FlexDirection::Row) {
                        Text(content: "File: ")
                        Text(content: file_name, color: Color::Cyan)
                    }
                }
            }
            
            // Search bar
            #( if !props.session_state.query.is_empty() {
                Some(element! {
                    View(
                        border_style: BorderStyle::Single,
                        border_color: Color::Grey,
                        padding: 1,
                        margin_top: 1
                    ) {
                        View(flex_direction: FlexDirection::Row) {
                            Text(content: "Filter: ")
                            Text(content: props.session_state.query.clone(), color: Color::Yellow)
                        }
                    }
                })
            } else {
                None
            })
            
            // Messages header
            View(
                border_style: BorderStyle::Single,
                border_color: Color::Grey,
                padding: 1,
                margin_top: 1
            ) {
                Text(
                    content: if has_filter {
                        format!("Messages ({} total, {} filtered)", total_messages, filtered_count)
                    } else {
                        format!("Messages ({} total)", total_messages)
                    },
                    weight: Weight::Bold
                )
            }
            
            // Messages list
            View(
                border_style: BorderStyle::Single,
                border_color: Color::Grey,
                flex_grow: 1.0,
                padding: 1
            ) {
                View(flex_direction: FlexDirection::Column) {
                    #(props.session_state.filtered_indices.iter()
                        .enumerate()
                        .skip(props.session_state.scroll_offset)
                        .take(10)
                        .map(|(list_idx, &msg_idx)| {
                            let msg = props.session_state.messages.get(msg_idx).unwrap();
                            let is_selected = list_idx == props.session_state.selected_index;
                            
                            // Parse message to extract role and content
                            let (role, timestamp, content) = parse_message_preview(msg);
                            
                            element! {
                                View(flex_direction: FlexDirection::Row) {
                                Text(
                                    content: if is_selected { ">" } else { " " },
                                    color: if is_selected { Color::Cyan } else { Color::White },
                                    weight: if is_selected { Weight::Bold } else { Weight::Normal }
                                )
                                Text(content: format!("{:3}. ", msg_idx + 1))
                                Text(
                                    content: format!("[{:^10}] ", role),
                                    color: Color::Yellow
                                )
                                Text(
                                    content: format!("{} ", timestamp),
                                    color: Color::Grey
                                )
                                Text(
                                    content: content,
                                    color: if is_selected { Color::White } else { Color::Grey }
                                )
                                }
                            }
                        }))
                    
                    // Scroll indicator
                    #( if props.session_state.filtered_indices.len() > 10 {
                        let start = props.session_state.scroll_offset + 1;
                        let end = (props.session_state.scroll_offset + 10).min(props.session_state.filtered_indices.len());
                        Some(element! {
                            View(flex_direction: FlexDirection::Column) {
                                Text(content: "")
                                Text(
                                    content: format!("Showing {}-{} of {} messages ↑/↓ to scroll", start, end, props.session_state.filtered_indices.len()),
                                    color: Color::Grey
                                )
                            }
                        })
                    } else {
                        None
                    })
                }
            }
            
            // Footer
            Text(
                content: "Enter: View | ↑/↓: Navigate | /: Search | I: Copy Session ID | O: Sort | C: Copy All | Esc: Back",
                color: Color::Grey
            )
        }
    }
}

fn parse_message_preview(json_line: &str) -> (String, String, String) {
    // Try to parse the JSON to extract role, timestamp, and content
    if let Ok(msg) = serde_json::from_str::<serde_json::Value>(json_line) {
        let role = msg.get("role")
            .and_then(|r| r.as_str())
            .unwrap_or("unknown")
            .to_string();
        
        let timestamp = msg.get("timestamp")
            .and_then(|t| t.as_str())
            .and_then(|t| chrono::DateTime::parse_from_rfc3339(t).ok())
            .map(|t| t.format("%m/%d %H:%M").to_string())
            .unwrap_or_else(|| "??/?? ??:??".to_string());
        
        let content = extract_content(&msg)
            .chars()
            .take(50)
            .collect::<String>()
            .replace('\n', " ");
        
        let content = if content.len() > 50 {
            content + "..."
        } else {
            content
        };
        
        (role, timestamp, content)
    } else {
        ("error".to_string(), "??/?? ??:??".to_string(), "Failed to parse message".to_string())
    }
}

fn extract_content(msg: &serde_json::Value) -> String {
    // Try different content extraction patterns
    if let Some(content) = msg.get("content") {
        if let Some(text) = content.as_str() {
            return text.to_string();
        } else if let Some(array) = content.as_array() {
            for item in array {
                if let Some(text) = item.get("text").and_then(|t| t.as_str()) {
                    return text.to_string();
                }
            }
        }
    }
    
    if let Some(message) = msg.get("message") {
        if let Some(content) = message.get("content").and_then(|c| c.as_str()) {
            return content.to_string();
        }
    }
    
    "[No content]".to_string()
}