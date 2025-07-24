pub mod help_dialog;
pub mod list_item;
pub mod list_viewer;
pub mod result_detail;
pub mod result_list;
pub mod search_bar;
pub mod session_viewer;
pub mod view_layout;

#[cfg(test)]
mod list_item_test;
#[cfg(test)]
mod list_viewer_test;
#[cfg(test)]
mod result_detail_test;
#[cfg(test)]
mod result_list_test;
#[cfg(test)]
mod search_bar_test;
#[cfg(test)]
mod session_viewer_test;
#[cfg(test)]
mod view_layout_test;

use crate::interactive_ratatui::ui::events::Message;
use crossterm::event::KeyEvent;
use ratatui::{Frame, layout::Rect};

pub trait Component {
    fn render(&mut self, f: &mut Frame, area: Rect);
    fn handle_key(&mut self, key: KeyEvent) -> Option<Message>;
}
