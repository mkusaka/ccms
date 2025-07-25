//! Search bar component with role filter display

use crate::interactive_iocraft::ui::contexts::Theme;
use crate::interactive_iocraft::ui::components::shared::text_input::AdvancedTextInput;
use iocraft::prelude::*;

#[non_exhaustive]
#[derive(Default, Props)]
pub struct SearchBarProps<'a> {
    pub value: String,
    pub on_change: Handler<'static, String>,
    pub role_filter: Option<String>,
    pub on_role_filter_toggle: Option<Handler<'a, ()>>,
    pub status: Option<String>,
    pub message: Option<String>,
    pub focused: bool,
}

#[component]
pub fn SearchBar<'a>(hooks: Hooks, props: &mut SearchBarProps<'a>) -> impl Into<AnyElement<'a>> {
    let theme = hooks.use_context::<Theme>();
    
    element! {
        Box(
            flex_direction: FlexDirection::Column,
            width: 100pct,
        ) {
            // Main search bar
            Box(
                flex_direction: FlexDirection::Row,
                border_style: if props.focused { theme.focused_border } else { theme.default_border },
                border_color: if props.focused { theme.accent_color } else { theme.primary_color },
                padding: 1,
                margin_bottom: 1,
            ) {
                // Search input
                Box(flex_grow: 1.0) {
                    AdvancedTextInput(
                        value: props.value.clone(),
                        has_focus: props.focused,
                        on_change: props.on_change.take(),
                    )
                }
                
                // Role filter indicator
                #(if let Some(ref filter) = props.role_filter {
                    element! {
                        Box(margin_left: 1) {
                            Text(
                                content: format!(" [{}]", filter),
                                color: theme.accent_color,
                            )
                        }
                    }
                } else {
                    element! { Box() }
                })
                
                // Status indicator
                #(if let Some(ref status) = props.status {
                    element! {
                        Box(margin_left: 1) {
                            Text(
                                content: format!(" ({})", status),
                                color: theme.status_color(status),
                            )
                        }
                    }
                } else {
                    element! { Box() }
                })
            }
            
            // Message display
            #(if let Some(ref message) = props.message {
                element! {
                    Box(margin_bottom: 1) {
                        Text(
                            content: message.to_string(),
                            color: theme.info_color,
                        )
                    }
                }
            } else {
                element! { Box() }
            })
            
            // Help text
            Box(
                flex_direction: FlexDirection::Row,
                margin_bottom: 1,
            ) {
                Text(
                    content: "Press ".to_string(),
                    color: Color::DarkGrey,
                )
                Text(
                    content: "Tab".to_string(),
                    color: theme.accent_color,
                    weight: Weight::Bold,
                )
                Text(
                    content: " to toggle role filter, ".to_string(),
                    color: Color::DarkGrey,
                )
                Text(
                    content: "Enter".to_string(),
                    color: theme.accent_color,
                    weight: Weight::Bold,
                )
                Text(
                    content: " to select, ".to_string(),
                    color: Color::DarkGrey,
                )
                Text(
                    content: "?".to_string(),
                    color: theme.accent_color,
                    weight: Weight::Bold,
                )
                Text(
                    content: " for help".to_string(),
                    color: Color::DarkGrey,
                )
            }
        }
    }
}

#[cfg(test)]
mod search_bar_test;