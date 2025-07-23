use iocraft::prelude::*;
use crate::interactive_iocraft::ui::UIState;

#[derive(Default, Props)]
pub struct HelpViewProps {
    pub ui_state: UIState,
}

#[component]
pub fn HelpView<'a>(_props: &HelpViewProps) -> impl Into<AnyElement<'a>> {
    element! {
        View(
            flex_direction: FlexDirection::Column,
            border_style: BorderStyle::Double,
            border_color: Color::Cyan,
            padding: 2
        ) {
            Text(
                content: "Interactive Claude Search - Help",
                weight: Weight::Bold,
                color: Color::Cyan,
                align: TextAlign::Center
            )
            Text(content: "")
            
            Text(content: "KEYBOARD SHORTCUTS", weight: Weight::Bold, color: Color::Yellow)
            Text(content: "")
            
            Text(content: "Search Mode:", weight: Weight::Bold)
            View(margin_left: 2) {
                View(flex_direction: FlexDirection::Column) {
                    HelpItem(label: "Any character".to_string(), description: "Add to search query".to_string())
                    HelpItem(label: "Backspace".to_string(), description: "Remove last character".to_string())
                    HelpItem(label: "↑/↓".to_string(), description: "Navigate results".to_string())
                    HelpItem(label: "Enter".to_string(), description: "View result details".to_string())
                    HelpItem(label: "Home/End".to_string(), description: "Jump to first/last result".to_string())
                    HelpItem(label: "PageUp/PageDown".to_string(), description: "Scroll by page".to_string())
                    HelpItem(label: "Tab".to_string(), description: "Cycle role filter".to_string())
                    HelpItem(label: "Ctrl+R".to_string(), description: "Clear cache and reload".to_string())
                    HelpItem(label: "Ctrl+T".to_string(), description: "Toggle truncation mode".to_string())
                    HelpItem(label: "?".to_string(), description: "Show this help".to_string())
                    HelpItem(label: "Esc/Ctrl+C".to_string(), description: "Exit application".to_string())
                }
            }
            
            Text(content: "")
            Text(content: "Result Detail Mode:", weight: Weight::Bold)
            View(margin_left: 2) {
                View(flex_direction: FlexDirection::Column) {
                    HelpItem(label: "S".to_string(), description: "View full session".to_string())
                    HelpItem(label: "F".to_string(), description: "Copy file path".to_string())
                    HelpItem(label: "I".to_string(), description: "Copy session ID".to_string())
                    HelpItem(label: "P".to_string(), description: "Copy project path".to_string())
                    HelpItem(label: "M".to_string(), description: "Copy message text".to_string())
                    HelpItem(label: "R".to_string(), description: "Copy raw JSON".to_string())
                    HelpItem(label: "J/↓".to_string(), description: "Scroll down".to_string())
                    HelpItem(label: "K/↑".to_string(), description: "Scroll up".to_string())
                    HelpItem(label: "PageUp/PageDown".to_string(), description: "Scroll by 10 lines".to_string())
                    HelpItem(label: "Esc".to_string(), description: "Return to search".to_string())
                }
            }
            
            Text(content: "")
            Text(content: "Session Viewer Mode:", weight: Weight::Bold)
            View(margin_left: 2) {
                View(flex_direction: FlexDirection::Column) {
                    HelpItem(label: "Type characters".to_string(), description: "Filter messages".to_string())
                    HelpItem(label: "↑/↓".to_string(), description: "Navigate messages".to_string())
                    HelpItem(label: "Enter".to_string(), description: "View message details".to_string())
                    HelpItem(label: "I".to_string(), description: "Copy session ID".to_string())
                    HelpItem(label: "O".to_string(), description: "Change sort order".to_string())
                    HelpItem(label: "C".to_string(), description: "Copy selected message".to_string())
                    HelpItem(label: "Shift+C".to_string(), description: "Copy all messages".to_string())
                    HelpItem(label: "Esc/Backspace".to_string(), description: "Return to detail view".to_string())
                }
            }
            
            Text(content: "")
            Text(content: "SEARCH SYNTAX", weight: Weight::Bold, color: Color::Yellow)
            View(margin_left: 2) {
                View(flex_direction: FlexDirection::Column) {
                    HelpItem(label: "word".to_string(), description: "Search for 'word'".to_string())
                    HelpItem(label: "\"multi word\"".to_string(), description: "Search exact phrase".to_string())
                    HelpItem(label: "word1 AND word2".to_string(), description: "Both words must appear".to_string())
                    HelpItem(label: "word1 OR word2".to_string(), description: "Either word must appear".to_string())
                    HelpItem(label: "NOT word".to_string(), description: "Word must not appear".to_string())
                    HelpItem(label: "/regex/".to_string(), description: "Regular expression search".to_string())
                    HelpItem(label: "()".to_string(), description: "Group expressions".to_string())
                }
            }
            
            Text(content: "")
            Text(
                content: "Press any key to close this help",
                color: Color::Grey,
                align: TextAlign::Center
            )
        }
    }
}

#[derive(Default, Props)]
struct HelpItemProps {
    label: String,
    description: String,
}

#[component]
fn HelpItem<'a>(props: &HelpItemProps) -> impl Into<AnyElement<'a>> {
    element! {
        View(flex_direction: FlexDirection::Row) {
            Text(
                content: format!("{:<20}", props.label),
                color: Color::Green
            )
            Text(content: props.description.clone())
        }
    }
}