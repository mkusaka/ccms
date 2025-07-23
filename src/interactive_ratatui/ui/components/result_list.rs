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
    truncation_enabled: bool,
}

impl ResultList {
    pub fn new() -> Self {
        Self {
            results: Vec::new(),
            selected_index: 0,
            scroll_offset: 0,
            truncation_enabled: true,
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

    #[allow(dead_code)]
    pub fn update_results(&mut self, results: Vec<SearchResult>, selected_index: usize) {
        self.results = results;
        self.selected_index = selected_index;
        self.scroll_offset = 0;
    }

    pub fn set_truncation_enabled(&mut self, enabled: bool) {
        self.truncation_enabled = enabled;
    }

    #[allow(dead_code)]
    pub fn update_selection(&mut self, index: usize) {
        if index < self.results.len() {
            self.selected_index = index;
        }
    }

    pub fn format_timestamp(timestamp: &str) -> String {
        if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(timestamp) {
            dt.format("%m/%d %H:%M").to_string()
        } else {
            "N/A".to_string()
        }
    }

    pub fn truncate_message(text: &str, max_width: usize) -> String {
        let text = text.replace('\n', " ");
        let chars: Vec<char> = text.chars().collect();

        if chars.len() <= max_width {
            text
        } else {
            let truncated: String = chars.into_iter().take(max_width - 3).collect();
            format!("{truncated}...")
        }
    }

    pub fn wrap_text(text: &str, max_width: usize) -> Vec<String> {
        if max_width == 0 {
            return vec![];
        }

        let text = text.replace('\n', " ");
        let mut lines = Vec::new();
        let mut current_line = String::new();
        let mut current_width = 0;

        for word in text.split_whitespace() {
            let word_width = word.chars().count();

            if current_width > 0 && current_width + 1 + word_width > max_width {
                // Start a new line
                lines.push(current_line.clone());
                current_line = word.to_string();
                current_width = word_width;
            } else {
                // Add to current line
                if current_width > 0 {
                    current_line.push(' ');
                    current_width += 1;
                }
                current_line.push_str(word);
                current_width += word_width;
            }
        }

        if !current_line.is_empty() {
            lines.push(current_line);
        }

        if lines.is_empty() {
            vec![String::new()]
        } else {
            lines
        }
    }

    fn calculate_visible_range(
        &self,
        available_height: u16,
        available_width: u16,
    ) -> (usize, usize) {
        if self.truncation_enabled {
            // In truncated mode, each item takes 1 line
            let visible_count = available_height as usize;
            let start = self.scroll_offset;
            let end = (start + visible_count).min(self.results.len());
            (start, end)
        } else {
            // In full text mode, calculate how many items fit
            let start = self.scroll_offset;
            let mut current_height = 0;
            let mut end = start;

            // Calculate available width for text (accounting for timestamp and role)
            let available_text_width = available_width.saturating_sub(35) as usize;

            while end < self.results.len() && current_height < available_height as usize {
                let result = &self.results[end];
                let wrapped_lines = Self::wrap_text(&result.text, available_text_width);
                let item_height = wrapped_lines.len().max(1);

                if current_height + item_height <= available_height as usize {
                    current_height += item_height;
                    end += 1;
                } else {
                    break;
                }
            }

            (start, end)
        }
    }

    fn adjust_scroll_offset(&mut self, available_height: u16, available_width: u16) {
        if self.truncation_enabled {
            // In truncated mode, each item takes 1 line
            let visible_count = available_height as usize;
            if self.selected_index < self.scroll_offset {
                self.scroll_offset = self.selected_index;
            } else if self.selected_index >= self.scroll_offset + visible_count {
                self.scroll_offset = self.selected_index - visible_count + 1;
            }
        } else {
            // In full text mode, calculate which items are visible
            let available_text_width = available_width.saturating_sub(35) as usize;
            let mut current_index = 0;
            let mut current_height = 0;

            // Find which item should be at the top to show selected_index
            while current_index <= self.selected_index && current_index < self.results.len() {
                if current_index == self.selected_index {
                    // If selected item is above current scroll offset, scroll up
                    if current_index < self.scroll_offset {
                        self.scroll_offset = current_index;
                    }
                    break;
                }

                let result = &self.results[current_index];
                let wrapped_lines = Self::wrap_text(&result.text, available_text_width);
                let item_height = wrapped_lines.len().max(1);
                current_height += item_height;

                // If we've exceeded the available height and haven't reached selected_index
                if current_height > available_height as usize && current_index < self.selected_index
                {
                    self.scroll_offset = current_index + 1;
                    current_height = 0;
                }

                current_index += 1;
            }
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
        self.adjust_scroll_offset(available_height, area.width);
        let (start, end) = self.calculate_visible_range(available_height, area.width);

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
                let available_text_width = area.width.saturating_sub(35) as usize;

                if self.truncation_enabled {
                    // Truncated mode - single line
                    let content = Self::truncate_message(&result.text, available_text_width);
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
                } else {
                    // Full text mode - multiple lines
                    let wrapped_lines = Self::wrap_text(&result.text, available_text_width);
                    let mut lines = Vec::new();

                    // First line with metadata
                    let first_line_spans = vec![
                        Span::styled(
                            format!("{timestamp:16} "),
                            Style::default().fg(Color::DarkGray),
                        ),
                        Span::styled(
                            format!("{:10} ", result.role),
                            Style::default().fg(role_color),
                        ),
                        Span::raw(wrapped_lines.first().cloned().unwrap_or_default()),
                    ];
                    lines.push(Line::from(first_line_spans));

                    // Additional lines (indented)
                    for line in wrapped_lines.iter().skip(1) {
                        let indent = " ".repeat(29); // 16 + 1 + 10 + 1 + 1 spaces
                        lines.push(Line::from(format!("{indent}{line}")));
                    }

                    let style = if is_selected {
                        Style::default()
                            .bg(Color::DarkGray)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default()
                    };

                    ListItem::new(lines).style(style)
                }
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
