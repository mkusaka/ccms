use std::sync::Arc;
use tuirealm::command::{CmdResult, Cmd};
use tuirealm::props::{AttrValue, Attribute, Props};
use tuirealm::{Component, Event, MockComponent, State};
use tuirealm::event::{Key, KeyEvent};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

use crate::interactive_ratatui::tuirealm_v3::messages::AppMessage;
use crate::interactive_ratatui::tuirealm_v3::error::{AppError, RecoverableError};

/// Error dialog component for displaying user-friendly error messages
pub struct ErrorDialog {
    props: Props,
}

impl ErrorDialog {
    pub fn new() -> Self {
        Self {
            props: Props::default(),
        }
    }
    
    /// Helper to get error from attributes
    fn get_error_from_attrs(&mut self) -> Option<RecoverableError> {
        // Get error type
        let error_type = self.props
            .get(Attribute::Custom("error_type"))
            .map(|v| unwrap_string(&v))
            .unwrap_or_default();
            
        if error_type.is_empty() {
            return None;
        }
        
        // Get error details
        let details = self.props
            .get(Attribute::Custom("error_details"))
            .map(|v| unwrap_string(&v))
            .unwrap_or_default();
            
        // Create appropriate error based on type
        let error = match error_type.as_str() {
            "FileReadError" => {
                let path = self.props
                    .get(Attribute::Custom("error_path"))
                    .map(|v| unwrap_string(&v))
                    .unwrap_or_else(|| "unknown".to_string());
                    
                AppError::FileReadError {
                    path,
                    source: Arc::new(std::io::Error::new(std::io::ErrorKind::NotFound, details)),
                }
            }
            "SearchServiceError" => AppError::SearchServiceError { details },
            "ClipboardServiceError" => AppError::ClipboardServiceError { details },
            "InvalidQueryError" => {
                let query = self.props
                    .get(Attribute::Custom("error_query"))
                    .map(|v| unwrap_string(&v))
                    .unwrap_or_default();
                    
                AppError::InvalidQueryError { query, details }
            }
            _ => AppError::Unknown { details },
        };
        
        Some(RecoverableError::new(error))
    }
}

fn unwrap_string(v: &AttrValue) -> String {
    match v {
        AttrValue::String(s) => s.clone(),
        _ => String::new(),
    }
}

impl MockComponent for ErrorDialog {
    fn view(&mut self, frame: &mut Frame, area: Rect) {
        // Get error to display
        let error = match self.get_error_from_attrs() {
            Some(e) => e,
            None => return,
        };
        
        // Calculate popup area (centered, 60% width, auto height)
        let popup_width = (area.width as f32 * 0.6) as u16;
        let popup_x = (area.width.saturating_sub(popup_width)) / 2;
        
        // Count lines needed for the error message
        let error_msg = error.user_message();
        let lines_needed = error_msg.lines().count() as u16 + 4; // +4 for borders and padding
        let popup_height = lines_needed.min(area.height.saturating_sub(4));
        let popup_y = (area.height.saturating_sub(popup_height)) / 2;
        
        let popup_area = Rect {
            x: area.x + popup_x,
            y: area.y + popup_y,
            width: popup_width,
            height: popup_height,
        };
        
        // Clear background
        frame.render_widget(Clear, popup_area);
        
        // Create error dialog
        let block = Block::default()
            .title(" Error ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Red));
            
        let inner_area = block.inner(popup_area);
        frame.render_widget(block, popup_area);
        
        // Split inner area for message and instructions
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(3),
                Constraint::Length(3),
            ])
            .split(inner_area);
            
        // Render error message
        let paragraph = Paragraph::new(error_msg)
            .style(Style::default().fg(Color::White))
            .wrap(Wrap { trim: false })
            .alignment(Alignment::Left);
            
        frame.render_widget(paragraph, chunks[0]);
        
        // Render instructions
        let mut instruction_spans = vec![
            Span::styled("Press ", Style::default().fg(Color::DarkGray)),
            Span::styled("Esc", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::styled(" to close", Style::default().fg(Color::DarkGray)),
        ];
        
        if error.can_retry {
            instruction_spans.push(Span::styled(" | ", Style::default().fg(Color::DarkGray)));
            instruction_spans.push(Span::styled("r", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)));
            instruction_spans.push(Span::styled(" to retry", Style::default().fg(Color::DarkGray)));
        }
        
        let instructions = Paragraph::new(Line::from(instruction_spans))
            .alignment(Alignment::Center);
            
        frame.render_widget(instructions, chunks[1]);
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

impl Component<AppMessage, NoUserEvent> for ErrorDialog {
    fn on(&mut self, ev: Event<NoUserEvent>) -> Option<AppMessage> {
        match ev {
            Event::Keyboard(KeyEvent { code: Key::Esc, .. }) => {
                Some(AppMessage::CloseError)
            }
            Event::Keyboard(KeyEvent { code: Key::Char('r'), modifiers }) if modifiers.is_empty() => {
                // Check if retry is allowed
                if let Some(error) = self.get_error_from_attrs() {
                    if error.can_retry {
                        Some(AppMessage::RetryLastOperation)
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}

use tuirealm::NoUserEvent;

#[cfg(test)]
#[path = "error_dialog_test.rs"]
mod tests;