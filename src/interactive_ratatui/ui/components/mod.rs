pub mod help_dialog;
pub mod result_detail;
pub mod result_list;
pub mod search_bar;
pub mod session_viewer;

use crate::interactive_ratatui::ui::events::Message;
use crossterm::event::KeyEvent;
use ratatui::{Frame, layout::Rect};

pub trait Component {
    fn render(&mut self, f: &mut Frame, area: Rect);
    fn handle_key(&mut self, key: KeyEvent) -> Option<Message>;
}
