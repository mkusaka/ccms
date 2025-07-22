use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use crate::interactive_ratatui::ui::events::Message;
use crate::interactive_ratatui::ui::components::Component;
use crate::interactive_ratatui::domain::models::SessionOrder;

pub struct SessionViewer {
    messages: Vec<String>,
    filtered_indices: Vec<usize>,
    selected_index: usize,
    scroll_offset: usize,
    query: String,
    order: Option<SessionOrder>,
    is_searching: bool,
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

    pub fn start_search(&mut self) {
        self.is_searching = true;
        self.query.clear();
    }

    pub fn stop_search(&mut self) {
        self.is_searching = false;
    }
}

impl Component for SessionViewer {
    fn render(&mut self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Search bar
                Constraint::Min(0),    // Messages
            ])
            .split(area);

        // Render search bar
        if self.is_searching {
            let search_text = vec![
                Span::raw(&self.query),
                Span::styled("_", Style::default().add_modifier(Modifier::SLOW_BLINK)),
            ];
            let search_bar = Paragraph::new(Line::from(search_text))
                .block(Block::default()
                    .title("Search in session (Esc to cancel)")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Yellow))
                );
            f.render_widget(search_bar, chunks[0]);
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
            let info_bar = Paragraph::new(info_text)
                .block(Block::default().borders(Borders::ALL));
            f.render_widget(info_bar, chunks[0]);
        }

        // Render message list
        if self.filtered_indices.is_empty() {
            let empty_msg = Paragraph::new("No messages match the search")
                .block(Block::default().title("Session Messages").borders(Borders::ALL))
                .style(Style::default().fg(Color::DarkGray));
            f.render_widget(empty_msg, chunks[1]);
            return;
        }

        let available_height = chunks[1].height.saturating_sub(2);
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
                    Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };

                ListItem::new(self.messages[msg_idx].clone()).style(style)
            })
            .collect();

        let title = format!(
            "Session Messages ({}/{}) - Showing {}-{}",
            self.selected_index + 1,
            self.filtered_indices.len(),
            start + 1,
            end
        );

        let list = List::new(items)
            .block(Block::default().title(title).borders(Borders::ALL));

        f.render_widget(list, chunks[1]);
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
                        if let Some(msg) = self.messages.get(msg_idx) {
                            Some(Message::CopyToClipboard(msg.clone()))
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                }
                KeyCode::Char('C') => {
                    let filtered_messages: Vec<String> = self.filtered_indices.iter()
                        .filter_map(|&idx| self.messages.get(idx).cloned())
                        .collect();
                    Some(Message::CopyToClipboard(filtered_messages.join("\n\n")))
                }
                KeyCode::Backspace | KeyCode::Esc => Some(Message::ExitToSearch),
                _ => None,
            }
        }
    }
}