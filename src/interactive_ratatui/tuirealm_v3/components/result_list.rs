use tuirealm::command::{Cmd, CmdResult};
use tuirealm::event::{Event, Key, KeyEvent, KeyModifiers};
use tuirealm::props::{AttrValue, Attribute, Props};
use tuirealm::{Component, Frame, MockComponent, NoUserEvent, State, StateValue};
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, Borders, List, ListItem as RatatuiListItem};
use ratatui::text::{Line, Span};

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

/// Internal state for ResultList component
#[derive(Debug, Clone)]
struct ResultListState {
    scroll_offset: usize,
}

/// ResultList component - displays search results
#[derive(Debug, Clone)]
pub struct ResultList {
    props: Props,
    state: ResultListState,
}

impl Default for ResultList {
    fn default() -> Self {
        Self {
            props: Props::default(),
            state: ResultListState {
                scroll_offset: 0,
            },
        }
    }
}

impl ResultList {
    pub fn new() -> Self {
        let mut component = Self::default();
        let borders = tuirealm::props::Borders::default()
            .sides(tuirealm::props::BorderSides::all());
        component.props.set(Attribute::Borders, AttrValue::Borders(borders));
        component
    }
    
    fn format_result(result: &SearchResult, truncate: bool, width: usize) -> Vec<Span<'static>> {
        let timestamp = format!("[{}]", &result.timestamp[11..19]);
        let role = format!("{:10}", result.role);
        
        let available_width = width.saturating_sub(timestamp.len() + role.len() + 3);
        
        let message = if truncate {
            Self::truncate_message(&result.text, available_width)
        } else {
            result.text.replace('\n', " ")
        };
        
        vec![
            Span::styled(timestamp, Style::default().fg(Color::DarkGray)),
            Span::raw(" "),
            Span::styled(
                role,
                Style::default().fg(match result.role.as_str() {
                    "User" => Color::Green,
                    "Assistant" => Color::Blue,
                    "System" => Color::Yellow,
                    _ => Color::White,
                }),
            ),
            Span::raw(" "),
            Span::raw(message),
        ]
    }
    
    fn truncate_message(text: &str, max_width: usize) -> String {
        let text = text.replace('\n', " ");
        let chars: Vec<char> = text.chars().collect();
        
        if chars.len() <= max_width {
            text
        } else {
            let truncated: String = chars.into_iter().take(max_width.saturating_sub(3)).collect();
            format!("{}...", truncated)
        }
    }
}

impl MockComponent for ResultList {
    fn view(&mut self, frame: &mut Frame, area: Rect) {
        // Get data from attributes
        // TODO: Handle complex types properly in tuirealm v3
        // For now, results will be passed differently
        let results = vec![];
            
        let selected_index = self.props
            .get(Attribute::Value)
            .map(|v| unwrap_usize(v))
            .unwrap_or(0);
            
        let truncate = self.props
            .get(Attribute::Custom("truncate"))
            .map(|v| unwrap_bool(v))
            .unwrap_or(true);
        
        if results.is_empty() {
            let block = Block::default()
                .title("Results")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray));
                
            frame.render_widget(block, area);
            return;
        }
        
        // Adjust scroll offset
        let visible_height = area.height.saturating_sub(2) as usize;
        if selected_index < self.state.scroll_offset {
            self.state.scroll_offset = selected_index;
        } else if selected_index >= self.state.scroll_offset + visible_height {
            self.state.scroll_offset = selected_index.saturating_sub(visible_height - 1);
        }
        
        // Create list items
        let items: Vec<RatatuiListItem> = results
            .iter()
            .enumerate()
            .skip(self.state.scroll_offset)
            .take(visible_height)
            .map(|(i, result)| {
                let spans = Self::format_result(result, truncate, area.width as usize - 2);
                let style = if i == selected_index {
                    Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };
                
                RatatuiListItem::new(Line::from(spans)).style(style)
            })
            .collect();
        
        let title = format!(
            "Results ({}/{}) - Showing {}-{}",
            selected_index + 1,
            results.len(),
            self.state.scroll_offset + 1,
            (self.state.scroll_offset + visible_height).min(results.len())
        );
        
        let list = List::new(items)
            .block(Block::default()
                .title(title)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)));
        
        frame.render_widget(list, area);
    }
    
    fn query(&self, attr: Attribute) -> Option<AttrValue> {
        self.props.get(attr)
    }
    
    fn attr(&mut self, attr: Attribute, value: AttrValue) {
        self.props.set(attr, value);
    }
    
    fn state(&self) -> State {
        State::One(StateValue::Usize(self.state.scroll_offset))
    }
    
    fn perform(&mut self, _cmd: Cmd) -> CmdResult {
        // Navigation is handled via attributes
        CmdResult::None
    }
}

impl Component<AppMessage, NoUserEvent> for ResultList {
    fn on(&mut self, ev: Event<NoUserEvent>) -> Option<AppMessage> {
        match ev {
            // Navigation
            Event::Keyboard(KeyEvent { code: Key::Up, modifiers }) if modifiers.is_empty() => {
                Some(AppMessage::ResultUp)
            }
            Event::Keyboard(KeyEvent { code: Key::Char('k'), modifiers }) if modifiers.is_empty() => {
                Some(AppMessage::ResultUp)
            }
            
            Event::Keyboard(KeyEvent { code: Key::Down, modifiers }) if modifiers.is_empty() => {
                Some(AppMessage::ResultDown)
            }
            Event::Keyboard(KeyEvent { code: Key::Char('j'), modifiers }) if modifiers.is_empty() => {
                Some(AppMessage::ResultDown)
            }
            
            Event::Keyboard(KeyEvent { code: Key::PageUp, .. }) |
            Event::Keyboard(KeyEvent { code: Key::Char('b'), modifiers: KeyModifiers::CONTROL }) => {
                Some(AppMessage::ResultPageUp)
            }
            
            Event::Keyboard(KeyEvent { code: Key::PageDown, .. }) |
            Event::Keyboard(KeyEvent { code: Key::Char('f'), modifiers: KeyModifiers::CONTROL }) => {
                Some(AppMessage::ResultPageDown)
            }
            
            Event::Keyboard(KeyEvent { code: Key::Home, modifiers }) if modifiers.is_empty() => {
                Some(AppMessage::ResultHome)
            }
            Event::Keyboard(KeyEvent { code: Key::Char('g'), modifiers }) if modifiers.is_empty() => {
                Some(AppMessage::ResultHome)
            }
            
            Event::Keyboard(KeyEvent { code: Key::End, .. }) |
            Event::Keyboard(KeyEvent { code: Key::Char('G'), modifiers: KeyModifiers::SHIFT }) => {
                Some(AppMessage::ResultEnd)
            }
            
            // Selection
            Event::Keyboard(KeyEvent { code: Key::Enter, .. }) => {
                let selected_index = self.props
                    .get(Attribute::Value)
                    .map(|v| unwrap_usize(v))
                    .unwrap_or(0);
                    
                Some(AppMessage::EnterResultDetail(selected_index))
            }
            
            // Session viewer
            Event::Keyboard(KeyEvent { code: Key::Char('s'), modifiers }) if modifiers.is_empty() => {
                let selected_index = self.props
                    .get(Attribute::Value)
                    .map(|v| unwrap_usize(v))
                    .unwrap_or(0);
                    
                // TODO: Handle complex types properly in tuirealm v3
                let results: Vec<SearchResult> = vec![];
                    
                if let Some(result) = results.get(selected_index) {
                    Some(AppMessage::EnterSessionViewer(result.session_id.clone()))
                } else {
                    None
                }
            }
            
            // Toggle truncation
            Event::Keyboard(KeyEvent { code: Key::Char('t'), modifiers }) if modifiers.is_empty() => {
                Some(AppMessage::ToggleTruncation)
            }
            
            _ => None,
        }
    }
}