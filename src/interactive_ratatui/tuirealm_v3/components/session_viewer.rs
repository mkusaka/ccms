use tuirealm::command::{Cmd, CmdResult};
use tuirealm::event::{Event, Key, KeyEvent, KeyModifiers};
use tuirealm::props::{AttrValue, Attribute, Props};
use tuirealm::{Component, Frame, MockComponent, NoUserEvent, State, StateValue};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, Borders, List, ListItem as RatatuiListItem, Paragraph};
use ratatui::text::{Line, Span};

use crate::interactive_ratatui::tuirealm_v3::models::SessionOrder;
use crate::interactive_ratatui::tuirealm_v3::type_safe_wrapper::{SessionMessages, TypeSafeAttr};

/// Helper function to extract string from AttrValue
fn unwrap_string(attr: AttrValue) -> String {
    match attr {
        AttrValue::String(s) => s,
        _ => String::new(),
    }
}

/// Helper function to extract bool from AttrValue
fn unwrap_bool(attr: AttrValue) -> bool {
    match attr {
        AttrValue::Flag(b) => b,
        _ => false,
    }
}

/// Helper function to extract usize from AttrValue
fn unwrap_usize(attr: AttrValue) -> usize {
    match attr {
        AttrValue::Length(n) => n,
        _ => 0,
    }
}
use crate::interactive_ratatui::tuirealm_v3::messages::AppMessage;

#[cfg(test)]
#[path = "session_viewer_test.rs"]
mod tests;

/// Internal state for SessionViewer component
#[derive(Debug, Clone)]
struct SessionViewerState {
    search_cursor_position: usize,
}

/// SessionViewer component - displays session messages
#[derive(Debug, Clone)]
pub struct SessionViewer {
    props: Props,
    state: SessionViewerState,
}

impl Default for SessionViewer {
    fn default() -> Self {
        Self {
            props: Props::default(),
            state: SessionViewerState {
                search_cursor_position: 0,
            },
        }
    }
}

impl SessionViewer {
    pub fn new() -> Self {
        let mut component = Self::default();
        let borders = tuirealm::props::Borders::default()
            .sides(tuirealm::props::BorderSides::all());
        component.props.set(Attribute::Borders, AttrValue::Borders(borders));
        // Initialize value to 0 for selected index
        component.props.set(Attribute::Value, AttrValue::Length(0));
        component
    }
    
    fn parse_message_line(line: &str) -> (String, String, String, String) {
        // Try to parse JSON format
        let mut line_copy = line.to_string();
        if let Ok(json) = unsafe { simd_json::serde::from_str::<serde_json::Value>(&mut line_copy) } {
            let timestamp = json.get("timestamp")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let role = json.get("role")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let message = json.get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            
            (timestamp, role, message, line.to_string())
        } else {
            (String::new(), String::new(), line.to_string(), line.to_string())
        }
    }
    
    fn format_message(index: usize, line: &str, truncate: bool, width: usize) -> Vec<Span<'static>> {
        let (timestamp, role, message, _) = Self::parse_message_line(line);
        
        if timestamp.is_empty() {
            // Raw line
            vec![
                Span::styled(format!("{:4} ", index + 1), Style::default().fg(Color::DarkGray)),
                Span::raw(line.to_string()),
            ]
        } else {
            // Formatted message
            let available_width = width.saturating_sub(25); // 4 + 1 + 8 + 1 + 10 + 1
            let display_message = if truncate {
                Self::truncate_message(&message, available_width)
            } else {
                message.replace('\n', " ")
            };
            
            vec![
                Span::styled(format!("{:4} ", index + 1), Style::default().fg(Color::DarkGray)),
                Span::styled(
                    format!("[{}] ", &timestamp[11..19]),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled(
                    format!("{role:10} "),
                    Style::default().fg(match role.as_str() {
                        "User" => Color::Green,
                        "Assistant" => Color::Blue,
                        "System" => Color::Yellow,
                        _ => Color::White,
                    }),
                ),
                Span::raw(display_message),
            ]
        }
    }
    
    fn truncate_message(text: &str, max_width: usize) -> String {
        let text = text.replace('\n', " ");
        let chars: Vec<char> = text.chars().collect();
        
        if chars.len() <= max_width {
            text
        } else {
            let truncated: String = chars.into_iter().take(max_width.saturating_sub(3)).collect();
            format!("{truncated}...")
        }
    }
}

impl MockComponent for SessionViewer {
    fn view(&mut self, frame: &mut Frame, area: Rect) {
        // Get data from attributes
        let messages: Vec<String> = self.props
            .get(Attribute::Custom("session_texts"))
            .and_then(|v| SessionMessages::from_attr_value(&v))
            .map(|sm| sm.0)
            .unwrap_or_default();
            
        // TODO: Better handling of filtered indices
        let _message_count = self.props
            .get(Attribute::Custom("message_count"))
            .map(|v| match v {
                AttrValue::String(s) => s.parse::<usize>().unwrap_or(0),
                _ => unwrap_usize(v),
            })
            .unwrap_or(0);
            
        let filtered_indices: Vec<usize> = (0..messages.len()).collect();
            
        let selected_index = self.props
            .get(Attribute::Value)
            .map(|v| match v {
                AttrValue::String(s) => s.parse::<usize>().unwrap_or(0),
                _ => unwrap_usize(v),
            })
            .unwrap_or(0);
            
        let scroll_offset = self.props
            .get(Attribute::Custom("scroll_offset"))
            .map(|v| match v {
                AttrValue::String(s) => s.parse::<usize>().unwrap_or(0),
                _ => unwrap_usize(v),
            })
            .unwrap_or(0);
            
        let order_str = self.props
            .get(Attribute::Custom("order"))
            .map(unwrap_string)
            .unwrap_or_default();
            
        let order: Option<SessionOrder> = match order_str.as_str() {
            "asc" => Some(SessionOrder::Ascending),
            "desc" => Some(SessionOrder::Descending),
            "original" => Some(SessionOrder::Original),
            _ => None,
        };
            
        let truncate = self.props
            .get(Attribute::Custom("truncate"))
            .map(unwrap_bool)
            .unwrap_or(true);
            
        let is_searching = self.props
            .get(Attribute::Custom("is_searching"))
            .map(unwrap_bool)
            .unwrap_or(false);
            
        let search_query = self.props
            .get(Attribute::Custom("search_query"))
            .map(unwrap_string)
            .unwrap_or_default();
            
        let session_id = self.props
            .get(Attribute::Custom("session_id"))
            .map(unwrap_string)
            .unwrap_or_else(|| "Unknown".to_string());
        
        if is_searching {
            // Search mode - show search bar
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(3), Constraint::Min(0)])
                .split(area);
            
            // Search bar
            let search_spans = vec![
                Span::raw("Search: "),
                Span::raw(search_query.chars().take(self.state.search_cursor_position).collect::<String>()),
                Span::styled(
                    search_query.chars().nth(self.state.search_cursor_position).unwrap_or(' ').to_string(),
                    Style::default().bg(Color::White).fg(Color::Black),
                ),
                Span::raw(search_query.chars().skip(self.state.search_cursor_position + 1).collect::<String>()),
            ];
            
            let search_paragraph = Paragraph::new(Line::from(search_spans))
                .block(Block::default()
                    .title("Search in session")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Yellow)));
            
            frame.render_widget(search_paragraph, chunks[0]);
            
            // Message list
            self.render_message_list(frame, chunks[1], &messages, &filtered_indices, selected_index, scroll_offset, truncate, &session_id, order);
        } else {
            // Normal mode
            self.render_message_list(frame, area, &messages, &filtered_indices, selected_index, scroll_offset, truncate, &session_id, order);
        }
    }
    
    fn query(&self, attr: Attribute) -> Option<AttrValue> {
        self.props.get(attr)
    }
    
    fn attr(&mut self, attr: Attribute, value: AttrValue) {
        // Reset search cursor when search query changes
        if matches!(attr, Attribute::Custom(name) if name == "search_query") {
            self.state.search_cursor_position = 0;
            if let AttrValue::String(query) = &value {
                self.state.search_cursor_position = query.chars().count();
            }
        }
        self.props.set(attr, value);
    }
    
    fn state(&self) -> State {
        State::One(StateValue::Usize(self.state.search_cursor_position))
    }
    
    fn perform(&mut self, cmd: Cmd) -> CmdResult {
        let is_searching = self.props
            .get(Attribute::Custom("is_searching"))
            .map(unwrap_bool)
            .unwrap_or(false);
            
        if !is_searching {
            return CmdResult::None;
        }
        
        match cmd {
            Cmd::Move(tuirealm::command::Direction::Left) => {
                if self.state.search_cursor_position > 0 {
                    self.state.search_cursor_position -= 1;
                    CmdResult::Changed(self.state())
                } else {
                    CmdResult::None
                }
            }
            Cmd::Move(tuirealm::command::Direction::Right) => {
                let query_len = self.props
                    .get(Attribute::Custom("search_query"))
                    .map(unwrap_string)
                    .map(|s| s.chars().count())
                    .unwrap_or(0);
                    
                if self.state.search_cursor_position < query_len {
                    self.state.search_cursor_position += 1;
                    CmdResult::Changed(self.state())
                } else {
                    CmdResult::None
                }
            }
            Cmd::GoTo(tuirealm::command::Position::Begin) => {
                if self.state.search_cursor_position != 0 {
                    self.state.search_cursor_position = 0;
                    CmdResult::Changed(self.state())
                } else {
                    CmdResult::None
                }
            }
            Cmd::GoTo(tuirealm::command::Position::End) => {
                let query_len = self.props
                    .get(Attribute::Custom("search_query"))
                    .map(unwrap_string)
                    .map(|s| s.chars().count())
                    .unwrap_or(0);
                    
                if self.state.search_cursor_position != query_len {
                    self.state.search_cursor_position = query_len;
                    CmdResult::Changed(self.state())
                } else {
                    CmdResult::None
                }
            }
            _ => CmdResult::None,
        }
    }
}

impl SessionViewer {
    #[allow(clippy::too_many_arguments)]
    fn render_message_list(
        &self,
        frame: &mut Frame,
        area: Rect,
        messages: &[String],
        filtered_indices: &[usize],
        selected_index: usize,
        scroll_offset: usize,
        truncate: bool,
        session_id: &str,
        order: Option<SessionOrder>,
    ) {
        if messages.is_empty() {
            let block = Block::default()
                .title(format!("Session: {session_id}"))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray));
                
            let paragraph = Paragraph::new("No messages in session")
                .block(block)
                .style(Style::default().fg(Color::DarkGray));
                
            frame.render_widget(paragraph, area);
            return;
        }
        
        // Create list items
        let visible_height = area.height.saturating_sub(2) as usize;
        let items: Vec<RatatuiListItem> = filtered_indices
            .iter()
            .skip(scroll_offset)
            .take(visible_height)
            .enumerate()
            .map(|(display_idx, &msg_idx)| {
                let is_selected = scroll_offset + display_idx == selected_index;
                let spans = Self::format_message(msg_idx, &messages[msg_idx], truncate, area.width as usize - 2);
                
                let style = if is_selected {
                    Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };
                
                RatatuiListItem::new(Line::from(spans)).style(style)
            })
            .collect();
        
        let title = format!(
            "Session: {} ({}/{}) - Order: {} - 't' to toggle truncation, 'o' to change order",
            session_id,
            if filtered_indices.is_empty() { 0 } else { selected_index + 1 },
            filtered_indices.len(),
            order.map(|o| format!("{o:?}")).unwrap_or_else(|| "None".to_string())
        );
        
        let list = List::new(items)
            .block(Block::default()
                .title(title)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)));
        
        frame.render_widget(list, area);
    }
}

impl Component<AppMessage, NoUserEvent> for SessionViewer {
    fn on(&mut self, ev: Event<NoUserEvent>) -> Option<AppMessage> {
        let is_searching = self.props
            .get(Attribute::Custom("is_searching"))
            .map(unwrap_bool)
            .unwrap_or(false);
        
        if is_searching {
            // Search mode event handling
            match ev {
                Event::Keyboard(KeyEvent { code: Key::Esc, .. }) => {
                    Some(AppMessage::SessionSearchEnd)
                }
                
                Event::Keyboard(KeyEvent { code: Key::Enter, .. }) => {
                    Some(AppMessage::SessionSearchEnd)
                }
                
                Event::Keyboard(KeyEvent { code: Key::Char(c), modifiers }) if modifiers.is_empty() => {
                    let mut query = self.props
                        .get(Attribute::Custom("search_query"))
                        .map(unwrap_string)
                        .unwrap_or_default();
                        
                    let chars: Vec<char> = query.chars().collect();
                    let mut new_chars = chars[..self.state.search_cursor_position].to_vec();
                    new_chars.push(c);
                    new_chars.extend_from_slice(&chars[self.state.search_cursor_position..]);
                    
                    query = new_chars.into_iter().collect();
                    self.state.search_cursor_position += 1;
                    
                    Some(AppMessage::SessionQueryChanged(query))
                }
                
                Event::Keyboard(KeyEvent { code: Key::Backspace, .. }) => {
                    if self.state.search_cursor_position > 0 {
                        let mut query = self.props
                            .get(Attribute::Custom("search_query"))
                            .map(unwrap_string)
                            .unwrap_or_default();
                            
                        if query.is_empty() {
                            return Some(AppMessage::SessionSearchEnd);
                        }
                        
                        let chars: Vec<char> = query.chars().collect();
                        let mut new_chars = chars[..self.state.search_cursor_position - 1].to_vec();
                        new_chars.extend_from_slice(&chars[self.state.search_cursor_position..]);
                        
                        query = new_chars.into_iter().collect();
                        self.state.search_cursor_position -= 1;
                        
                        Some(AppMessage::SessionQueryChanged(query))
                    } else {
                        None
                    }
                }
                
                // Navigation shortcuts in search mode
                Event::Keyboard(KeyEvent { code: Key::Left, .. }) => {
                    self.perform(Cmd::Move(tuirealm::command::Direction::Left));
                    None
                }
                Event::Keyboard(KeyEvent { code: Key::Right, .. }) => {
                    self.perform(Cmd::Move(tuirealm::command::Direction::Right));
                    None
                }
                Event::Keyboard(KeyEvent { code: Key::Home, .. }) => {
                    self.perform(Cmd::GoTo(tuirealm::command::Position::Begin));
                    None
                }
                Event::Keyboard(KeyEvent { code: Key::End, .. }) => {
                    self.perform(Cmd::GoTo(tuirealm::command::Position::End));
                    None
                }
                
                // Readline shortcuts
                Event::Keyboard(KeyEvent { code: Key::Char('a'), modifiers: KeyModifiers::CONTROL }) => {
                    self.perform(Cmd::GoTo(tuirealm::command::Position::Begin));
                    None
                }
                Event::Keyboard(KeyEvent { code: Key::Char('e'), modifiers: KeyModifiers::CONTROL }) => {
                    self.perform(Cmd::GoTo(tuirealm::command::Position::End));
                    None
                }
                Event::Keyboard(KeyEvent { code: Key::Char('b'), modifiers: KeyModifiers::CONTROL }) => {
                    self.perform(Cmd::Move(tuirealm::command::Direction::Left));
                    None
                }
                Event::Keyboard(KeyEvent { code: Key::Char('f'), modifiers: KeyModifiers::CONTROL }) => {
                    self.perform(Cmd::Move(tuirealm::command::Direction::Right));
                    None
                }
                
                _ => None,
            }
        } else {
            // Normal mode event handling
            match ev {
                // Navigation
                Event::Keyboard(KeyEvent { code: Key::Up, modifiers }) if modifiers.is_empty() => {
                    Some(AppMessage::SessionScrollUp)
                }
                Event::Keyboard(KeyEvent { code: Key::Char('k'), modifiers }) if modifiers.is_empty() => {
                    Some(AppMessage::SessionScrollUp)
                }
                
                Event::Keyboard(KeyEvent { code: Key::Down, modifiers }) if modifiers.is_empty() => {
                    Some(AppMessage::SessionScrollDown)
                }
                Event::Keyboard(KeyEvent { code: Key::Char('j'), modifiers }) if modifiers.is_empty() => {
                    Some(AppMessage::SessionScrollDown)
                }
                
                Event::Keyboard(KeyEvent { code: Key::PageUp, .. }) => {
                    Some(AppMessage::SessionPageUp)
                }
                
                Event::Keyboard(KeyEvent { code: Key::PageDown, .. }) => {
                    Some(AppMessage::SessionPageDown)
                }
                
                // Exit
                Event::Keyboard(KeyEvent { code: Key::Esc, modifiers }) if modifiers.is_empty() => {
                    Some(AppMessage::ExitSessionViewer)
                }
                Event::Keyboard(KeyEvent { code: Key::Char('q'), modifiers }) if modifiers.is_empty() => {
                    Some(AppMessage::ExitSessionViewer)
                }
                
                // Search
                Event::Keyboard(KeyEvent { code: Key::Char('/'), modifiers }) if modifiers.is_empty() => {
                    Some(AppMessage::SessionSearchStart)
                }
                
                // Toggle options
                Event::Keyboard(KeyEvent { code: Key::Char('t'), modifiers }) if modifiers.is_empty() => {
                    Some(AppMessage::ToggleTruncation)
                }
                
                Event::Keyboard(KeyEvent { code: Key::Char('o'), modifiers }) if modifiers.is_empty() => {
                    Some(AppMessage::SessionToggleOrder)
                }
                
                // Copy operations
                Event::Keyboard(KeyEvent { code: Key::Char('c'), modifiers }) if modifiers.is_empty() => {
                    Some(AppMessage::CopyMessage)
                }
                
                Event::Keyboard(KeyEvent { code: Key::Char('C'), modifiers: KeyModifiers::SHIFT }) => {
                    Some(AppMessage::CopyRawJson)
                }
                
                Event::Keyboard(KeyEvent { code: Key::Char('Y'), modifiers: KeyModifiers::SHIFT }) => {
                    Some(AppMessage::CopySessionId)
                }
                
                _ => None,
            }
        }
    }
}