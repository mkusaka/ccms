use crate::interactive_ratatui::constants::*;
use crate::interactive_ratatui::ui::components::{
    Component, list_viewer::ListViewer, view_layout::Styles,
};
use crate::interactive_ratatui::ui::events::Message;
use crate::query::condition::SearchResult;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
};

#[derive(Default)]
pub struct ResultList {
    list_viewer: ListViewer<SearchResult>,
    preview_enabled: bool,
}

impl ResultList {
    pub fn new() -> Self {
        Self {
            list_viewer: ListViewer::new("Results".to_string(), "No results found".to_string()),
            preview_enabled: false,
        }
    }

    pub fn set_results(&mut self, results: Vec<SearchResult>) {
        self.list_viewer.set_items(results);
    }

    pub fn set_preview_enabled(&mut self, enabled: bool) {
        self.preview_enabled = enabled;
    }

    pub fn set_selected_index(&mut self, index: usize) {
        // Use set_filtered_position since we're dealing with filtered indices
        self.list_viewer.set_filtered_position(index);
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
        // Use set_filtered_position since we're dealing with filtered indices
        self.list_viewer.set_filtered_position(index);
    }

    pub fn get_selected_index(&self) -> usize {
        self.list_viewer.selected_index
    }

    pub fn get_scroll_offset(&self) -> usize {
        self.list_viewer.scroll_offset
    }
}

impl Component for ResultList {
    fn render(&mut self, f: &mut Frame, area: Rect) {
        // Calculate the actual height needed for the status bar
        let status_text = "Tab: Filter | ↑/↓ or Ctrl+P/N: Navigate | Enter: View details | Ctrl+S: View full session | Ctrl+T: Toggle preview | Esc: Exit | ?: Help";
        let status_paragraph = Paragraph::new(status_text).wrap(Wrap { trim: true });
        let status_height = (status_paragraph.line_count(area.width) as u16).clamp(1, 3);
        
        // Split area into title, content (list), and status
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(RESULT_LIST_TITLE_HEIGHT),  // Title
                Constraint::Min(0),                            // Content (list)
                Constraint::Length(status_height),             // Status (dynamic height)
            ])
            .split(area);

        // Render title
        let title_lines = vec![Line::from(vec![Span::styled(
            "Search Results",
            Styles::title(),
        )])];
        let title = Paragraph::new(title_lines).block(Block::default().borders(Borders::BOTTOM));
        f.render_widget(title, chunks[0]);

        // Render list
        self.list_viewer.render(f, chunks[1]);

        // Render status bar (updated to include Ctrl+S and Tab: Filter)
        let status_text = "Tab: Filter | ↑/↓ or Ctrl+P/N: Navigate | Enter: View details | Ctrl+S: View full session | Ctrl+T: Toggle preview | Esc: Exit | ?: Help";
        let status_bar = Paragraph::new(status_text)
            .style(Styles::dimmed())
            .alignment(ratatui::layout::Alignment::Center)
            .wrap(Wrap { trim: true });
        f.render_widget(status_bar, chunks[2]);
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
            KeyCode::Enter => Some(Message::EnterMessageDetail),
            KeyCode::Char('s') if key.modifiers == KeyModifiers::CONTROL => {
                Some(Message::EnterSessionViewer) // Ctrl+S
            }
            KeyCode::Char('t') if key.modifiers == KeyModifiers::CONTROL => {
                Some(Message::TogglePreview) // Ctrl+T
            }
            KeyCode::Esc => {
                // If preview is enabled, close it. Otherwise, let the event bubble up
                if self.preview_enabled {
                    Some(Message::TogglePreview)
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}
