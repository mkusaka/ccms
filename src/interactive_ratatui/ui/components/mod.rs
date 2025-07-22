pub mod search_bar;
pub mod result_list;
pub mod result_detail;
pub mod session_viewer;
pub mod help_dialog;

use ratatui::{Frame, layout::Rect};
use crossterm::event::KeyEvent;
use crate::interactive_ratatui::ui::events::Message;

pub trait Component {
    fn render(&mut self, f: &mut Frame, area: Rect);
    fn handle_key(&mut self, key: KeyEvent) -> Option<Message>;
}