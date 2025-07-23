use crate::interactive_ratatui::domain::models::SessionOrder;
use crate::interactive_ratatui::domain::session_list_item::SessionListItem;
use crate::interactive_ratatui::ui::components::{Component, list_viewer::ListViewer};
use crate::interactive_ratatui::ui::events::Message;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

#[derive(Default)]
pub struct SessionViewer {
    list_viewer: ListViewer<SessionListItem>,
    raw_messages: Vec<String>,
    query: String,
    order: Option<SessionOrder>,
    is_searching: bool,
    file_path: Option<String>,
    session_id: Option<String>,
}

impl SessionViewer {
    pub fn new() -> Self {
        Self {
            list_viewer: ListViewer::new(
                "Session Messages".to_string(),
                "No messages in session".to_string(),
            ),
            raw_messages: Vec::new(),
            query: String::new(),
            order: None,
            is_searching: false,
            file_path: None,
            session_id: None,
        }
    }

    pub fn set_messages(&mut self, messages: Vec<String>) {
        self.raw_messages = messages;

        // Convert raw messages to SessionListItems
        let items: Vec<SessionListItem> = self
            .raw_messages
            .iter()
            .enumerate()
            .filter_map(|(idx, line)| SessionListItem::from_json_line(idx, line))
            .collect();

        self.list_viewer.set_items(items);
    }

    pub fn set_filtered_indices(&mut self, indices: Vec<usize>) {
        self.list_viewer.set_filtered_indices(indices);
    }

    pub fn set_query(&mut self, query: String) {
        self.query = query;
    }

    pub fn set_order(&mut self, order: Option<SessionOrder>) {
        self.order = order;
    }

    pub fn set_file_path(&mut self, file_path: Option<String>) {
        self.file_path = file_path;
    }

    pub fn set_session_id(&mut self, session_id: Option<String>) {
        self.session_id = session_id;
    }

    pub fn set_selected_index(&mut self, index: usize) {
        self.list_viewer.set_selected_index(index);
    }

    pub fn set_scroll_offset(&mut self, offset: usize) {
        self.list_viewer.set_scroll_offset(offset);
    }

    #[allow(dead_code)]
    pub fn start_search(&mut self) {
        self.is_searching = true;
        self.query.clear();
    }

    #[allow(dead_code)]
    pub fn stop_search(&mut self) {
        self.is_searching = false;
    }
}

impl Component for SessionViewer {
    fn render(&mut self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(5), // Header with metadata
                Constraint::Length(3), // Search bar
                Constraint::Min(0),    // Messages
                Constraint::Length(2), // Status bar
            ])
            .split(area);

        // Header with metadata
        let mut header_lines = vec![Line::from(vec![Span::styled(
            "Session Viewer",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )])];

        if let Some(ref session_id) = self.session_id {
            header_lines.push(Line::from(vec![
                Span::styled("Session: ", Style::default().fg(Color::Yellow)),
                Span::raw(session_id),
            ]));
        }

        if let Some(ref file_path) = self.file_path {
            header_lines.push(Line::from(vec![
                Span::styled("File: ", Style::default().fg(Color::Yellow)),
                Span::raw(file_path),
            ]));
        }

        let header = Paragraph::new(header_lines).block(Block::default().borders(Borders::BOTTOM));
        f.render_widget(header, chunks[0]);

        // Render search bar
        if self.is_searching {
            let search_text = vec![
                Span::raw(&self.query),
                Span::styled("_", Style::default().add_modifier(Modifier::SLOW_BLINK)),
            ];
            let search_bar = Paragraph::new(Line::from(search_text)).block(
                Block::default()
                    .title("Search in session (Esc to cancel)")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Yellow)),
            );
            f.render_widget(search_bar, chunks[1]);
        } else {
            let info_text = format!(
                "Messages: {} (filtered: {}) | Order: {} | Press '/' to search",
                self.list_viewer.items_count(),
                self.list_viewer.filtered_count(),
                match self.order {
                    Some(SessionOrder::Ascending) => "Ascending",
                    Some(SessionOrder::Descending) => "Descending",
                    Some(SessionOrder::Original) => "Original",
                    None => "Default",
                }
            );
            let info_bar = Paragraph::new(info_text).block(Block::default().borders(Borders::ALL));
            f.render_widget(info_bar, chunks[1]);
        }

        // Render message list using ListViewer
        self.list_viewer.render(f, chunks[2]);

        // Status bar
        let status = "↑/↓: Navigate | o: Sort | c: Copy | C: Copy All | I: Copy Session ID | /: Search | Esc: Back";
        let status_bar = Paragraph::new(status).style(Style::default().fg(Color::DarkGray));
        f.render_widget(status_bar, chunks[3]);
    }

    fn handle_key(&mut self, key: KeyEvent) -> Option<Message> {
        if self.is_searching {
            match key.code {
                KeyCode::Char(c) => {
                    self.query.push(c);
                    Some(Message::SessionQueryChanged(self.query.clone()))
                }
                KeyCode::Backspace => {
                    self.query.pop();
                    if self.query.is_empty() {
                        self.is_searching = false;
                    }
                    Some(Message::SessionQueryChanged(self.query.clone()))
                }
                KeyCode::Esc => {
                    self.is_searching = false;
                    self.query.clear();
                    Some(Message::SessionQueryChanged(String::new()))
                }
                KeyCode::Enter => {
                    self.is_searching = false;
                    None
                }
                _ => None,
            }
        } else {
            match key.code {
                KeyCode::Up => {
                    if self.list_viewer.move_up() {
                        Some(Message::SessionScrollUp)
                    } else {
                        None
                    }
                }
                KeyCode::Down => {
                    if self.list_viewer.move_down() {
                        Some(Message::SessionScrollDown)
                    } else {
                        None
                    }
                }
                KeyCode::Char('/') => {
                    self.is_searching = true;
                    None
                }
                KeyCode::Char('o') => Some(Message::ToggleSessionOrder),
                KeyCode::Char('c') => self
                    .list_viewer
                    .get_selected_item()
                    .map(|item| Message::CopyToClipboard(item.raw_json.clone())),
                KeyCode::Char('C') => {
                    // Copy all raw messages for now
                    // TODO: Add method to ListViewer to get filtered items
                    Some(Message::CopyToClipboard(self.raw_messages.join("\n\n")))
                }
                KeyCode::Char('i') | KeyCode::Char('I') => {
                    self.session_id.clone().map(Message::CopyToClipboard)
                }
                KeyCode::Backspace | KeyCode::Esc => Some(Message::ExitToSearch),
                _ => None,
            }
        }
    }
}
