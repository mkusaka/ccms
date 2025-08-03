use crate::interactive_ratatui::ui::app_state::SessionInfo;
use crate::interactive_ratatui::ui::components::Component;
use crate::interactive_ratatui::ui::events::Message;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
};

#[derive(Default)]
pub struct SessionList {
    sessions: Vec<SessionInfo>,
    selected_index: usize,
    scroll_offset: usize,
    is_loading: bool,
    preview_enabled: bool,
}

impl SessionList {
    pub fn new() -> Self {
        Self {
            sessions: Vec::new(),
            selected_index: 0,
            scroll_offset: 0,
            is_loading: false,
            preview_enabled: false,
        }
    }

    pub fn set_sessions(&mut self, sessions: Vec<SessionInfo>) {
        self.sessions = sessions;
    }

    pub fn set_selected_index(&mut self, index: usize) {
        self.selected_index = index;
    }

    pub fn set_is_loading(&mut self, is_loading: bool) {
        self.is_loading = is_loading;
    }

    pub fn set_preview_enabled(&mut self, enabled: bool) {
        self.preview_enabled = enabled;
    }

    pub fn get_selected_session(&self) -> Option<&SessionInfo> {
        self.sessions.get(self.selected_index)
    }
}

impl Component for SessionList {
    fn render(&mut self, f: &mut Frame, area: Rect) {
        // Split area into sessions list and status bar
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(0),    // Sessions list
                Constraint::Length(2), // Status bar
            ])
            .split(area);

        let block = Block::default().borders(Borders::ALL).title("Sessions");

        if self.is_loading {
            let loading = List::new(vec![ListItem::new("Loading...")]).block(block);
            f.render_widget(loading, chunks[0]);
        } else if self.sessions.is_empty() {
            let empty = List::new(vec![ListItem::new("No sessions found")]).block(block);
            f.render_widget(empty, chunks[0]);
        } else {
            let items: Vec<ListItem> = self
                .sessions
                .iter()
                .enumerate()
                .map(|(i, session)| {
                    // Format timestamp as mm/dd hh:MM
                    let formatted_time = if let Ok(parsed) =
                        chrono::DateTime::parse_from_rfc3339(&session.timestamp)
                    {
                        parsed.format("%m/%d %H:%M").to_string()
                    } else {
                        session.timestamp.chars().take(16).collect::<String>()
                    };

                    let line = Line::from(vec![
                        Span::styled(formatted_time, Style::default().fg(Color::Yellow)),
                        Span::raw(" "),
                        Span::styled(
                            format!("[{}]", session.session_id),
                            Style::default().fg(Color::Cyan),
                        ),
                        Span::raw(format!(" {} messages - ", session.message_count)),
                        Span::styled(&session.first_message, Style::default().fg(Color::DarkGray)),
                    ]);

                    let style = if i == self.selected_index {
                        Style::default()
                            .bg(Color::Rgb(60, 60, 60))
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default()
                    };

                    ListItem::new(line).style(style)
                })
                .collect();

            let visible_height = chunks[0].height.saturating_sub(2) as usize; // -2 for borders

            // Adjust scroll offset to keep selected item visible
            if self.selected_index < self.scroll_offset {
                self.scroll_offset = self.selected_index;
            } else if self.selected_index >= self.scroll_offset + visible_height {
                self.scroll_offset = self.selected_index - visible_height + 1;
            }

            let visible_items: Vec<ListItem> = items
                .into_iter()
                .skip(self.scroll_offset)
                .take(visible_height)
                .collect();

            let list = List::new(visible_items)
                .block(block)
                .style(Style::default().fg(Color::White));

            f.render_widget(list, chunks[0]);
        }

        // Render status bar
        let status_text = if self.preview_enabled {
            "Shift+Tab: Switch tabs | ↑/↓: Navigate | Enter: Open session | Ctrl+S: View session | Ctrl+T: Hide preview | Esc: Exit | ?: Help"
        } else {
            "Shift+Tab: Switch tabs | ↑/↓: Navigate | Enter: Open session | Ctrl+S: View session | Ctrl+T: Show preview | Esc: Exit | ?: Help"
        };
        let status_bar = Paragraph::new(status_text)
            .style(Style::default().fg(Color::DarkGray))
            .alignment(ratatui::layout::Alignment::Center)
            .wrap(Wrap { trim: true });
        f.render_widget(status_bar, chunks[1]);
    }

    fn handle_key(&mut self, key: KeyEvent) -> Option<Message> {
        use crossterm::event::KeyModifiers;

        match key.code {
            KeyCode::Up => Some(Message::SessionListScrollUp),
            KeyCode::Down => Some(Message::SessionListScrollDown),
            KeyCode::PageUp => Some(Message::SessionListPageUp),
            KeyCode::PageDown => Some(Message::SessionListPageDown),
            // Half-page scrolling
            KeyCode::Char('u') if key.modifiers == KeyModifiers::CONTROL => {
                Some(Message::SessionListHalfPageUp)
            }
            KeyCode::Char('d') if key.modifiers == KeyModifiers::CONTROL => {
                Some(Message::SessionListHalfPageDown)
            }
            KeyCode::Enter => {
                if !self.sessions.is_empty() {
                    self.sessions.get(self.selected_index).map(|session| {
                        Message::EnterSessionViewerFromList(session.file_path.clone())
                    })
                } else {
                    None
                }
            }
            KeyCode::Char('s') if key.modifiers == KeyModifiers::CONTROL => {
                if !self.sessions.is_empty() {
                    self.sessions.get(self.selected_index).map(|session| {
                        Message::EnterSessionViewerFromList(session.file_path.clone())
                    })
                } else {
                    None
                }
            }
            KeyCode::Char('t') if key.modifiers == KeyModifiers::CONTROL => {
                Some(Message::ToggleSessionListPreview)
            }
            _ => None,
        }
    }
}
