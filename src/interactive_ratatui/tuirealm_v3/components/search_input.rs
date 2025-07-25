use tuirealm::command::{Cmd, CmdResult};
use tuirealm::event::{Event, Key, KeyEvent, KeyModifiers};
use tuirealm::props::{Alignment, AttrValue, Attribute, Props, Borders as TuirealmBorders};
use tuirealm::{Component, Frame, MockComponent, NoUserEvent, State, StateValue};
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::widgets::{Block, Borders as RatatuiBorders, Paragraph};
use ratatui::text::{Line, Span};

use crate::interactive_ratatui::tuirealm_v3::messages::AppMessage;

#[cfg(test)]
#[path = "search_input_test.rs"]
mod tests;

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

/// Helper function to extract title from AttrValue
fn unwrap_title(attr: AttrValue) -> Option<(String, Alignment)> {
    match attr {
        AttrValue::Title((s, a)) => Some((s, a)),
        _ => None,
    }
}

/// Internal state for SearchInput component
#[derive(Debug, Clone)]
struct SearchInputState {
    cursor_position: usize,
}

/// SearchInput component - follows tui-realm best practices
#[derive(Debug, Clone)]
pub struct SearchInput {
    props: Props,
    state: SearchInputState,
}

impl Default for SearchInput {
    fn default() -> Self {
        Self {
            props: Props::default(),
            state: SearchInputState {
                cursor_position: 0,
            },
        }
    }
}

impl SearchInput {
    pub fn new() -> Self {
        let mut props = Props::default();
        props.set(Attribute::Title, AttrValue::Title(("Search".to_string(), Alignment::Left)));
        // Create tuirealm Borders with all sides
        let borders = TuirealmBorders::default()
            .sides(tuirealm::props::BorderSides::all());
        props.set(Attribute::Borders, AttrValue::Borders(borders));
        // Initialize text to empty string
        props.set(Attribute::Text, AttrValue::String(String::new()));
        
        Self {
            props,
            state: SearchInputState {
                cursor_position: 0,
            },
        }
    }
}

impl MockComponent for SearchInput {
    fn view(&mut self, frame: &mut Frame, area: Rect) {
        // Get data from attributes
        let query = self.props
            .get(Attribute::Text)
            .map(unwrap_string)
            .unwrap_or_default();
            
        let is_searching = self.props
            .get(Attribute::Custom("is_searching"))
            .map(unwrap_bool)
            .unwrap_or(false);
            
        let is_typing = self.props
            .get(Attribute::Custom("is_typing"))
            .map(unwrap_bool)
            .unwrap_or(false);
            
        let role_filter = self.props
            .get(Attribute::Custom("role_filter"))
            .map(unwrap_string);
            
        let message = self.props
            .get(Attribute::Custom("message"))
            .map(unwrap_string);
        
        // Build the search bar content
        let mut spans = vec![];
        
        // Role filter indicator
        if let Some(filter) = role_filter {
            spans.push(Span::styled(
                format!("[{filter}] "),
                ratatui::style::Style::default().fg(Color::Yellow),
            ));
        }
        
        // Cursor visualization
        let (before_cursor, at_cursor, after_cursor) = if self.state.cursor_position <= query.chars().count() {
            let chars: Vec<char> = query.chars().collect();
            let before = chars[..self.state.cursor_position].iter().collect::<String>();
            let at = chars.get(self.state.cursor_position).map(|&c| c.to_string()).unwrap_or(" ".to_string());
            let after = chars[(self.state.cursor_position + 1).min(chars.len())..].iter().collect::<String>();
            (before, at, after)
        } else {
            (query.clone(), " ".to_string(), String::new())
        };
        
        spans.push(Span::raw(before_cursor));
        spans.push(Span::styled(
            at_cursor,
            ratatui::style::Style::default()
                .bg(Color::White)
                .fg(Color::Black),
        ));
        spans.push(Span::raw(after_cursor));
        
        // Status indicators
        if is_searching {
            spans.push(Span::styled(
                " (searching...)",
                ratatui::style::Style::default().fg(Color::Cyan),
            ));
        } else if is_typing {
            spans.push(Span::styled(
                " (typing...)",
                ratatui::style::Style::default().fg(Color::DarkGray),
            ));
        } else if let Some(msg) = message {
            spans.push(Span::styled(
                format!(" ({msg})"),
                ratatui::style::Style::default().fg(Color::Red),
            ));
        }
        
        let title = self.props
            .get(Attribute::Title)
            .and_then(unwrap_title)
            .map(|(s, _)| s)
            .unwrap_or_else(|| "Search".to_string());
        
        let paragraph = Paragraph::new(Line::from(spans))
            .block(Block::default()
                .title(title)
                .borders(RatatuiBorders::ALL)
                .border_style(ratatui::style::Style::default().fg(Color::Cyan)));
        
        frame.render_widget(paragraph, area);
    }
    
    fn query(&self, attr: Attribute) -> Option<AttrValue> {
        self.props.get(attr)
    }
    
    fn attr(&mut self, attr: Attribute, value: AttrValue) {
        // When text changes, update cursor position
        if attr == Attribute::Text {
            if let AttrValue::String(text) = &value {
                let text_len = text.chars().count();
                // Move cursor to end of text when text is set
                self.state.cursor_position = text_len;
            }
        }
        self.props.set(attr, value);
    }
    
    fn state(&self) -> State {
        State::One(StateValue::Usize(self.state.cursor_position))
    }
    
    fn perform(&mut self, cmd: Cmd) -> CmdResult {
        match cmd {
            Cmd::Move(tuirealm::command::Direction::Left) => {
                if self.state.cursor_position > 0 {
                    self.state.cursor_position -= 1;
                    CmdResult::Changed(self.state())
                } else {
                    CmdResult::None
                }
            }
            Cmd::Move(tuirealm::command::Direction::Right) => {
                let text_len = self.props
                    .get(Attribute::Text)
                    .map(|v| v.unwrap_string())
                    .map(|s| s.chars().count())
                    .unwrap_or(0);
                    
                if self.state.cursor_position < text_len {
                    self.state.cursor_position += 1;
                    CmdResult::Changed(self.state())
                } else {
                    CmdResult::None
                }
            }
            Cmd::GoTo(tuirealm::command::Position::Begin) => {
                if self.state.cursor_position != 0 {
                    self.state.cursor_position = 0;
                    CmdResult::Changed(self.state())
                } else {
                    CmdResult::None
                }
            }
            Cmd::GoTo(tuirealm::command::Position::End) => {
                let text_len = self.props
                    .get(Attribute::Text)
                    .map(|v| v.unwrap_string())
                    .map(|s| s.chars().count())
                    .unwrap_or(0);
                    
                if self.state.cursor_position != text_len {
                    self.state.cursor_position = text_len;
                    CmdResult::Changed(self.state())
                } else {
                    CmdResult::None
                }
            }
            _ => CmdResult::None,
        }
    }
}

impl Component<AppMessage, NoUserEvent> for SearchInput {
    fn on(&mut self, ev: Event<NoUserEvent>) -> Option<AppMessage> {
        match ev {
            // Global shortcuts
            Event::Keyboard(KeyEvent { code: Key::Char('c'), modifiers: KeyModifiers::CONTROL }) => {
                Some(AppMessage::Quit)
            }
            Event::Keyboard(KeyEvent { code: Key::Char('t'), modifiers: KeyModifiers::CONTROL }) => {
                Some(AppMessage::ToggleTruncation)
            }
            Event::Keyboard(KeyEvent { code: Key::Char('?'), modifiers }) if modifiers.is_empty() => {
                Some(AppMessage::ShowHelp)
            }
            Event::Keyboard(KeyEvent { code: Key::Char('h'), modifiers }) if modifiers.is_empty() => {
                Some(AppMessage::ShowHelp)
            }
            
            // Character input
            Event::Keyboard(KeyEvent { code: Key::Char(c), modifiers }) if modifiers.is_empty() => {
                let mut query = self.props
                    .get(Attribute::Text)
                    .map(|v| v.unwrap_string())
                    .unwrap_or_default();
                    
                let chars: Vec<char> = query.chars().collect();
                // Ensure cursor position is within bounds
                let cursor_pos = self.state.cursor_position.min(chars.len());
                
                let mut new_chars = chars[..cursor_pos].to_vec();
                new_chars.push(c);
                new_chars.extend_from_slice(&chars[cursor_pos..]);
                
                query = new_chars.into_iter().collect();
                self.state.cursor_position = cursor_pos + 1;
                
                Some(AppMessage::SearchQueryChanged(query))
            }
            
            // Navigation shortcuts
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
            
            // Readline/Emacs shortcuts
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
            
            // Deletion shortcuts
            Event::Keyboard(KeyEvent { code: Key::Backspace, .. }) |
            Event::Keyboard(KeyEvent { code: Key::Char('h'), modifiers: KeyModifiers::CONTROL }) => {
                if self.state.cursor_position > 0 {
                    let mut query = self.props
                        .get(Attribute::Text)
                        .map(|v| v.unwrap_string())
                        .unwrap_or_default();
                        
                    let chars: Vec<char> = query.chars().collect();
                    // Ensure cursor position is within bounds
                    let cursor_pos = self.state.cursor_position.min(chars.len());
                    if cursor_pos > 0 {
                        let mut new_chars = chars[..cursor_pos - 1].to_vec();
                        new_chars.extend_from_slice(&chars[cursor_pos..]);
                        
                        query = new_chars.into_iter().collect();
                        self.state.cursor_position = cursor_pos - 1;
                        
                        Some(AppMessage::SearchQueryChanged(query))
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            
            Event::Keyboard(KeyEvent { code: Key::Delete, .. }) |
            Event::Keyboard(KeyEvent { code: Key::Char('d'), modifiers: KeyModifiers::CONTROL }) => {
                let mut query = self.props
                    .get(Attribute::Text)
                    .map(|v| v.unwrap_string())
                    .unwrap_or_default();
                    
                let chars: Vec<char> = query.chars().collect();
                if self.state.cursor_position < chars.len() {
                    let mut new_chars = chars[..self.state.cursor_position].to_vec();
                    new_chars.extend_from_slice(&chars[self.state.cursor_position + 1..]);
                    
                    query = new_chars.into_iter().collect();
                    Some(AppMessage::SearchQueryChanged(query))
                } else {
                    None
                }
            }
            
            // Ctrl+U - Delete to beginning
            Event::Keyboard(KeyEvent { code: Key::Char('u'), modifiers: KeyModifiers::CONTROL }) => {
                if self.state.cursor_position > 0 {
                    let query = self.props
                        .get(Attribute::Text)
                        .map(|v| v.unwrap_string())
                        .unwrap_or_default();
                        
                    let chars: Vec<char> = query.chars().collect();
                    let cursor_pos = self.state.cursor_position.min(chars.len());
                    let new_query: String = chars[cursor_pos..].iter().collect();
                    self.state.cursor_position = 0;
                    
                    Some(AppMessage::SearchQueryChanged(new_query))
                } else {
                    None
                }
            }
            
            // Ctrl+K - Delete to end
            Event::Keyboard(KeyEvent { code: Key::Char('k'), modifiers: KeyModifiers::CONTROL }) => {
                let query = self.props
                    .get(Attribute::Text)
                    .map(|v| v.unwrap_string())
                    .unwrap_or_default();
                    
                let chars: Vec<char> = query.chars().collect();
                if self.state.cursor_position < chars.len() {
                    let new_query: String = chars[..self.state.cursor_position].iter().collect();
                    Some(AppMessage::SearchQueryChanged(new_query))
                } else {
                    None
                }
            }
            
            // Ctrl+W - Delete word before cursor
            Event::Keyboard(KeyEvent { code: Key::Char('w'), modifiers: KeyModifiers::CONTROL }) => {
                if self.state.cursor_position > 0 {
                    let query = self.props
                        .get(Attribute::Text)
                        .map(|v| v.unwrap_string())
                        .unwrap_or_default();
                        
                    let chars: Vec<char> = query.chars().collect();
                    let cursor_pos = self.state.cursor_position.min(chars.len());
                    let before_cursor: String = chars[..cursor_pos].iter().collect();
                    let after_cursor: String = chars[cursor_pos..].iter().collect();
                    
                    // Find word boundary
                    let trimmed = before_cursor.trim_end();
                    let last_space = trimmed.rfind(' ').map(|i| i + 1).unwrap_or(0);
                    
                    let new_before = &before_cursor[..last_space];
                    let new_query = format!("{new_before}{after_cursor}");
                    self.state.cursor_position = new_before.chars().count();
                    
                    Some(AppMessage::SearchQueryChanged(new_query))
                } else {
                    None
                }
            }
            
            // Alt+B - Move word backward
            Event::Keyboard(KeyEvent { code: Key::Char('b'), modifiers: KeyModifiers::ALT }) => {
                if self.state.cursor_position > 0 {
                    let query = self.props
                        .get(Attribute::Text)
                        .map(|v| v.unwrap_string())
                        .unwrap_or_default();
                        
                    let chars: Vec<char> = query.chars().collect();
                    let cursor_pos = self.state.cursor_position.min(chars.len());
                    let before_cursor: String = chars[..cursor_pos].iter().collect();
                    
                    let trimmed = before_cursor.trim_end();
                    let last_space = trimmed.rfind(' ').map(|i| i + 1).unwrap_or(0);
                    
                    self.state.cursor_position = last_space;
                }
                None
            }
            
            // Alt+F - Move word forward
            Event::Keyboard(KeyEvent { code: Key::Char('f'), modifiers: KeyModifiers::ALT }) => {
                let query = self.props
                    .get(Attribute::Text)
                    .map(|v| v.unwrap_string())
                    .unwrap_or_default();
                    
                let chars: Vec<char> = query.chars().collect();
                if self.state.cursor_position < chars.len() {
                    let after_cursor: String = chars[self.state.cursor_position..].iter().collect();
                    let next_space = after_cursor.find(' ').unwrap_or(after_cursor.len());
                    let skip_spaces = after_cursor[next_space..].chars().take_while(|&c| c == ' ').count();
                    
                    self.state.cursor_position = (self.state.cursor_position + next_space + skip_spaces).min(chars.len());
                }
                None
            }
            
            // Enter to search
            Event::Keyboard(KeyEvent { code: Key::Enter, .. }) => {
                Some(AppMessage::SearchRequested)
            }
            
            // Tab to toggle role filter
            Event::Keyboard(KeyEvent { code: Key::Tab, .. }) => {
                Some(AppMessage::ToggleRoleFilter)
            }
            
            _ => None,
        }
    }
}