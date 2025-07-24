use tuirealm::command::{Cmd, CmdResult};
use tuirealm::event::{Event, Key, KeyEvent, KeyModifiers};
use tuirealm::props::{AttrValue, Attribute, Props};
use tuirealm::{Component, Frame, MockComponent, NoUserEvent, State};
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem as RatatuiListItem, Paragraph};
use ratatui::text::{Line, Span};

use crate::interactive_ratatui::tuirealm_v3::messages::AppMessage;

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

/// HelpDialog component - shows keyboard shortcuts
#[derive(Debug, Clone)]
pub struct HelpDialog {
    props: Props,
}

impl Default for HelpDialog {
    fn default() -> Self {
        Self {
            props: Props::default(),
        }
    }
}

impl HelpDialog {
    pub fn new() -> Self {
        let mut component = Self::default();
        let borders = tuirealm::props::Borders::default()
            .sides(tuirealm::props::BorderSides::all());
        component.props.set(Attribute::Borders, AttrValue::Borders(borders));
        component
    }
    
    fn get_help_sections() -> Vec<(&'static str, Vec<(&'static str, &'static str)>)> {
        vec![
            (
                "Navigation",
                vec![
                    ("↑/k", "Move up"),
                    ("↓/j", "Move down"),
                    ("Enter", "Select item"),
                    ("Esc", "Go back"),
                    ("q", "Quit / Go back"),
                ],
            ),
            (
                "Search",
                vec![
                    ("Tab", "Toggle role filter"),
                    ("Enter", "Execute search"),
                    ("Ctrl+U", "Clear line"),
                    ("Ctrl+K", "Delete to end"),
                    ("Ctrl+W", "Delete word"),
                ],
            ),
            (
                "Readline/Emacs",
                vec![
                    ("Ctrl+A", "Go to beginning"),
                    ("Ctrl+E", "Go to end"),
                    ("Ctrl+B/←", "Move left"),
                    ("Ctrl+F/→", "Move right"),
                    ("Ctrl+H", "Delete backward"),
                    ("Ctrl+D", "Delete forward"),
                    ("Alt+B", "Move word backward"),
                    ("Alt+F", "Move word forward"),
                ],
            ),
            (
                "View Options",
                vec![
                    ("t", "Toggle truncation"),
                    ("s", "Open session viewer"),
                    ("o", "Change session order"),
                    ("/", "Search in session"),
                ],
            ),
            (
                "Copy Operations",
                vec![
                    ("c", "Copy message content"),
                    ("y", "Copy session ID"),
                    ("Y", "Copy timestamp"),
                    ("C", "Copy raw JSON"),
                ],
            ),
            (
                "Help",
                vec![
                    ("?", "Show this help"),
                    ("h", "Show this help"),
                ],
            ),
        ]
    }
}

impl MockComponent for HelpDialog {
    fn view(&mut self, frame: &mut Frame, area: Rect) {
        // Calculate centered area
        let popup_width = 60.min(area.width - 4);
        let popup_height = 30.min(area.height - 4);
        
        let horizontal_margin = (area.width - popup_width) / 2;
        let vertical_margin = (area.height - popup_height) / 2;
        
        let popup_area = Rect {
            x: area.x + horizontal_margin,
            y: area.y + vertical_margin,
            width: popup_width,
            height: popup_height,
        };
        
        // Clear the area behind the popup
        frame.render_widget(Clear, popup_area);
        
        // Render help content
        let help_sections = Self::get_help_sections();
        let mut lines = vec![];
        
        for (section_title, shortcuts) in help_sections {
            // Section header
            lines.push(Line::from(vec![
                Span::styled(
                    format!("{}:", section_title),
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
            ]));
            
            // Shortcuts
            for (key, description) in shortcuts {
                lines.push(Line::from(vec![
                    Span::raw("  "),
                    Span::styled(
                        format!("{:<15}", key),
                        Style::default().fg(Color::Yellow),
                    ),
                    Span::raw(description.to_string()),
                ]));
            }
            
            // Empty line between sections
            lines.push(Line::from(""));
        }
        
        // Create scrollable list
        let items: Vec<RatatuiListItem> = lines
            .into_iter()
            .map(|line| RatatuiListItem::new(line))
            .collect();
        
        let list = List::new(items)
            .block(
                Block::default()
                    .title("Help - Press Esc or q to close")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Cyan)),
            );
        
        frame.render_widget(list, popup_area);
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

impl Component<AppMessage, NoUserEvent> for HelpDialog {
    fn on(&mut self, ev: Event<NoUserEvent>) -> Option<AppMessage> {
        match ev {
            Event::Keyboard(KeyEvent { code: Key::Esc, modifiers }) if modifiers.is_empty() => {
                Some(AppMessage::ExitHelp)
            }
            Event::Keyboard(KeyEvent { code: Key::Char('q'), modifiers }) if modifiers.is_empty() => {
                Some(AppMessage::ExitHelp)
            }
            _ => None,
        }
    }
}