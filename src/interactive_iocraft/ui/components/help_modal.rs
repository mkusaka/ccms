//! Help modal component

use crate::interactive_iocraft::ui::contexts::Theme;
use iocraft::prelude::*;

#[derive(Default, Props)]
pub struct HelpModalProps<'a> {
    pub on_close: Handler<'a, ()>,
}

#[component]
pub fn HelpModal<'a>(mut hooks: Hooks, props: &HelpModalProps<'a>) -> impl Into<AnyElement<'a>> {
    let theme = hooks.use_context::<Theme>();
    
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
            Box(
                width: 80pct,
                height: 80pct,
                background_color: Color::Reset,
                border_style: theme.default_border,
                border_color: theme.primary_color,
                padding: 2,
            ) {
                Box(flex_direction: FlexDirection::Column) {
                    // Title
                    Box(margin_bottom: 2, flex_direction: FlexDirection::Column) {
                        Text(
                            content: "Claude Session Search - Help".to_string(),
                            color: theme.primary_color,
                            weight: Weight::Bold,
                            align: TextAlign::Center,
                        )
                        Text(
                            content: "(iocraft UI - React-like TUI framework)".to_string(),
                            color: Color::DarkGrey,
                            align: TextAlign::Center,
                        )
                    }
                    
                    // Features section
                    Box(margin_bottom: 1) {
                        Text(
                            content: "FEATURES:".to_string(),
                            color: theme.accent_color,
                            weight: Weight::Bold,
                        )
                    }
                    
                    Box(margin_left: 2, margin_bottom: 1) {
                        Box(flex_direction: FlexDirection::Column) {
                            Text(content: "• Real-time search with debouncing (300ms delay)", color: Color::DarkGrey)
                            Text(content: "• Virtual scrolling for large result sets", color: Color::DarkGrey)
                            Text(content: "• Multi-byte character support (Japanese/Emoji)", color: Color::DarkGrey)
                            Text(content: "• Non-blocking async operations", color: Color::DarkGrey)
                            Text(content: "• Secure clipboard integration", color: Color::DarkGrey)
                        }
                    }
                    
                    // Search mode shortcuts
                    Box(margin_bottom: 1) {
                        Text(
                            content: "SEARCH MODE:".to_string(),
                            color: theme.accent_color,
                            weight: Weight::Bold,
                        )
                    }
                    
                    Box(margin_left: 2, margin_bottom: 1) {
                        Box(flex_direction: FlexDirection::Column) {
                            HelpItem(key: "↑/↓, j/k", description: "Navigate results")
                            HelpItem(key: "PgUp/PgDn", description: "Page up/down")
                            HelpItem(key: "Home/End", description: "Jump to first/last")
                            HelpItem(key: "Enter", description: "View result details")
                            HelpItem(key: "Tab", description: "Toggle role filter (user/assistant/system)")
                            HelpItem(key: "t", description: "Toggle text truncation")
                            HelpItem(key: "/", description: "Focus search bar")
                            HelpItem(key: "?", description: "Show this help")
                            HelpItem(key: "Ctrl+C", description: "Quit (press twice to confirm)")
                        }
                    }
                    
                    // Detail mode shortcuts
                    Box(margin_bottom: 1) {
                        Text(
                            content: "DETAIL MODE:".to_string(),
                            color: theme.accent_color,
                            weight: Weight::Bold,
                        )
                    }
                    
                    Box(margin_left: 2, margin_bottom: 1) {
                        Box(flex_direction: FlexDirection::Column) {
                            HelpItem(key: "↑/↓, j/k", description: "Scroll content")
                            HelpItem(key: "c", description: "Copy message content")
                            HelpItem(key: "f", description: "Copy file path")
                            HelpItem(key: "i", description: "Copy session ID")
                            HelpItem(key: "p", description: "Copy project path")
                            HelpItem(key: "r", description: "Copy raw JSON")
                            HelpItem(key: "u", description: "Copy Claude URL")
                            HelpItem(key: "s", description: "View full session")
                            HelpItem(key: "ESC", description: "Back to search")
                        }
                    }
                    
                    // Session viewer shortcuts
                    Box(margin_bottom: 1) {
                        Text(
                            content: "SESSION VIEWER:".to_string(),
                            color: theme.accent_color,
                            weight: Weight::Bold,
                        )
                    }
                    
                    Box(margin_left: 2, margin_bottom: 1) {
                        Box(flex_direction: FlexDirection::Column) {
                            HelpItem(key: "↑/↓, j/k", description: "Navigate messages")
                            HelpItem(key: "/", description: "Search in session")
                            HelpItem(key: "o", description: "Change sort order")
                            HelpItem(key: "c", description: "Copy selected message")
                            HelpItem(key: "C", description: "Copy all messages")
                            HelpItem(key: "t", description: "Toggle truncation")
                            HelpItem(key: "ESC", description: "Back to detail view")
                        }
                    }
                    
                    // Footer
                    Box(margin_top: 2) {
                        Text(
                            content: "Press ESC to close".to_string(),
                            color: Color::DarkGrey,
                            align: TextAlign::Center,
                        )
                    }
                }
            }
        }
    }
}

#[derive(Default, Props)]
struct HelpItemProps {
    key: &'static str,
    description: &'static str,
}

#[component]
fn HelpItem(mut hooks: Hooks, props: &HelpItemProps) -> impl Into<AnyElement<'static>> {
    let theme = hooks.use_context::<Theme>();
    
    element! {
        Box(flex_direction: FlexDirection::Row, margin_bottom: 1) {
            Text(
                content: format!("{:>12} ", props.key),
                color: theme.success_color,
                weight: Weight::Bold,
            )
            Text(
                content: props.description.to_string(),
                color: Color::Reset,
            )
        }
    }
}