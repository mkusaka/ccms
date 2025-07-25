use tuirealm::command::{Cmd, CmdResult};
use tuirealm::event::{Event, Key, KeyEvent, KeyModifiers};
use tuirealm::props::{AttrValue, Attribute, Props};
use tuirealm::{Component, Frame, MockComponent, NoUserEvent, State};
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::text::{Line, Span, Text};


/// Helper function to extract string from AttrValue
fn unwrap_string(attr: AttrValue) -> String {
    match attr {
        AttrValue::String(s) => s,
        _ => String::new(),
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
#[path = "result_detail_test.rs"]
mod tests;

/// ResultDetail component - shows detailed view of a search result
#[derive(Debug, Clone, Default)]
pub struct ResultDetail {
    props: Props,
}

impl ResultDetail {
    pub fn new() -> Self {
        let mut component = Self::default();
        let borders = tuirealm::props::Borders::default()
            .sides(tuirealm::props::BorderSides::all());
        component.props.set(Attribute::Borders, AttrValue::Borders(borders));
        component
    }
    
    #[allow(clippy::vec_init_then_push)]
    fn format_content(session_id: &str, file: &str, timestamp: &str, role: &str, text: &str) -> Vec<Line<'static>> {
        let mut lines = vec![];
        
        // Metadata section
        lines.push(Line::from(vec![
            Span::styled("Session ID: ", Style::default().fg(Color::Yellow)),
            Span::raw(session_id.to_string()),
        ]));
        
        lines.push(Line::from(vec![
            Span::styled("File: ", Style::default().fg(Color::Yellow)),
            Span::raw(file.to_string()),
        ]));
        
        lines.push(Line::from(vec![
            Span::styled("Timestamp: ", Style::default().fg(Color::Yellow)),
            Span::raw(timestamp.to_string()),
        ]));
        
        lines.push(Line::from(vec![
            Span::styled("Role: ", Style::default().fg(Color::Yellow)),
            Span::styled(
                role.to_string(),
                Style::default().fg(match role {
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
        for line in text.lines() {
            lines.push(Line::from(line.to_string()));
        }
        
        lines
    }
}

impl MockComponent for ResultDetail {
    fn view(&mut self, frame: &mut Frame, area: Rect) {
        // Get data from attributes
        let session_id = self.props
            .get(Attribute::Custom("session_id"))
            .map(unwrap_string);
            
        let file = self.props
            .get(Attribute::Custom("file"))
            .map(unwrap_string);
            
        let timestamp = self.props
            .get(Attribute::Custom("timestamp"))
            .map(unwrap_string);
            
        let role = self.props
            .get(Attribute::Custom("role"))
            .map(unwrap_string);
            
        let text = self.props
            .get(Attribute::Custom("text"))
            .map(unwrap_string);
            
        let scroll_offset = self.props
            .get(Attribute::Custom("scroll_offset"))
            .map(|v| match v {
                AttrValue::String(s) => s.parse::<usize>().unwrap_or(0),
                _ => unwrap_usize(v),
            })
            .unwrap_or(0);
            
        let message = self.props
            .get(Attribute::Custom("message"))
            .map(unwrap_string);
        
        if let (Some(session_id), Some(file), Some(timestamp), Some(role), Some(text)) = 
            (session_id, file, timestamp, role, text) {
            let content = Self::format_content(&session_id, &file, &timestamp, &role, &text);
            let text = Text::from(content);
            
            let title = if let Some(msg) = message {
                format!("Result Detail - {msg}")
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
                let session_id = self.props
                    .get(Attribute::Custom("session_id"))
                    .map(unwrap_string);
                    
                session_id.map(AppMessage::EnterSessionViewer)
            }
            
            _ => None,
        }
    }
}