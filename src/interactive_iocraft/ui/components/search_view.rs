use crate::interactive_iocraft::ui::{SearchState, UIState};
use iocraft::prelude::*;

#[derive(Default, Props)]
pub struct SearchViewProps {
    pub search_state: SearchState,
    pub ui_state: UIState,
}

#[component]
pub fn SearchView<'a>(props: &SearchViewProps) -> impl Into<AnyElement<'a>> {
    let role_prefix = if let Some(ref role) = props.search_state.role_filter {
        format!("[{role}] ")
    } else {
        String::new()
    };

    let search_status = if props.search_state.is_searching {
        "searching..."
    } else if props.search_state.query.is_empty() {
        ""
    } else {
        "typing..."
    };

    element! {
        View(flex_direction: FlexDirection::Column) {
            // Header
            Text(
                content: "Interactive Claude Search",
                weight: Weight::Bold,
                color: Color::Cyan
            )
            Text(
                content: "Type to search, ↑/↓ to navigate, Enter to select, Tab for role filter, Ctrl+R to reload, Esc/Ctrl+C to exit",
                color: Color::Grey
            )
            Text(content: "")

            // Search bar
            View(flex_direction: FlexDirection::Row) {
                Text(content: format!("Search{}: ", role_prefix))

                // Display query with cursor
                #( {
                    let chars: Vec<char> = props.search_state.query.chars().collect();
                    let cursor_pos = props.search_state.cursor_position;
                    let mut display_string = String::new();

                    // Build display string with cursor
                    for (i, ch) in chars.iter().enumerate() {
                        if i == cursor_pos {
                            display_string.push('|'); // Cursor marker
                        }
                        display_string.push(*ch);
                    }

                    // If cursor is at the end
                    if cursor_pos >= chars.len() {
                        display_string.push('|');
                    }

                    vec![element! {
                        Text(content: display_string, color: Color::White)
                    }]
                })

                #( if !search_status.is_empty() {
                    Some(element! {
                        Text(
                            content: format!(" {}", search_status),
                            color: Color::Grey
                        )
                    })
                } else {
                    None
                })
            }

            // Result count
            #( if !props.search_state.results.is_empty() {
                Some(element! {
                    Text(
                        content: format!("Found {} results", props.search_state.results.len()),
                        color: Color::Green
                    )
                })
            } else {
                None
            })

            // Message
            #( props.ui_state.message.as_ref().map(|msg| element! {
                Text(
                    content: msg.clone(),
                    color: if msg.starts_with('✓') { Color::Green } else { Color::Yellow }
                )
            }))

            Text(content: "")

            // Results list
            View(flex_direction: FlexDirection::Column) {
                #({
                    let (start, end) = props.search_state.calculate_visible_range(props.ui_state.terminal_height);
                    let start_copy = start;
                    props.search_state.results[start..end].iter().enumerate().map(move |(i, result)| {
                        let idx = start_copy + i;
                    let is_selected = idx == props.search_state.selected_index;
                    let role_color = Color::Yellow;
                    let text_color = if is_selected { Color::White } else { Color::Grey };

                    // Format timestamp
                    let timestamp_str = if let Ok(ts) = chrono::DateTime::parse_from_rfc3339(&result.timestamp) {
                        ts.format("%m/%d %H:%M").to_string()
                    } else {
                        "??/?? ??:??".to_string()
                    };

                    // Preview text - respect truncation mode
                    let preview = if props.ui_state.truncation_enabled {
                        // Truncated mode - show limited preview
                        let char_count = result.text.chars().count();
                        let text = result.text.chars().take(80).collect::<String>()
                            .replace('\n', " ");
                        if char_count > 80 {
                            format!("{text}...")
                        } else {
                            text
                        }
                    } else {
                        // Full text mode - show more content
                        let char_count = result.text.chars().count();
                        let text = result.text.chars().take(200).collect::<String>()
                            .replace('\n', " ");
                        if char_count > 200 {
                            format!("{text}...")
                        } else {
                            text
                        }
                    };

                    element! {
                        View(flex_direction: FlexDirection::Row) {
                        Text(
                            content: if is_selected { "> " } else { "  " },
                            color: if is_selected { Color::Cyan } else { Color::White },
                            weight: if is_selected { Weight::Bold } else { Weight::Normal }
                        )
                        Text(
                            content: format!("{}. ", idx + 1),
                            color: text_color
                        )
                        Text(
                            content: format!("[{}]", result.role.to_uppercase()),
                            color: role_color
                        )
                        Text(
                            content: format!("    {} ", timestamp_str),
                            color: text_color
                        )
                        Text(
                            content: preview,
                            color: text_color
                        )
                        }
                    }
                    })
                })
            }

            // More results indicator
            #( {
                let (_, end) = props.search_state.calculate_visible_range(props.ui_state.terminal_height);
                if props.search_state.results.len() > end {
                    Some(element! {
                        Text(
                            content: format!("... and {} more results", props.search_state.results.len() - end),
                            color: Color::Grey
                        )
                    })
                } else {
                    None
                }
            })

            // Truncation mode indicator
            Text(content: "")
            #( if props.ui_state.truncation_enabled {
                Some(element! {
                    Text(
                        content: "[Truncated]",
                        color: Color::Grey
                    )
                })
            } else {
                Some(element! {
                    Text(
                        content: "[Full Text]",
                        color: Color::Grey
                    )
                })
            })
        }
    }
}
