use iocraft::prelude::*;
use crate::interactive_iocraft::ui::{DetailState, UIState};

#[derive(Default, Props)]
pub struct ResultDetailViewProps {
    pub detail_state: DetailState,
    pub ui_state: UIState,
}

#[component]
pub fn ResultDetailView<'a>(props: &ResultDetailViewProps) -> impl Into<AnyElement<'a>> {
    let result = match &props.detail_state.selected_result {
        Some(r) => r,
        None => {
            return element! {
                View(flex_direction: FlexDirection::Column) {
                    Text(content: "No result selected")
                }
            };
        }
    };
    
    // Format timestamp
    let timestamp_str = if let Ok(ts) = chrono::DateTime::parse_from_rfc3339(&result.timestamp) {
        ts.format("%Y-%m-%d %H:%M:%S").to_string()
    } else {
        "Unknown".to_string()
    };
    
    // Extract project path
    let project_path = extract_project_path(&result.file)
        .unwrap_or_else(|| result.project_path.clone());
    
    let file_name = std::path::Path::new(&result.file)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("Unknown");
    
    // Split content into lines for scrolling
    let content_lines: Vec<String> = result.text
        .lines()
        .map(|line| line.to_string())
        .collect();
    
    element! {
        View(flex_direction: FlexDirection::Column) {
            // Header separator
            Text(
                content: "─".repeat(80),
                color: Color::Grey
            )
            
            // Metadata
            View(flex_direction: FlexDirection::Row) {
                Text(content: "Role: ", weight: Weight::Bold)
                Text(content: result.role.clone())
            }
            View(flex_direction: FlexDirection::Row) {
                Text(content: "Time: ", weight: Weight::Bold)
                Text(content: timestamp_str)
            }
            View(flex_direction: FlexDirection::Row) {
                Text(content: "File: ", weight: Weight::Bold)
                Text(content: file_name)
            }
            View(flex_direction: FlexDirection::Row) {
                Text(content: "Project: ", weight: Weight::Bold)
                Text(content: project_path)
            }
            View(flex_direction: FlexDirection::Row) {
                Text(content: "UUID: ", weight: Weight::Bold)
                Text(content: result.uuid.clone())
            }
            View(flex_direction: FlexDirection::Row) {
                Text(content: "Session: ", weight: Weight::Bold)
                Text(content: result.session_id.clone())
            }
            
            // Content separator
            Text(
                content: "─".repeat(80),
                color: Color::Grey
            )
            
            // Content with scrolling
            View(flex_direction: FlexDirection::Column) {
                #(content_lines.iter()
                    .skip(props.detail_state.scroll_offset)
                    .take(20)
                    .map(|line| {
                        element! {
                            Text(content: line.clone())
                        }
                    }))
            }
            
            // Footer separator
            Text(
                content: "─".repeat(80),
                color: Color::Grey
            )
            
            // Actions
            Text(content: "\nActions:", weight: Weight::Bold)
            Text(content: "  [S] - View full session")
            Text(content: "  [F] - Copy file path")
            Text(content: "  [I] - Copy session ID")
            Text(content: "  [P] - Copy project path")
            Text(content: "  [M] - Copy message text")
            Text(content: "  [R] - Copy raw JSON")
            Text(content: "  [J/↓] - Scroll down")
            Text(content: "  [K/↑] - Scroll up")
            Text(content: "  [PageDown] - Scroll down 10 lines")
            Text(content: "  [PageUp] - Scroll up 10 lines")
            Text(content: "  [Esc] - Return to search results")
            
            // Message
            #( if let Some(ref msg) = props.ui_state.message {
                Some(element! {
                    View(flex_direction: FlexDirection::Column) {
                        Text(content: "")
                        Text(
                            content: msg.clone(),
                            color: if msg.starts_with('✓') { Color::Green } else { Color::Yellow }
                        )
                    }
                })
            } else {
                None
            })
        }
    }
}

fn extract_project_path(file_path: &str) -> Option<String> {
    // Extract project path from ~/.claude/projects/{encoded-path}/{session}.jsonl
    if let Some(start) = file_path.find("/.claude/projects/") {
        let after_projects = &file_path[start + "/.claude/projects/".len()..];
        if let Some(slash_pos) = after_projects.find('/') {
            let encoded = &after_projects[..slash_pos];
            return Some(encoded.replace('-', "/"));
        }
    }
    None
}