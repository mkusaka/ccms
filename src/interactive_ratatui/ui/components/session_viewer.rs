use crate::interactive_ratatui::domain::models::SessionOrder;
use crate::interactive_ratatui::ui::components::Component;
use crate::interactive_ratatui::ui::events::Message;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
};

pub struct SessionViewer {
    pub(super) messages: Vec<String>,
    pub(super) filtered_indices: Vec<usize>,
    pub(super) selected_index: usize,
    pub(super) scroll_offset: usize,
    pub(super) query: String,
    pub(super) order: Option<SessionOrder>,
    pub(super) is_searching: bool,
    pub(super) file_path: Option<String>,
    pub(super) session_id: Option<String>,
}

impl SessionViewer {
    pub fn new() -> Self {
        Self {
            messages: Vec::new(),
            filtered_indices: Vec::new(),
            selected_index: 0,
            scroll_offset: 0,
            query: String::new(),
            order: None,
            is_searching: false,
            file_path: None,
            session_id: None,
        }
    }

    pub fn set_messages(&mut self, messages: Vec<String>) {
        self.messages = messages;
        self.filtered_indices = (0..self.messages.len()).collect();
        self.selected_index = 0;
        self.scroll_offset = 0;
    }

    pub fn set_filtered_indices(&mut self, indices: Vec<usize>) {
        self.filtered_indices = indices;
        if self.selected_index >= self.filtered_indices.len() {
            self.selected_index = 0;
            self.scroll_offset = 0;
        }
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
        let mut header_lines = vec![
            Line::from(vec![Span::styled(
                "Session Viewer",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )]),
        ];
        
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
        
        let header = Paragraph::new(header_lines)
            .block(Block::default().borders(Borders::BOTTOM));
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
                self.messages.len(),
                self.filtered_indices.len(),
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

        // Render message list
        if self.filtered_indices.is_empty() && !self.messages.is_empty() {
            // If there are messages but no filtered indices, show all messages
            self.filtered_indices = (0..self.messages.len()).collect();
        }
        
        if self.messages.is_empty() {
            let empty_msg = Paragraph::new("No messages in session")
                .block(
                    Block::default()
                        .title("Session Messages")
                        .borders(Borders::ALL),
                )
                .style(Style::default().fg(Color::DarkGray));
            f.render_widget(empty_msg, chunks[2]);
            
            // Status bar
            let status = "No messages | I: Copy Session ID | Esc: Back";
            let status_bar = Paragraph::new(status).style(Style::default().fg(Color::DarkGray));
            f.render_widget(status_bar, chunks[3]);
            return;
        }
        
        if self.filtered_indices.is_empty() {
            let empty_msg = Paragraph::new("No messages match the search")
                .block(
                    Block::default()
                        .title("Session Messages")
                        .borders(Borders::ALL),
                )
                .style(Style::default().fg(Color::DarkGray));
            f.render_widget(empty_msg, chunks[2]);
            
            // Status bar
            let status = "No matches | I: Copy Session ID | Esc: Back";
            let status_bar = Paragraph::new(status).style(Style::default().fg(Color::DarkGray));
            f.render_widget(status_bar, chunks[3]);
            return;
        }

        let available_height = chunks[2].height.saturating_sub(2);
        let visible_count = available_height as usize;

        // Adjust scroll offset
        if self.selected_index < self.scroll_offset {
            self.scroll_offset = self.selected_index;
        } else if self.selected_index >= self.scroll_offset + visible_count {
            self.scroll_offset = self.selected_index - visible_count + 1;
        }

        let start = self.scroll_offset;
        let end = (start + visible_count).min(self.filtered_indices.len());

        let items: Vec<ListItem> = (start..end)
            .map(|i| {
                let msg_idx = self.filtered_indices[i];
                let is_selected = i == self.selected_index;

                let style = if is_selected {
                    Style::default()
                        .bg(Color::DarkGray)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };

                // Format the message for display
                let msg = &self.messages[msg_idx];
                let display_text = if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(msg) {
                    // Try to extract meaningful information from JSON
                    let role = json_value.get("type")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown");
                    let timestamp = json_value.get("timestamp")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    let content = json_value.get("message")
                        .and_then(|m| m.get("content"))
                        .and_then(|c| c.as_str())
                        .or_else(|| json_value.get("message")
                            .and_then(|m| m.get("content"))
                            .and_then(|c| c.as_array())
                            .and_then(|arr| arr.first())
                            .and_then(|item| item.get("text"))
                            .and_then(|t| t.as_str()))
                        .unwrap_or("");
                    
                    let display_timestamp = if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(timestamp) {
                        dt.format("%m/%d %H:%M").to_string()
                    } else {
                        timestamp.chars().take(16).collect()
                    };
                    
                    let truncated_content = content.chars().take(100).collect::<String>();
                    let suffix = if content.chars().count() > 100 { "..." } else { "" };
                    
                    format!("[{:9}] {} {}{}", role, display_timestamp, truncated_content.replace('\n', " "), suffix)
                } else {
                    // If not valid JSON, just show the raw message
                    msg.chars().take(120).collect::<String>()
                };
                
                ListItem::new(display_text).style(style)
            })
            .collect();

        let title = format!(
            "Session Messages ({}/{}) - Showing {}-{}",
            self.selected_index + 1,
            self.filtered_indices.len(),
            start + 1,
            end
        );

        let list = List::new(items).block(Block::default().title(title).borders(Borders::ALL));

        f.render_widget(list, chunks[2]);
        
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
                    if self.query.is_empty() {
                        self.is_searching = false;
                        None
                    } else {
                        self.query.pop();
                        Some(Message::SessionQueryChanged(self.query.clone()))
                    }
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
                KeyCode::Up => Some(Message::SessionScrollUp),
                KeyCode::Down => Some(Message::SessionScrollDown),
                KeyCode::Char('/') => {
                    self.is_searching = true;
                    None
                }
                KeyCode::Char('o') => Some(Message::ToggleSessionOrder),
                KeyCode::Char('c') => {
                    if let Some(&msg_idx) = self.filtered_indices.get(self.selected_index) {
                        self.messages
                            .get(msg_idx)
                            .map(|msg| Message::CopyToClipboard(msg.clone()))
                    } else {
                        None
                    }
                }
                KeyCode::Char('C') => {
                    let filtered_messages: Vec<String> = self
                        .filtered_indices
                        .iter()
                        .filter_map(|&idx| self.messages.get(idx).cloned())
                        .collect();
                    Some(Message::CopyToClipboard(filtered_messages.join("\n\n")))
                }
                KeyCode::Char('i') | KeyCode::Char('I') => {
                    self.session_id
                        .clone()
                        .map(Message::CopyToClipboard)
                }
                KeyCode::Backspace | KeyCode::Esc => Some(Message::ExitToSearch),
                _ => None,
            }
        }
    }
}
