use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph, Wrap},
};
use crossterm::event::{KeyCode, KeyEvent};
use crate::query::condition::SearchResult;
use crate::interactive_ratatui::ui::events::Message;
use crate::interactive_ratatui::ui::components::Component;

pub struct ResultDetail {
    result: Option<SearchResult>,
    scroll_offset: usize,
}

impl ResultDetail {
    pub fn new() -> Self {
        Self {
            result: None,
            scroll_offset: 0,
        }
    }

    pub fn set_result(&mut self, result: SearchResult) {
        self.result = Some(result);
        self.scroll_offset = 0;
    }

    #[allow(dead_code)]
    pub fn clear(&mut self) {
        self.result = None;
        self.scroll_offset = 0;
    }
}

impl Component for ResultDetail {
    fn render(&mut self, f: &mut Frame, area: Rect) {
        let Some(result) = &self.result else {
            return;
        };

        let content = if let Some(raw_json) = &result.raw_json {
            raw_json.clone()
        } else {
            format!(
                "File: {}\nUUID: {}\nTimestamp: {}\nSession ID: {}\nRole: {}\nText: {}\nProject: {}",
                result.file, result.uuid, result.timestamp, result.session_id, 
                result.role, result.text, result.project_path
            )
        };

        let paragraph = Paragraph::new(content)
            .block(
                Block::default()
                    .title(format!(" Result Detail - {} ", result.file))
                    .borders(Borders::ALL)
            )
            .style(Style::default().fg(Color::White))
            .wrap(Wrap { trim: false })
            .scroll((self.scroll_offset as u16, 0));

        f.render_widget(paragraph, area);
    }

    fn handle_key(&mut self, key: KeyEvent) -> Option<Message> {
        match key.code {
            KeyCode::Up => {
                if self.scroll_offset > 0 {
                    self.scroll_offset -= 1;
                }
                None
            }
            KeyCode::Down => {
                self.scroll_offset += 1;
                None
            }
            KeyCode::PageUp => {
                self.scroll_offset = self.scroll_offset.saturating_sub(10);
                None
            }
            KeyCode::PageDown => {
                self.scroll_offset += 10;
                None
            }
            KeyCode::Char('s') => Some(Message::EnterSessionViewer),
            KeyCode::Char('c') => {
                self.result.as_ref().map(|result| Message::CopyToClipboard(result.text.clone()))
            }
            KeyCode::Char('C') => {
                if let Some(result) = &self.result {
                    if let Some(raw_json) = &result.raw_json {
                        Some(Message::CopyToClipboard(raw_json.clone()))
                    } else {
                        let formatted = format!(
                            "File: {}\nUUID: {}\nTimestamp: {}\nSession ID: {}\nRole: {}\nText: {}\nProject: {}",
                            result.file, result.uuid, result.timestamp, result.session_id, 
                            result.role, result.text, result.project_path
                        );
                        Some(Message::CopyToClipboard(formatted))
                    }
                } else {
                    None
                }
            }
            KeyCode::Backspace | KeyCode::Esc => Some(Message::ExitToSearch),
            _ => None,
        }
    }
}