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
    filtered_sessions: Vec<SessionInfo>,
    query: String,
    selected_index: usize,
    scroll_offset: usize,
    is_loading: bool,
    is_searching: bool,
    preview_enabled: bool,
}

impl SessionList {
    pub fn new() -> Self {
        Self {
            sessions: Vec::new(),
            filtered_sessions: Vec::new(),
            query: String::new(),
            selected_index: 0,
            scroll_offset: 0,
            is_loading: false,
            is_searching: false,
            preview_enabled: true, // Default to true for better UX
        }
    }

    pub fn set_sessions(&mut self, sessions: Vec<SessionInfo>) {
        self.sessions = sessions;
        // Initially show all sessions if there's no search query
        if self.query.is_empty() {
            self.filtered_sessions = self.sessions.clone();
        }
    }

    pub fn set_filtered_sessions(&mut self, sessions: Vec<SessionInfo>) {
        self.filtered_sessions = sessions;
    }

    pub fn set_query(&mut self, query: String) {
        self.query = query;
    }

    pub fn set_selected_index(&mut self, index: usize) {
        self.selected_index = index;
    }

    pub fn set_is_loading(&mut self, is_loading: bool) {
        self.is_loading = is_loading;
    }

    pub fn set_is_searching(&mut self, is_searching: bool) {
        self.is_searching = is_searching;
    }

    pub fn set_preview_enabled(&mut self, enabled: bool) {
        self.preview_enabled = enabled;
    }

    pub fn get_selected_session(&self) -> Option<&SessionInfo> {
        self.filtered_sessions.get(self.selected_index)
    }
}

impl Component for SessionList {
    fn render(&mut self, f: &mut Frame, area: Rect) {
        // Split area into search bar, sessions list and status bar
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Search bar
                Constraint::Min(0),    // Sessions list
                Constraint::Length(2), // Status bar
            ])
            .split(area);

        // Render search bar
        let search_status = if self.is_searching {
            " [searching...]"
        } else {
            ""
        };
        let session_count = if !self.query.is_empty() {
            format!(
                " ({}/{})",
                self.filtered_sessions.len(),
                self.sessions.len()
            )
        } else {
            format!(" ({})", self.sessions.len())
        };
        let search_block = Block::default()
            .borders(Borders::ALL)
            .title(format!("Search Sessions{search_status}{session_count}"));
        let search_text = Paragraph::new(self.query.as_str())
            .style(Style::default().fg(Color::White))
            .block(search_block);
        f.render_widget(search_text, chunks[0]);

        let block = Block::default().borders(Borders::ALL).title("Sessions");

        if self.is_loading {
            let loading = List::new(vec![ListItem::new("Loading...")]).block(block);
            f.render_widget(loading, chunks[1]);
        } else if self.filtered_sessions.is_empty() && !self.query.is_empty() {
            let empty =
                List::new(vec![ListItem::new("No sessions match your search")]).block(block);
            f.render_widget(empty, chunks[1]);
        } else if self.filtered_sessions.is_empty() {
            let empty = List::new(vec![ListItem::new("No sessions found")]).block(block);
            f.render_widget(empty, chunks[1]);
        } else {
            let items: Vec<ListItem> = self
                .filtered_sessions
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
                        Span::raw(" "),
                        Span::styled(&session.file_path, Style::default().fg(Color::Green)),
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

            let visible_height = chunks[1].height.saturating_sub(2) as usize; // -2 for borders

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

            f.render_widget(list, chunks[1]);
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
        f.render_widget(status_bar, chunks[2]);
    }

    fn handle_key(&mut self, key: KeyEvent) -> Option<Message> {
        use crossterm::event::KeyModifiers;

        match key.code {
            // Text input for search
            KeyCode::Char(c)
                if key.modifiers.is_empty() || key.modifiers == KeyModifiers::SHIFT =>
            {
                self.query.push(c);
                Some(Message::SessionListQueryChanged(self.query.clone()))
            }
            KeyCode::Backspace => {
                self.query.pop();
                Some(Message::SessionListQueryChanged(self.query.clone()))
            }
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
                if !self.filtered_sessions.is_empty() {
                    self.filtered_sessions
                        .get(self.selected_index)
                        .map(|session| {
                            Message::EnterSessionViewerFromList(session.file_path.clone())
                        })
                } else {
                    None
                }
            }
            KeyCode::Char('s') if key.modifiers == KeyModifiers::CONTROL => {
                if !self.filtered_sessions.is_empty() {
                    self.filtered_sessions
                        .get(self.selected_index)
                        .map(|session| {
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
