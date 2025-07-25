/// Global keyboard shortcuts handler component
/// This component processes keyboard events that should work regardless of the current mode
use tuirealm::command::{CmdResult, Cmd};
use tuirealm::props::{AttrValue, Attribute, Props};
use tuirealm::{Component, Event, MockComponent, State};
use tuirealm::event::{Key, KeyEvent, KeyModifiers};
use ratatui::Frame;
use ratatui::layout::Rect;
use std::time::Instant;
use tuirealm::NoUserEvent;

use crate::interactive_ratatui::tuirealm_v3::messages::{AppMessage, AppMode};

/// Global shortcuts component that handles keyboard events across all modes
#[derive(Debug, Clone, Default)]
pub struct GlobalShortcuts {
    props: Props,
    last_ctrl_c_press: Option<Instant>,
}

impl GlobalShortcuts {
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Get the current mode from attributes
    fn get_current_mode(&self) -> AppMode {
        self.props
            .get(Attribute::Custom("current_mode"))
            .and_then(|v| match v {
                AttrValue::Number(n) => match n {
                    0 => Some(AppMode::Search),
                    1 => Some(AppMode::ResultDetail),
                    2 => Some(AppMode::SessionViewer),
                    3 => Some(AppMode::Help),
                    4 => Some(AppMode::Error),
                    _ => None,
                },
                _ => None,
            })
            .unwrap_or(AppMode::Search)
    }
}

impl MockComponent for GlobalShortcuts {
    fn view(&mut self, _frame: &mut Frame, _area: Rect) {
        // This component doesn't render anything
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

impl Component<AppMessage, NoUserEvent> for GlobalShortcuts {
    fn on(&mut self, ev: Event<NoUserEvent>) -> Option<AppMessage> {
        match ev {
            Event::Keyboard(key_event) => self.handle_key_event(key_event),
            _ => None,
        }
    }
}

impl GlobalShortcuts {
    fn handle_key_event(&mut self, key: KeyEvent) -> Option<AppMessage> {
        let current_mode = self.get_current_mode();
        
        match (key.code, key.modifiers) {
            // Global quit with double Ctrl+C
            (Key::Char('c'), modifiers) if modifiers == KeyModifiers::CONTROL => {
                if let Some(last_press) = self.last_ctrl_c_press {
                    if last_press.elapsed().as_millis() < 500 {
                        return Some(AppMessage::Quit);
                    }
                }
                self.last_ctrl_c_press = Some(Instant::now());
                Some(AppMessage::ShowMessage("Press Ctrl+C again to quit".to_string()))
            }
            
            // Global help access
            (Key::Char('?'), modifiers) | (Key::Char('h'), modifiers) 
                if modifiers.is_empty() && current_mode != AppMode::Help => {
                Some(AppMessage::ShowHelp)
            }
            
            // Global truncation toggle
            (Key::Char('t'), modifiers) if modifiers == KeyModifiers::CONTROL => {
                Some(AppMessage::ToggleTruncation)
            }
            
            // Mode-specific shortcuts
            _ => self.handle_mode_specific_shortcuts(key, current_mode),
        }
    }
    
    fn handle_mode_specific_shortcuts(&self, key: KeyEvent, mode: AppMode) -> Option<AppMessage> {
        match mode {
            AppMode::Search => self.handle_search_shortcuts(key),
            AppMode::ResultDetail => self.handle_detail_shortcuts(key),
            AppMode::SessionViewer => self.handle_session_shortcuts(key),
            AppMode::Help => self.handle_help_shortcuts(key),
            AppMode::Error => None, // Error mode handles its own shortcuts
        }
    }
    
    fn handle_search_shortcuts(&self, key: KeyEvent) -> Option<AppMessage> {
        match (key.code, key.modifiers) {
            // Vim-style navigation
            (Key::Char('j'), modifiers) if modifiers.is_empty() => Some(AppMessage::ResultDown),
            (Key::Char('k'), modifiers) if modifiers.is_empty() => Some(AppMessage::ResultUp),
            
            // Copy shortcuts
            (Key::Char('y'), modifiers) if modifiers == KeyModifiers::CONTROL => {
                Some(AppMessage::CopyRawJson)
            }
            
            _ => None,
        }
    }
    
    fn handle_detail_shortcuts(&self, key: KeyEvent) -> Option<AppMessage> {
        match (key.code, key.modifiers) {
            // Vim-style navigation
            (Key::Char('j'), modifiers) if modifiers.is_empty() => Some(AppMessage::DetailScrollDown),
            (Key::Char('k'), modifiers) if modifiers.is_empty() => Some(AppMessage::DetailScrollUp),
            
            // Copy shortcuts
            (Key::Char('y'), modifiers) if modifiers == KeyModifiers::CONTROL => {
                Some(AppMessage::CopyRawJson)
            }
            
            _ => None,
        }
    }
    
    fn handle_session_shortcuts(&self, key: KeyEvent) -> Option<AppMessage> {
        match (key.code, key.modifiers) {
            // Vim-style navigation
            (Key::Char('j'), modifiers) if modifiers.is_empty() => Some(AppMessage::SessionScrollDown),
            (Key::Char('k'), modifiers) if modifiers.is_empty() => Some(AppMessage::SessionScrollUp),
            
            // Copy shortcuts
            (Key::Char('y'), modifiers) if modifiers == KeyModifiers::CONTROL => {
                Some(AppMessage::CopyRawJson)
            }
            
            _ => None,
        }
    }
    
    fn handle_help_shortcuts(&self, key: KeyEvent) -> Option<AppMessage> {
        match (key.code, key.modifiers) {
            // Exit help
            (Key::Esc, _) => Some(AppMessage::ExitHelp),
            (Key::Char('q'), modifiers) if modifiers.is_empty() => {
                Some(AppMessage::ExitHelp)
            }
            _ => None,
        }
    }
}

#[cfg(test)]
#[path = "global_shortcuts_test.rs"]
mod tests;