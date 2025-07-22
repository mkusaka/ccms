use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};
use crossterm::event::{KeyCode, KeyEvent};
use crate::interactive_ratatui::ui::events::Message;
use crate::interactive_ratatui::ui::components::Component;

pub struct SearchBar {
    query: String,
    cursor_position: usize,
    is_searching: bool,
    message: Option<String>,
    role_filter: Option<String>,
}

impl SearchBar {
    pub fn new() -> Self {
        Self {
            query: String::new(),
            cursor_position: 0,
            is_searching: false,
            message: None,
            role_filter: None,
        }
    }

    pub fn set_query(&mut self, query: String) {
        self.query = query;
        self.cursor_position = self.query.chars().count();
    }

    pub fn set_searching(&mut self, is_searching: bool) {
        self.is_searching = is_searching;
    }

    pub fn set_message(&mut self, message: Option<String>) {
        self.message = message;
    }

    pub fn set_role_filter(&mut self, role_filter: Option<String>) {
        self.role_filter = role_filter;
    }
}

impl Component for SearchBar {
    fn render(&mut self, f: &mut Frame, area: Rect) {
        let input_text = if self.cursor_position < self.query.chars().count() {
            let (before, after) = self.query.chars()
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

        let mut title = "Search".to_string();
        if let Some(role) = &self.role_filter {
            title.push_str(&format!(" [role:{role}]"));
        }
        if let Some(msg) = &self.message {
            title.push_str(&format!(" - {msg}"));
        }

        let input = Paragraph::new(Line::from(input_text))
            .block(Block::default().title(title).borders(Borders::ALL))
            .style(Style::default().fg(Color::Yellow));

        f.render_widget(input, area);
    }

    fn handle_key(&mut self, key: KeyEvent) -> Option<Message> {
        match key.code {
            KeyCode::Char(c) => {
                let char_pos = self.cursor_position;
                let byte_pos = self.query.chars()
                    .take(char_pos)
                    .map(|c| c.len_utf8())
                    .sum::<usize>();
                
                self.query.insert(byte_pos, c);
                self.cursor_position += 1;
                Some(Message::QueryChanged(self.query.clone()))
            }
            KeyCode::Backspace => {
                if self.cursor_position > 0 {
                    let char_pos = self.cursor_position - 1;
                    let byte_start = self.query.chars()
                        .take(char_pos)
                        .map(|c| c.len_utf8())
                        .sum::<usize>();
                    let ch = self.query.chars().nth(char_pos).unwrap();
                    let byte_end = byte_start + ch.len_utf8();
                    
                    self.query.drain(byte_start..byte_end);
                    self.cursor_position -= 1;
                    Some(Message::QueryChanged(self.query.clone()))
                } else {
                    None
                }
            }
            KeyCode::Delete => {
                if self.cursor_position < self.query.chars().count() {
                    let byte_start = self.query.chars()
                        .take(self.cursor_position)
                        .map(|c| c.len_utf8())
                        .sum::<usize>();
                    let ch = self.query.chars().nth(self.cursor_position).unwrap();
                    let byte_end = byte_start + ch.len_utf8();
                    
                    self.query.drain(byte_start..byte_end);
                    Some(Message::QueryChanged(self.query.clone()))
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
            _ => None,
        }
    }
}