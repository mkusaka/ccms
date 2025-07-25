//! Generic modal component

use iocraft::prelude::*;

#[derive(Default, Props)]
pub struct ModalProps<'a> {
    pub children: &'a AnyElement<'a>,
    pub on_close: Option<Handler<'a, ()>>,
}

#[component]
pub fn Modal<'a>(props: &ModalProps<'a>) -> impl Into<AnyElement<'a>> {
    element! {
        Box(
            position: Position::Absolute,
            top: 0,
            left: 0,
            width: 100pct,
            height: 100pct,
            background_color: Color::Black,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
        ) {
            #(props.children)
        }
    }
}