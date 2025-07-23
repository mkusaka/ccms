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
                Text(
                    content: props.search_state.query.clone(),
                    color: Color::White
                )
                Text(
                    content: " ",
                    color: Color::White
                )
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
                #(props.search_state.results.iter().enumerate().skip(props.search_state.scroll_offset).take(10).map(|(idx, result)| {
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
                }))
            }

            // More results indicator
            #( if props.search_state.results.len() > props.search_state.scroll_offset + 10 {
                Some(element! {
                    Text(
                        content: format!("... and {} more results", props.search_state.results.len() - props.search_state.scroll_offset - 10),
                        color: Color::Grey
                    )
                })
            } else {
                None
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
