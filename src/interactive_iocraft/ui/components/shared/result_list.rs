//! Result list component for displaying search results

use crate::interactive_iocraft::ui::contexts::{Theme, Settings};
use crate::interactive_iocraft::ui::hooks::use_virtual_list;
use crate::interactive_iocraft::SearchResult;
use iocraft::prelude::*;
use chrono::{Local, TimeZone};

#[non_exhaustive]
#[derive(Default, Props)]
pub struct ResultListProps<'a> {
    pub results: Vec<SearchResult>,
    pub selected: usize,
    pub scroll_offset: usize,
    pub on_select: Handler<'a, usize>,
    pub truncate: bool,
    pub max_width: usize,
}

#[component]
pub fn ResultList<'a>(mut hooks: Hooks, props: &'a ResultListProps<'a>) -> impl Into<AnyElement<'a>> {
    let theme = hooks.use_context::<Theme>();
    let settings = hooks.use_context::<Settings>();
    let terminal_size = hooks.use_terminal_size();
    
    // Calculate visible area
    let height = terminal_size.1.saturating_sub(10) as usize; // Leave room for other UI elements
    
    // Always use virtual list hook (but we can choose whether to use its results)
    let (_, virtual_visible_results) = use_virtual_list(
        &mut hooks,
        &props.results,
        1, // item_height = 1 row
        height,
    );
    
    // Decide whether to use virtual scrolling
    let use_virtual = settings.performance.enable_virtual_scroll && props.results.len() > height * 2;
    
    let visible_results: Vec<(usize, SearchResult)> = if use_virtual {
        virtual_visible_results
    } else {
        // Traditional slicing for smaller lists
        props.results[props.scroll_offset..props.results.len().min(props.scroll_offset + height)]
            .iter()
            .enumerate()
            .map(|(idx, r)| (props.scroll_offset + idx, r.clone()))
            .collect()
    };
    
    element! {
        Box(
            flex_grow: 1.0,
            border_style: theme.default_border,
            border_color: theme.primary_color,
        ) {
            // Results header
            Box(
                flex_direction: FlexDirection::Row,
                padding: 1,
                background_color: Color::DarkGrey,
            ) {
                Text(
                    content: format!("Results: {} items", props.results.len()),
                    color: Color::White,
                    weight: Weight::Bold,
                )
                Box(margin_left: 1) {
                    Text(
                        content: " | Press 't' to toggle truncation".to_string(),
                        color: Color::Grey,
                    )
                }
            }
            
            // Results list
            #(if props.results.is_empty() {
                element! {
                    Box(
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        flex_grow: 1.0,
                    ) {
                        Text(
                            content: "No results found".to_string(),
                            color: Color::DarkGrey,
                        )
                    }
                }
            } else {
                element! {
                    Box(flex_direction: FlexDirection::Column) {
                        #(visible_results.into_iter().map(|(absolute_idx, result)| {
                            let is_selected = absolute_idx == props.selected;
                            
                            element! {
                                ResultItem(
                                    result: result,
                                    index: absolute_idx,
                                    selected: is_selected,
                                    truncate: props.truncate,
                                    max_width: props.max_width,
                                )
                            }
                        }))
                    }
                }
            })
            
            // Scroll indicator
            #(if props.results.len() > height {
                let scroll_percentage = (props.scroll_offset as f32 / (props.results.len() - height) as f32 * 100.0) as u32;
                element! {
                    Box(
                        position: Position::Absolute,
                        top: 0,
                        right: 0,
                        width: 1,
                        height: 100pct,
                    ) {
                        Text(
                            content: format!("{}%", scroll_percentage),
                            color: Color::DarkGrey,
                        )
                    }
                }
            } else {
                element! { Box() }
            })
        }
    }
}

#[derive(Props)]
struct ResultItemProps {
    result: SearchResult,
    index: usize,
    selected: bool,
    truncate: bool,
    max_width: usize,
}

impl Default for ResultItemProps {
    fn default() -> Self {
        panic!("ResultItemProps cannot be default constructed")
    }
}

#[component]
fn ResultItem(hooks: Hooks, props: &ResultItemProps) -> impl Into<AnyElement<'static>> {
    let theme = hooks.use_context::<Theme>();
    let role_color = theme.role_color(&props.result.role);
    
    // Format timestamp
    let timestamp = props.result.timestamp
        .parse::<i64>()
        .ok()
        .and_then(|ts| Local.timestamp_opt(ts, 0).single())
        .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
        .unwrap_or_else(|| props.result.timestamp.clone());
    
    // Truncate content if needed
    let content = if props.truncate && props.result.text.len() > props.max_width {
        let truncated: String = props.result.text
            .chars()
            .take(props.max_width.saturating_sub(3))
            .collect();
        format!("{}...", truncated)
    } else {
        props.result.text.clone()
    };
    
    element! {
        Box(
            flex_direction: FlexDirection::Row,
            padding: 1,
            background_color: if props.selected { Color::DarkGrey } else { Color::Reset },
        ) {
            // Index
            Text(
                content: format!("{:>3} ", props.index + 1),
                color: if props.selected { Color::Yellow } else { Color::DarkGrey },
                weight: if props.selected { Weight::Bold } else { Weight::Normal },
            )
            
            // Role
            Text(
                content: format!("[{:>9}] ", props.result.role),
                color: role_color,
                weight: Weight::Bold,
            )
            
            // Timestamp
            Text(
                content: format!("{} ", timestamp),
                color: Color::DarkGrey,
            )
            
            // Content
            Box(flex_grow: 1.0) {
                Text(
                    content: content,
                    color: if props.selected { Color::White } else { Color::Reset },
                )
            }
        }
    }
}

#[cfg(test)]
mod result_list_test;