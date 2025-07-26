pub mod help_dialog;
pub mod list_item;
pub mod list_viewer;
pub mod result_detail;
pub mod result_list;
pub mod search_bar;
pub mod session_viewer;
pub mod text_input;
pub mod view_layout;

// #[cfg(test)]
// mod list_item_test;  // Temporarily disabled - functions were removed
#[cfg(test)]
mod actual_app_rendering_test;
#[cfg(test)]
mod actual_rendering_test;
#[cfg(test)]
mod border_fix_verification_test;
#[cfg(test)]
mod double_border_test;
#[cfg(test)]
mod exact_border_pattern_test;
#[cfg(test)]
mod exact_issue_test;
#[cfg(test)]
mod final_fix_test;
#[cfg(test)]
mod fixed_column_test;
#[cfg(test)]
mod full_layout_test;
#[cfg(test)]
mod list_viewer_test;
#[cfg(test)]
mod list_rendering_issue_test;
#[cfg(test)]
mod list_widget_test;
#[cfg(test)]
mod result_detail_test;
#[cfg(test)]
mod ratatui_table_test;
#[cfg(test)]
mod result_list_test;
#[cfg(test)]
mod search_bar_test;
#[cfg(test)]
mod search_result_content_test;
#[cfg(test)]
mod search_results_border_test;
#[cfg(test)]
mod session_viewer_test;
#[cfg(test)]
mod specific_pattern_test;
#[cfg(test)]
mod table_column_test;
#[cfg(test)]
mod table_debug_detailed_test;
#[cfg(test)]
mod table_debug_test;
#[cfg(test)]
mod table_extra_column_test;
#[cfg(test)]
mod table_padding_test;
#[cfg(test)]
mod table_rendering_test;
#[cfg(test)]
mod table_row_debug_test;
#[cfg(test)]
mod text_input_test;
#[cfg(test)]
mod unicode_width_test;
#[cfg(test)]
mod user_issue_reproduction_test;
#[cfg(test)]
mod view_layout_test;

use crate::interactive_ratatui::ui::events::Message;
use crossterm::event::KeyEvent;
use ratatui::{Frame, layout::Rect};

pub trait Component {
    fn render(&mut self, f: &mut Frame, area: Rect);
    fn handle_key(&mut self, key: KeyEvent) -> Option<Message>;
}
