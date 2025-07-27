use crate::interactive_ratatui::ui::components::{Component, text_input::TextInput};
use crate::interactive_ratatui::ui::events::Message;
use crossterm::event::KeyEvent;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    text::Line,
    widgets::{Block, Borders, Paragraph},
};

#[derive(Default)]
pub struct SearchBar {
    text_input: TextInput,
    is_searching: bool,
    message: Option<String>,
    role_filter: Option<String>,
}

impl SearchBar {
    pub fn new() -> Self {
        Self {
            text_input: TextInput::new(),
            is_searching: false,
            message: None,
            role_filter: None,
        }
    }

    pub fn set_query(&mut self, query: String) {
        self.text_input.set_text(query);
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

    pub fn get_query(&self) -> &str {
        self.text_input.text()
    }

    pub fn is_searching(&self) -> bool {
        self.is_searching
    }
}

impl Component for SearchBar {
    fn render(&mut self, f: &mut Frame, area: Rect) {
        let input_text = self.text_input.render_cursor_spans();

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
        let changed = self.text_input.handle_key(key);
        if changed {
            Some(Message::QueryChanged(self.text_input.text().to_string()))
        } else {
            None
        }
    }
}
