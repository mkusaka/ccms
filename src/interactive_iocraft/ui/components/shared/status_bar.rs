//! Status bar component

use iocraft::prelude::*;

#[derive(Default, Props)]
pub struct StatusBarProps {
    pub message: Option<String>,
}

#[component]
pub fn StatusBar(props: &StatusBarProps) -> impl Into<AnyElement<'static>> {
    element! {
        Box(
            flex_direction: FlexDirection::Row,
            height: 1,
            background_color: Color::DarkGrey,
            padding_left: 1,
            padding_right: 1,
        ) {
            Text(
                content: props.message.clone().unwrap_or_else(|| "Ready".to_string()),
                color: Color::White,
            )
        }
    }
}