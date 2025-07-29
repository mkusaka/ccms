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
        // Split area into title, content (list), shortcuts, and status
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Title
                Constraint::Min(0),    // Content (list)
                Constraint::Length(8), // Shortcuts (increased to show all)
                Constraint::Length(2), // Status
            ])
            .split(area);

        // Render title
        let title_lines = vec![
            Line::from(vec![Span::styled("Search Results", Styles::title())]),
            Line::from(vec![Span::raw(format!(
                "{} results found | Ctrl+T: Toggle truncation",
                self.list_viewer.filtered_count()
            ))]),
        ];
        let title = Paragraph::new(title_lines).block(Block::default().borders(Borders::BOTTOM));
        f.render_widget(title, chunks[0]);

        // Render list
        self.list_viewer.render(f, chunks[1]);

        // Render shortcuts
        let shortcuts = vec![
            Line::from(vec![Span::styled("Shortcuts:", Styles::title())]),
            Line::from(vec![
                Span::styled("[↑/↓ or j/k or Ctrl+P/N]", Styles::action_key()),
                Span::styled(" - Navigate", Styles::action_description()),
            ]),
            Line::from(vec![
                Span::styled("[Enter]", Styles::action_key()),
                Span::styled(" - View details", Styles::action_description()),
            ]),
            Line::from(vec![
                Span::styled("[Ctrl+T]", Styles::action_key()),
                Span::styled(" - Toggle truncation", Styles::action_description()),
            ]),
            Line::from(vec![
                Span::styled("[Esc]", Styles::action_key()),
                Span::styled(" - Exit", Styles::action_description()),
            ]),
            Line::from(vec![
                Span::styled("[?]", Styles::action_key()),
                Span::styled(" - Help", Styles::action_description()),
            ]),
        ];

        let shortcuts_widget = Paragraph::new(shortcuts)
            .block(Block::default().borders(Borders::ALL))
            .wrap(Wrap { trim: true });
        f.render_widget(shortcuts_widget, chunks[2]);

        // Render status bar
        let status_text =
            "↑/↓ or j/k or Ctrl+P/N: Navigate | Enter: View details | Esc: Exit | ?: Help";
        let status_bar = Paragraph::new(status_text)
            .style(Styles::dimmed())
            .alignment(ratatui::layout::Alignment::Center);
        f.render_widget(status_bar, chunks[3]);
    }

    fn handle_key(&mut self, key: KeyEvent) -> Option<Message> {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                if self.list_viewer.move_up() {
                    Some(Message::SelectResult(self.list_viewer.selected_index()))
                } else {
                    None
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
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
            KeyCode::Enter => Some(Message::EnterResultDetail),
            KeyCode::Char('s') if key.modifiers == KeyModifiers::CONTROL => {
                Some(Message::EnterSessionViewer) // Ctrl+S
            }
            _ => None,
        }
    }
}
