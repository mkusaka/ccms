use crate::interactive_ratatui::ui::components::{
    Component, list_viewer::ListViewer, view_layout::ViewLayout,
};
use crate::interactive_ratatui::ui::events::Message;
use crate::query::condition::SearchResult;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{Frame, layout::Rect};

#[derive(Default)]
pub struct ResultList {
    list_viewer: ListViewer<SearchResult>,
}

impl ResultList {
    pub fn new() -> Self {
        Self {
            list_viewer: ListViewer::new("Results".to_string(), "No results found".to_string()),
        }
    }

    pub fn set_results(&mut self, results: Vec<SearchResult>) {
        self.list_viewer.set_items(results);
    }

    pub fn set_selected_index(&mut self, index: usize) {
        self.list_viewer.set_selected_index(index);
    }

    pub fn selected_result(&self) -> Option<&SearchResult> {
        self.list_viewer.get_selected_item()
    }

    pub fn update_results(&mut self, results: Vec<SearchResult>, selected_index: usize) {
        self.list_viewer.set_items(results);
        self.list_viewer.set_selected_index(selected_index);
    }

    pub fn set_truncation_enabled(&mut self, enabled: bool) {
        self.list_viewer.set_truncation_enabled(enabled);
    }

    pub fn update_selection(&mut self, index: usize) {
        self.list_viewer.set_selected_index(index);
    }
}

impl Component for ResultList {
    fn render(&mut self, f: &mut Frame, area: Rect) {
        let truncation_status = if self.list_viewer.is_truncation_enabled() {
            "Truncated"
        } else {
            "Full Text"
        };
        let layout = ViewLayout::new("Search Results".to_string())
            .with_subtitle(format!(
                "{} results found | Ctrl+T: Toggle truncation [{}]",
                self.list_viewer.filtered_count(),
                truncation_status
            ))
            .with_status_text(
                "Tab: Filter | ↑/↓ or Ctrl+P/N or Ctrl+U/D: Navigate | Enter: Detail | s: Session | Alt+←/→: History | ?: Help | Esc: Exit"
                    .to_string(),
            );

        layout.render(f, area, |f, content_area| {
            self.list_viewer.render(f, content_area);
        });
    }

    fn handle_key(&mut self, key: KeyEvent) -> Option<Message> {
        match key.code {
            KeyCode::Up => {
                if self.list_viewer.move_up() {
                    Some(Message::SelectResult(self.list_viewer.selected_index()))
                } else {
                    None
                }
            }
            KeyCode::Down => {
                if self.list_viewer.move_down() {
                    Some(Message::SelectResult(self.list_viewer.selected_index()))
                } else {
                    None
                }
            }
            KeyCode::Char('p') if key.modifiers == KeyModifiers::CONTROL => {
                if self.list_viewer.move_up() {
                    Some(Message::SelectResult(self.list_viewer.selected_index()))
                } else {
                    None
                }
            }
            KeyCode::Char('n') if key.modifiers == KeyModifiers::CONTROL => {
                if self.list_viewer.move_down() {
                    Some(Message::SelectResult(self.list_viewer.selected_index()))
                } else {
                    None
                }
            }
            KeyCode::PageUp => {
                if self.list_viewer.page_up() {
                    Some(Message::SelectResult(self.list_viewer.selected_index()))
                } else {
                    None
                }
            }
            KeyCode::PageDown => {
                if self.list_viewer.page_down() {
                    Some(Message::SelectResult(self.list_viewer.selected_index()))
                } else {
                    None
                }
            }
            KeyCode::Home => {
                if self.list_viewer.move_to_start() {
                    Some(Message::SelectResult(self.list_viewer.selected_index()))
                } else {
                    None
                }
            }
            KeyCode::End => {
                if self.list_viewer.move_to_end() {
                    Some(Message::SelectResult(self.list_viewer.selected_index()))
                } else {
                    None
                }
            }
            KeyCode::Char('u') if key.modifiers == KeyModifiers::CONTROL => {
                if self.list_viewer.half_page_up() {
                    Some(Message::SelectResult(self.list_viewer.selected_index()))
                } else {
                    None
                }
            }
            KeyCode::Char('d') if key.modifiers == KeyModifiers::CONTROL => {
                if self.list_viewer.half_page_down() {
                    Some(Message::SelectResult(self.list_viewer.selected_index()))
                } else {
                    None
                }
            }
            KeyCode::Enter => Some(Message::EnterResultDetail),
            KeyCode::Char('s') if key.modifiers == KeyModifiers::CONTROL => {
                Some(Message::EnterSessionViewer) // Ctrl+S
            }
            _ => None,
        }
    }
}
