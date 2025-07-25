use crate::interactive_ratatui::domain::models::SessionOrder;
use crate::interactive_ratatui::domain::session_list_item::SessionListItem;
use crate::interactive_ratatui::ui::components::{
    Component,
    list_viewer::ListViewer,
    text_input::TextInput,
    view_layout::{ColorScheme, ViewLayout},
};
use crate::interactive_ratatui::ui::events::Message;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    text::Line,
    widgets::{Block, Borders, Paragraph},
};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

#[derive(Default)]
pub struct SessionViewer {
    #[cfg(test)]
    pub list_viewer: ListViewer<SessionListItem>,
    #[cfg(not(test))]
    list_viewer: ListViewer<SessionListItem>,
    raw_messages: Vec<String>,
    text_input: TextInput,
    order: Option<SessionOrder>,
    is_searching: bool,
    file_path: Option<String>,
    session_id: Option<String>,
    messages_hash: u64,
}

impl SessionViewer {
    pub fn new() -> Self {
        Self {
            list_viewer: ListViewer::new(
                "Session Messages".to_string(),
                "No messages in session".to_string(),
            ),
            raw_messages: Vec::new(),
            text_input: TextInput::new(),
            order: None,
            is_searching: false,
            file_path: None,
            session_id: None,
            messages_hash: 0,
        }
    }

    pub fn set_messages(&mut self, messages: Vec<String>) {
        // Calculate hash of new messages to check if they changed
        let mut hasher = DefaultHasher::new();
        messages.hash(&mut hasher);
        let new_hash = hasher.finish();

        // Only update if messages have changed
        if new_hash != self.messages_hash {
            self.messages_hash = new_hash;
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
    }

    pub fn set_filtered_indices(&mut self, indices: Vec<usize>) {
        self.list_viewer.set_filtered_indices(indices);
    }

    pub fn set_query(&mut self, query: String) {
        self.text_input.set_text(query);
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

    pub fn set_truncation_enabled(&mut self, enabled: bool) {
        self.list_viewer.set_truncation_enabled(enabled);
    }

    #[allow(dead_code)]
    pub fn start_search(&mut self) {
        self.is_searching = true;
        self.text_input.set_text(String::new());
    }

    #[allow(dead_code)]
    pub fn stop_search(&mut self) {
        self.is_searching = false;
    }

    #[cfg(test)]
    pub fn set_cursor_position(&mut self, pos: usize) {
        self.text_input.set_cursor_position(pos);
    }

    #[cfg(test)]
    pub fn cursor_position(&self) -> usize {
        self.text_input.cursor_position()
    }

    #[cfg(test)]
    pub fn query(&self) -> &str {
        self.text_input.text()
    }

    fn render_content(&mut self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Search bar or info bar
                Constraint::Min(0),    // Messages
            ])
            .split(area);

        // Render search bar
        if self.is_searching {
            let search_text = self.text_input.render_cursor_spans();

            let search_bar = Paragraph::new(Line::from(search_text)).block(
                Block::default()
                    .title("Search in session (Esc to cancel, ↑/↓ or Ctrl+P/N to scroll)")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(ColorScheme::SECONDARY)),
            );
            f.render_widget(search_bar, chunks[0]);
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
            f.render_widget(info_bar, chunks[0]);
        }

        // Render message list using ListViewer
        self.list_viewer.render(f, chunks[1]);
    }
}

impl Component for SessionViewer {
    fn render(&mut self, f: &mut Frame, area: Rect) {
        let subtitle = match (&self.session_id, &self.file_path) {
            (Some(session), Some(file)) => format!("Session: {session} | File: {file}"),
            (Some(session), None) => format!("Session: {session}"),
            (None, Some(file)) => format!("File: {file}"),
            (None, None) => String::new(),
        };

        let layout = ViewLayout::new("Session Viewer".to_string())
            .with_subtitle(subtitle)
            .with_status_text(
                "↑/↓ or j/k or Ctrl+P/N: Navigate | Enter: View Detail | o: Sort | c: Copy JSON | /: Search | Esc: Back"
                    .to_string(),
            );

        layout.render(f, area, |f, content_area| {
            self.render_content(f, content_area);
        });
    }

    fn handle_key(&mut self, key: KeyEvent) -> Option<Message> {
        if self.is_searching {
            match key.code {
                KeyCode::Esc => {
                    self.is_searching = false;
                    self.text_input.set_text(String::new());
                    Some(Message::SessionQueryChanged(String::new()))
                }
                KeyCode::Enter => {
                    self.is_searching = false;
                    None
                }
                KeyCode::Up => {
                    self.list_viewer.move_up();
                    Some(Message::SessionNavigated)
                }
                KeyCode::Down => {
                    self.list_viewer.move_down();
                    Some(Message::SessionNavigated)
                }
                KeyCode::Char('p') if key.modifiers == KeyModifiers::CONTROL => {
                    self.list_viewer.move_up();
                    Some(Message::SessionNavigated)
                }
                KeyCode::Char('n') if key.modifiers == KeyModifiers::CONTROL => {
                    self.list_viewer.move_down();
                    Some(Message::SessionNavigated)
                }
                _ => {
                    let changed = self.text_input.handle_key(key);
                    if changed {
                        Some(Message::SessionQueryChanged(
                            self.text_input.text().to_string(),
                        ))
                    } else {
                        None
                    }
                }
            }
        } else {
            match key.code {
                KeyCode::Up | KeyCode::Char('k') => {
                    self.list_viewer.move_up();
                    Some(Message::SessionNavigated)
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    self.list_viewer.move_down();
                    Some(Message::SessionNavigated)
                }
                KeyCode::Char('p') if key.modifiers == KeyModifiers::CONTROL => {
                    self.list_viewer.move_up();
                    Some(Message::SessionNavigated)
                }
                KeyCode::Char('n') if key.modifiers == KeyModifiers::CONTROL => {
                    self.list_viewer.move_down();
                    Some(Message::SessionNavigated)
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
                KeyCode::Char('f') | KeyCode::Char('F') => {
                    self.file_path.clone().map(Message::CopyToClipboard)
                }
                KeyCode::Char('m') | KeyCode::Char('M') => self
                    .list_viewer
                    .get_selected_item()
                    .map(|item| Message::CopyToClipboard(item.content.clone())),
                KeyCode::Enter => self.list_viewer.get_selected_item().and_then(|item| {
                    self.file_path.as_ref().map(|path| {
                        Message::EnterResultDetailFromSession(
                            item.raw_json.clone(),
                            path.clone(),
                            self.session_id.clone(),
                        )
                    })
                }),
                KeyCode::Esc => Some(Message::ExitToSearch),
                _ => None,
            }
        }
    }
}
