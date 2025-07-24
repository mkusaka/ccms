use tuirealm::command::{Cmd, CmdResult};
use tuirealm::event::{Event, Key, KeyEvent, KeyModifiers};
use tuirealm::props::{AttrValue, Attribute, Props};
use tuirealm::{Component, Frame, MockComponent, NoUserEvent, State, StateValue};
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::text::{Line, Span, Text};

use crate::query::condition::SearchResult;

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
        AttrValue::Length(n) => n as usize,
        _ => 0,
    }
}
use crate::interactive_ratatui::tuirealm_v3::messages::AppMessage;

/// Internal state for ResultDetail component
#[derive(Debug, Clone)]
struct ResultDetailState {
    // No internal state needed - scroll offset comes from attributes
}

/// ResultDetail component - shows detailed view of a search result
#[derive(Debug, Clone)]
pub struct ResultDetail {
    props: Props,
    state: ResultDetailState,
}

impl Default for ResultDetail {
    fn default() -> Self {
        Self {
            props: Props::default(),
            state: ResultDetailState {},
        }
    }
}

impl ResultDetail {
    pub fn new() -> Self {
        let mut component = Self::default();
        let borders = tuirealm::props::Borders::default()
            .sides(tuirealm::props::BorderSides::all());
        component.props.set(Attribute::Borders, AttrValue::Borders(borders));
        component
    }
    
    fn format_content(result: &SearchResult) -> Vec<Line<'static>> {
        let mut lines = vec![];
        
        // Metadata section
        lines.push(Line::from(vec![
            Span::styled("Session ID: ", Style::default().fg(Color::Yellow)),
            Span::raw(result.session_id.clone()),
        ]));
        
        lines.push(Line::from(vec![
            Span::styled("File: ", Style::default().fg(Color::Yellow)),
            Span::raw(result.file.clone()),
        ]));
        
        lines.push(Line::from(vec![
            Span::styled("Timestamp: ", Style::default().fg(Color::Yellow)),
            Span::raw(result.timestamp.clone()),
        ]));
        
        lines.push(Line::from(vec![
            Span::styled("Role: ", Style::default().fg(Color::Yellow)),
            Span::styled(
                result.role.clone(),
                Style::default().fg(match result.role.as_str() {
                    "User" => Color::Green,
                    "Assistant" => Color::Blue,
                    "System" => Color::Yellow,
                    _ => Color::White,
                }),
            ),
        ]));
        
        lines.push(Line::from("")); // Empty line
        
        // Message content
        lines.push(Line::from(vec![
            Span::styled("Content:", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        ]));
        
        // Split message into lines
        for line in result.text.lines() {
            lines.push(Line::from(line.to_string()));
        }
        
        lines
    }
}

impl MockComponent for ResultDetail {
    fn view(&mut self, frame: &mut Frame, area: Rect) {
        // Get result from attributes
        // TODO: Handle complex types properly in tuirealm v3
        let result: Option<SearchResult> = None;
            
        let scroll_offset = self.props
            .get(Attribute::Custom("scroll_offset"))
            .map(|v| unwrap_usize(v))
            .unwrap_or(0);
            
        let message = self.props
            .get(Attribute::Custom("message"))
            .map(|v| unwrap_string(v));
        
        if let Some(result) = result {
            let content = Self::format_content(&result);
            let text = Text::from(content);
            
            let title = if let Some(msg) = message {
                format!("Result Detail - {}", msg)
            } else {
                "Result Detail - Press 'c' to copy, 'y' for session, 'Y' for timestamp, 'C' for JSON".to_string()
            };
            
            let paragraph = Paragraph::new(text)
                .block(Block::default()
                    .title(title)
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Cyan)))
                .wrap(Wrap { trim: false })
                .scroll((scroll_offset as u16, 0));
                
            frame.render_widget(paragraph, area);
        } else {
            let block = Block::default()
                .title("Result Detail")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray));
                
            frame.render_widget(block, area);
        }
    }
    
    fn query(&self, attr: Attribute) -> Option<AttrValue> {
        self.props.get(attr)
    }
    
    fn attr(&mut self, attr: Attribute, value: AttrValue) {
        self.props.set(attr, value);
    }
    
    fn state(&self) -> State {
        State::None
    }
    
    fn perform(&mut self, _cmd: Cmd) -> CmdResult {
        CmdResult::None
    }
}

impl Component<AppMessage, NoUserEvent> for ResultDetail {
    fn on(&mut self, ev: Event<NoUserEvent>) -> Option<AppMessage> {
        match ev {
            // Navigation
            Event::Keyboard(KeyEvent { code: Key::Esc, modifiers }) if modifiers.is_empty() => {
                Some(AppMessage::ExitResultDetail)
            }
            Event::Keyboard(KeyEvent { code: Key::Char('q'), modifiers }) if modifiers.is_empty() => {
                Some(AppMessage::ExitResultDetail)
            }
            
            // Scrolling
            Event::Keyboard(KeyEvent { code: Key::Up, modifiers }) if modifiers.is_empty() => {
                Some(AppMessage::DetailScrollUp)
            }
            Event::Keyboard(KeyEvent { code: Key::Char('k'), modifiers }) if modifiers.is_empty() => {
                Some(AppMessage::DetailScrollUp)
            }
            
            Event::Keyboard(KeyEvent { code: Key::Down, modifiers }) if modifiers.is_empty() => {
                Some(AppMessage::DetailScrollDown)
            }
            Event::Keyboard(KeyEvent { code: Key::Char('j'), modifiers }) if modifiers.is_empty() => {
                Some(AppMessage::DetailScrollDown)
            }
            
            Event::Keyboard(KeyEvent { code: Key::PageUp, .. }) |
            Event::Keyboard(KeyEvent { code: Key::Char('b'), modifiers: KeyModifiers::CONTROL }) => {
                Some(AppMessage::DetailPageUp)
            }
            
            Event::Keyboard(KeyEvent { code: Key::PageDown, .. }) |
            Event::Keyboard(KeyEvent { code: Key::Char('f'), modifiers: KeyModifiers::CONTROL }) => {
                Some(AppMessage::DetailPageDown)
            }
            
            // Copy operations
            Event::Keyboard(KeyEvent { code: Key::Char('c'), modifiers }) if modifiers.is_empty() => {
                Some(AppMessage::CopyMessage)
            }
            
            Event::Keyboard(KeyEvent { code: Key::Char('y'), modifiers }) if modifiers.is_empty() => {
                Some(AppMessage::CopySession)
            }
            
            Event::Keyboard(KeyEvent { code: Key::Char('Y'), modifiers: KeyModifiers::SHIFT }) => {
                Some(AppMessage::CopyTimestamp)
            }
            
            Event::Keyboard(KeyEvent { code: Key::Char('C'), modifiers: KeyModifiers::SHIFT }) => {
                Some(AppMessage::CopyRawJson)
            }
            
            // Session viewer
            Event::Keyboard(KeyEvent { code: Key::Char('s'), modifiers }) if modifiers.is_empty() => {
                // TODO: Get result from state instead
                None
            }
            
            _ => None,
        }
    }
}