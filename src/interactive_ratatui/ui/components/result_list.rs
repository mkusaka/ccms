use crate::interactive_ratatui::ui::components::Component;
use crate::interactive_ratatui::ui::events::Message;
use crate::query::condition::SearchResult;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
};

pub struct ResultList {
    results: Vec<SearchResult>,
    selected_index: usize,
    scroll_offset: usize,
}

impl ResultList {
    pub fn new() -> Self {
        Self {
            results: Vec::new(),
            selected_index: 0,
            scroll_offset: 0,
        }
    }

    pub fn set_results(&mut self, results: Vec<SearchResult>) {
        self.results = results;
        self.selected_index = 0;
        self.scroll_offset = 0;
    }

    pub fn set_selected_index(&mut self, index: usize) {
        if index < self.results.len() {
            self.selected_index = index;
        }
    }

    #[allow(dead_code)]
    pub fn selected_result(&self) -> Option<&SearchResult> {
        self.results.get(self.selected_index)
    }

    fn format_timestamp(timestamp: &str) -> String {
        if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(timestamp) {
            dt.format("%Y-%m-%d %H:%M").to_string()
        } else {
            "N/A".to_string()
        }
    }

    fn truncate_message(text: &str, max_width: usize) -> String {
        let text = text.replace('\n', " ");
        let chars: Vec<char> = text.chars().collect();

        if chars.len() <= max_width {
            text
        } else {
            let truncated: String = chars.into_iter().take(max_width - 3).collect();
            format!("{truncated}...")
        }
    }

    fn calculate_visible_range(&self, available_height: u16) -> (usize, usize) {
        let visible_count = available_height as usize;
        let start = self.scroll_offset;
        let end = (start + visible_count).min(self.results.len());
        (start, end)
    }

    fn adjust_scroll_offset(&mut self, available_height: u16) {
        let visible_count = available_height as usize;

        if self.selected_index < self.scroll_offset {
            self.scroll_offset = self.selected_index;
        } else if self.selected_index >= self.scroll_offset + visible_count {
            self.scroll_offset = self.selected_index - visible_count + 1;
        }
    }
}

impl Component for ResultList {
    fn render(&mut self, f: &mut Frame, area: Rect) {
        if self.results.is_empty() {
            let empty_message = Paragraph::new("No results found")
                .block(Block::default().title("Results").borders(Borders::ALL))
                .style(Style::default().fg(Color::DarkGray));
            f.render_widget(empty_message, area);
            return;
        }

        let available_height = area.height.saturating_sub(2); // Account for borders
        self.adjust_scroll_offset(available_height);
        let (start, end) = self.calculate_visible_range(available_height);

        let items: Vec<ListItem> = self.results[start..end]
            .iter()
            .enumerate()
            .map(|(i, result)| {
                let global_index = start + i;
                let is_selected = global_index == self.selected_index;

                let role_color = match result.role.as_str() {
                    "user" => Color::Green,
                    "assistant" => Color::Blue,
                    "system" => Color::Yellow,
                    _ => Color::White,
                };

                let timestamp = Self::format_timestamp(&result.timestamp);
                let content = Self::truncate_message(&result.text, area.width as usize - 35);

                let spans = vec![
                    Span::styled(
                        format!("{timestamp:16} "),
                        Style::default().fg(Color::DarkGray),
                    ),
                    Span::styled(
                        format!("{:10} ", result.role),
                        Style::default().fg(role_color),
                    ),
                    Span::raw(content),
                ];

                let style = if is_selected {
                    Style::default()
                        .bg(Color::DarkGray)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };

                ListItem::new(Line::from(spans)).style(style)
            })
            .collect();

        let title = format!(
            "Results ({}/{}) - Showing {}-{}",
            self.selected_index + 1,
            self.results.len(),
            start + 1,
            end
        );

        let list = List::new(items)
            .block(Block::default().title(title).borders(Borders::ALL))
            .style(Style::default());

        f.render_widget(list, area);
    }

    fn handle_key(&mut self, key: KeyEvent) -> Option<Message> {
        match key.code {
            KeyCode::Up => {
                if self.selected_index > 0 {
                    self.selected_index -= 1;
                    Some(Message::SelectResult(self.selected_index))
                } else {
                    None
                }
            }
            KeyCode::Down => {
                if self.selected_index + 1 < self.results.len() {
                    self.selected_index += 1;
                    Some(Message::SelectResult(self.selected_index))
                } else {
                    None
                }
            }
            KeyCode::PageUp => {
                let new_index = self.selected_index.saturating_sub(10);
                if new_index != self.selected_index {
                    self.selected_index = new_index;
                    Some(Message::SelectResult(self.selected_index))
                } else {
                    None
                }
            }
            KeyCode::PageDown => {
                let new_index =
                    (self.selected_index + 10).min(self.results.len().saturating_sub(1));
                if new_index != self.selected_index {
                    self.selected_index = new_index;
                    Some(Message::SelectResult(self.selected_index))
                } else {
                    None
                }
            }
            KeyCode::Home => {
                if self.selected_index > 0 {
                    self.selected_index = 0;
                    self.scroll_offset = 0;
                    Some(Message::SelectResult(0))
                } else {
                    None
                }
            }
            KeyCode::End => {
                let last_index = self.results.len().saturating_sub(1);
                if self.selected_index < last_index {
                    self.selected_index = last_index;
                    Some(Message::SelectResult(last_index))
                } else {
                    None
                }
            }
            KeyCode::Enter => Some(Message::EnterResultDetail),
            KeyCode::Char('s') => Some(Message::EnterSessionViewer),
            _ => None,
        }
    }
}
