pub mod help_dialog;
pub mod list_item;
pub mod list_viewer;
pub mod message_detail;
pub mod message_preview;
pub mod result_list;
pub mod search_bar;
pub mod session_list;
pub mod session_preview;
pub mod session_viewer;
pub mod tab_bar;
pub mod text_input;
pub mod view_layout;

#[cfg(test)]
mod list_item_test;
#[cfg(test)]
mod list_viewer_test;
#[cfg(test)]
mod message_detail_test;
#[cfg(test)]
mod message_preview_test;
#[cfg(test)]
mod result_list_test;
#[cfg(test)]
mod search_bar_test;
#[cfg(test)]
mod session_list_test;
#[cfg(test)]
mod session_preview_test;
#[cfg(test)]
mod text_input_test;
#[cfg(test)]
mod view_layout_test;

use crate::interactive_ratatui::ui::events::Message;
use crossterm::event::KeyEvent;
use ratatui::{Frame, layout::Rect};

pub trait Component {
    fn render(&mut self, f: &mut Frame, area: Rect);
    fn handle_key(&mut self, key: KeyEvent) -> Option<Message>;
}

/// Check if a message is the exit prompt
pub fn is_exit_prompt(message: &Option<String>) -> bool {
    message
        .as_ref()
        .map(|msg| msg == "Press Ctrl+C again to exit")
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_exit_prompt() {
        // Test with exit prompt message
        let exit_message = Some("Press Ctrl+C again to exit".to_string());
        assert!(is_exit_prompt(&exit_message));

        // Test with other message
        let other_message = Some("Some other message".to_string());
        assert!(!is_exit_prompt(&other_message));

        // Test with None
        assert!(!is_exit_prompt(&None));

        // Test with empty string
        let empty_message = Some("".to_string());
        assert!(!is_exit_prompt(&empty_message));
    }
}
