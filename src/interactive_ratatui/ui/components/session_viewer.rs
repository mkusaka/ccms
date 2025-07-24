use crate::interactive_ratatui::domain::models::SessionOrder;
use crate::interactive_ratatui::domain::session_list_item::SessionListItem;
use crate::interactive_ratatui::ui::components::{
    Component,
    list_viewer::ListViewer,
    view_layout::{ColorScheme, ViewLayout},
};
use crate::interactive_ratatui::ui::events::Message;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

#[derive(Default)]
pub struct SessionViewer {
    list_viewer: ListViewer<SessionListItem>,
    raw_messages: Vec<String>,
    query: String,
    cursor_position: usize,
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
            cursor_position: 0,
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

    pub fn set_truncation_enabled(&mut self, enabled: bool) {
        self.list_viewer.set_truncation_enabled(enabled);
    }

    #[allow(dead_code)]
    pub fn start_search(&mut self) {
        self.is_searching = true;
        self.query.clear();
        self.cursor_position = 0;
    }

    #[allow(dead_code)]
    pub fn stop_search(&mut self) {
        self.is_searching = false;
    }

    #[cfg(test)]
    pub fn set_cursor_position(&mut self, pos: usize) {
        self.cursor_position = pos;
    }

    #[cfg(test)]
    pub fn cursor_position(&self) -> usize {
        self.cursor_position
    }

    #[cfg(test)]
    pub fn query(&self) -> &str {
        &self.query
    }

    /// Find the previous word boundary from the given position
    fn find_prev_word_boundary(&self, from: usize) -> usize {
        let chars: Vec<char> = self.query.chars().collect();
        let mut pos = from;

        // Skip whitespace backwards
        while pos > 0 && chars.get(pos - 1).is_some_and(|c| c.is_whitespace()) {
            pos -= 1;
        }

        // Skip non-whitespace backwards
        while pos > 0 && chars.get(pos - 1).is_some_and(|c| !c.is_whitespace()) {
            pos -= 1;
        }

        pos
    }

    /// Find the next word boundary from the given position
    fn find_next_word_boundary(&self, from: usize) -> usize {
        let chars: Vec<char> = self.query.chars().collect();
        let mut pos = from;
        let len = chars.len();

        // Skip non-whitespace forwards
        while pos < len && chars.get(pos).is_some_and(|c| !c.is_whitespace()) {
            pos += 1;
        }

        // Skip whitespace forwards
        while pos < len && chars.get(pos).is_some_and(|c| c.is_whitespace()) {
            pos += 1;
        }

        pos
    }

    /// Delete from start position to end position and return if query changed
    fn delete_range(&mut self, start: usize, end: usize) -> bool {
        if start >= end || end > self.query.chars().count() {
            return false;
        }

        let byte_start = self
            .query
            .chars()
            .take(start)
            .map(|c| c.len_utf8())
            .sum::<usize>();
        let byte_end = self
            .query
            .chars()
            .take(end)
            .map(|c| c.len_utf8())
            .sum::<usize>();

        self.query.drain(byte_start..byte_end);
        self.cursor_position = start;
        true
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
            let search_text = if self.cursor_position < self.query.chars().count() {
                let (before, after) = self
                    .query
                    .chars()
                    .enumerate()
                    .partition::<Vec<_>, _>(|(i, _)| *i < self.cursor_position);

                let before: String = before.into_iter().map(|(_, c)| c).collect();
                let after: String = after.into_iter().map(|(_, c)| c).collect();

                vec![
                    Span::raw(before),
                    Span::styled(
                        after.chars().next().unwrap_or(' ').to_string(),
                        Style::default().bg(Color::White).fg(Color::Black),
                    ),
                    Span::raw(after.chars().skip(1).collect::<String>()),
                ]
            } else {
                vec![
                    Span::raw(&self.query),
                    Span::styled(" ", Style::default().bg(Color::White).fg(Color::Black)),
                ]
            };

            let search_bar = Paragraph::new(Line::from(search_text)).block(
                Block::default()
                    .title("Search in session (Esc to cancel)")
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
                "↑/↓ or j/k: Navigate | o: Sort | c: Copy JSON | /: Search | Esc: Back".to_string(),
            );

        layout.render(f, area, |f, content_area| {
            self.render_content(f, content_area);
        });
    }

    fn handle_key(&mut self, key: KeyEvent) -> Option<Message> {
        if self.is_searching {
            // Handle Control key combinations
            if key.modifiers.contains(KeyModifiers::CONTROL) {
                match key.code {
                    // Ctrl+A - Move cursor to beginning of line
                    KeyCode::Char('a') => {
                        self.cursor_position = 0;
                        return None;
                    }
                    // Ctrl+E - Move cursor to end of line
                    KeyCode::Char('e') => {
                        self.cursor_position = self.query.chars().count();
                        return None;
                    }
                    // Ctrl+B - Move cursor backward one character
                    KeyCode::Char('b') => {
                        if self.cursor_position > 0 {
                            self.cursor_position -= 1;
                        }
                        return None;
                    }
                    // Ctrl+F - Move cursor forward one character
                    KeyCode::Char('f') => {
                        if self.cursor_position < self.query.chars().count() {
                            self.cursor_position += 1;
                        }
                        return None;
                    }
                    // Ctrl+H - Delete character before cursor (same as backspace)
                    KeyCode::Char('h') => {
                        if self.cursor_position > 0 {
                            let char_pos = self.cursor_position - 1;
                            let byte_start = self
                                .query
                                .chars()
                                .take(char_pos)
                                .map(|c| c.len_utf8())
                                .sum::<usize>();
                            let ch = self.query.chars().nth(char_pos).unwrap();
                            let byte_end = byte_start + ch.len_utf8();

                            self.query.drain(byte_start..byte_end);
                            self.cursor_position -= 1;
                            return Some(Message::SessionQueryChanged(self.query.clone()));
                        }
                        return None;
                    }
                    // Ctrl+D - Delete character under cursor
                    KeyCode::Char('d') => {
                        if self.cursor_position < self.query.chars().count() {
                            let byte_start = self
                                .query
                                .chars()
                                .take(self.cursor_position)
                                .map(|c| c.len_utf8())
                                .sum::<usize>();
                            let ch = self.query.chars().nth(self.cursor_position).unwrap();
                            let byte_end = byte_start + ch.len_utf8();

                            self.query.drain(byte_start..byte_end);
                            return Some(Message::SessionQueryChanged(self.query.clone()));
                        }
                        return None;
                    }
                    // Ctrl+W - Delete word before cursor
                    KeyCode::Char('w') => {
                        if self.cursor_position > 0 {
                            let new_pos = self.find_prev_word_boundary(self.cursor_position);
                            if self.delete_range(new_pos, self.cursor_position) {
                                return Some(Message::SessionQueryChanged(self.query.clone()));
                            }
                        }
                        return None;
                    }
                    // Ctrl+U - Delete from cursor to beginning of line
                    KeyCode::Char('u') => {
                        if self.cursor_position > 0 && self.delete_range(0, self.cursor_position) {
                            return Some(Message::SessionQueryChanged(self.query.clone()));
                        }
                        return None;
                    }
                    // Ctrl+K - Delete from cursor to end of line
                    KeyCode::Char('k') => {
                        let len = self.query.chars().count();
                        if self.cursor_position < len
                            && self.delete_range(self.cursor_position, len)
                        {
                            return Some(Message::SessionQueryChanged(self.query.clone()));
                        }
                        return None;
                    }
                    _ => {}
                }
            }

            // Handle Alt key combinations
            if key.modifiers.contains(KeyModifiers::ALT) {
                match key.code {
                    // Alt+B - Move cursor backward one word
                    KeyCode::Char('b') => {
                        self.cursor_position = self.find_prev_word_boundary(self.cursor_position);
                        return None;
                    }
                    // Alt+F - Move cursor forward one word
                    KeyCode::Char('f') => {
                        self.cursor_position = self.find_next_word_boundary(self.cursor_position);
                        return None;
                    }
                    _ => {}
                }
            }

            // Handle regular keys
            match key.code {
                KeyCode::Char(c) => {
                    // Skip if it was a control character we already handled
                    if key.modifiers.contains(KeyModifiers::CONTROL)
                        || key.modifiers.contains(KeyModifiers::ALT)
                    {
                        return None;
                    }

                    let char_pos = self.cursor_position;
                    let byte_pos = self
                        .query
                        .chars()
                        .take(char_pos)
                        .map(|c| c.len_utf8())
                        .sum::<usize>();

                    self.query.insert(byte_pos, c);
                    self.cursor_position += 1;
                    Some(Message::SessionQueryChanged(self.query.clone()))
                }
                KeyCode::Backspace => {
                    if self.cursor_position > 0 {
                        let char_pos = self.cursor_position - 1;
                        let byte_start = self
                            .query
                            .chars()
                            .take(char_pos)
                            .map(|c| c.len_utf8())
                            .sum::<usize>();
                        let ch = self.query.chars().nth(char_pos).unwrap();
                        let byte_end = byte_start + ch.len_utf8();

                        self.query.drain(byte_start..byte_end);
                        self.cursor_position -= 1;

                        if self.query.is_empty() {
                            self.is_searching = false;
                        }
                        Some(Message::SessionQueryChanged(self.query.clone()))
                    } else {
                        None
                    }
                }
                KeyCode::Delete => {
                    if self.cursor_position < self.query.chars().count() {
                        let byte_start = self
                            .query
                            .chars()
                            .take(self.cursor_position)
                            .map(|c| c.len_utf8())
                            .sum::<usize>();
                        let ch = self.query.chars().nth(self.cursor_position).unwrap();
                        let byte_end = byte_start + ch.len_utf8();

                        self.query.drain(byte_start..byte_end);
                        Some(Message::SessionQueryChanged(self.query.clone()))
                    } else {
                        None
                    }
                }
                KeyCode::Left => {
                    if self.cursor_position > 0 {
                        self.cursor_position -= 1;
                    }
                    None
                }
                KeyCode::Right => {
                    if self.cursor_position < self.query.chars().count() {
                        self.cursor_position += 1;
                    }
                    None
                }
                KeyCode::Home => {
                    self.cursor_position = 0;
                    None
                }
                KeyCode::End => {
                    self.cursor_position = self.query.chars().count();
                    None
                }
                KeyCode::Esc => {
                    self.is_searching = false;
                    self.query.clear();
                    self.cursor_position = 0;
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
                KeyCode::Up | KeyCode::Char('k') => {
                    if self.list_viewer.move_up() {
                        Some(Message::SessionScrollUp)
                    } else {
                        None
                    }
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if self.list_viewer.move_down() {
                        Some(Message::SessionScrollDown)
                    } else {
                        None
                    }
                }
                KeyCode::Char('/') => {
                    self.is_searching = true;
                    self.cursor_position = self.query.chars().count();
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
                KeyCode::Esc => Some(Message::ExitToSearch),
                _ => None,
            }
        }
    }
}
