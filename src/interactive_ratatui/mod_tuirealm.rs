use anyhow::{Context, Result};
use tuirealm::application::PollStrategy;
use tuirealm::{Event, Key, KeyEvent, KeyModifiers, NoUserEvent};

use crate::SearchOptions;

mod application;
mod domain;
pub mod ui;

use self::ui::tuirealm_components::{
    app::Model,
    messages::{AppMessage, ComponentId},
    help_dialog::HelpDialog,
    result_detail::ResultDetail,
    result_list::ResultList,
    search_bar::SearchBar,
    session_viewer::SessionViewer,
};

pub struct InteractiveSearch {
    model: Model,
}

impl InteractiveSearch {
    pub fn new(options: SearchOptions) -> Result<Self> {
        let max_results = options.max_results.unwrap_or(100);
        let model = Model::new(options, max_results)?;
        
        Ok(Self { model })
    }

    pub fn run(&mut self, pattern: &str) -> Result<()> {
        // Set initial pattern if provided
        if !pattern.is_empty() {
            self.model.update(Some(AppMessage::QueryChanged(pattern.to_string())));
            self.model.update(Some(AppMessage::SearchRequested));
        }

        // Mount all components
        self.mount_components()?;

        // Main loop
        loop {
            // Process events
            let messages = self.model.tick(PollStrategy::Once)?;
            
            for msg in messages {
                self.model.update(Some(msg));
            }

            // Handle quit
            if self.model.quit {
                break;
            }

            // Render
            self.model.view()?;

            // Check for keyboard input
            if let Ok(true) = crossterm::event::poll(std::time::Duration::from_millis(50)) {
                if let Ok(crossterm::event::Event::Key(key_event)) = crossterm::event::read() {
                    let event = self.convert_crossterm_event(key_event);
                    
                    // Handle global shortcuts
                    if let Some(msg) = self.handle_global_shortcuts(&event) {
                        self.model.update(Some(msg));
                    } else {
                        // Forward to active component
                        match self.model.mode {
                            domain::models::Mode::Search => {
                                if let Ok(msg) = self.model.app.query(&ComponentId::SearchBar, event) {
                                    self.model.update(msg);
                                }
                            }
                            domain::models::Mode::ResultDetail => {
                                if let Ok(msg) = self.model.app.query(&ComponentId::ResultDetail, event) {
                                    self.model.update(msg);
                                }
                            }
                            domain::models::Mode::SessionViewer => {
                                if let Ok(msg) = self.model.app.query(&ComponentId::SessionViewer, event) {
                                    self.model.update(msg);
                                }
                            }
                            domain::models::Mode::Help => {
                                if let Ok(msg) = self.model.app.query(&ComponentId::HelpDialog, event) {
                                    self.model.update(msg);
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    fn mount_components(&mut self) -> Result<()> {
        use tuirealm::MockComponent;
        
        // Mount SearchBar
        let search_bar = SearchBar::new();
        self.model.app.mount(
            ComponentId::SearchBar,
            Box::new(search_bar),
            vec![]
        )?;

        // Mount ResultList
        let result_list = ResultList::new();
        self.model.app.mount(
            ComponentId::ResultList,
            Box::new(result_list),
            vec![]
        )?;

        // Mount ResultDetail
        let result_detail = ResultDetail::new();
        self.model.app.mount(
            ComponentId::ResultDetail,
            Box::new(result_detail),
            vec![]
        )?;

        // Mount SessionViewer
        let session_viewer = SessionViewer::new();
        self.model.app.mount(
            ComponentId::SessionViewer,
            Box::new(session_viewer),
            vec![]
        )?;

        // Mount HelpDialog
        let help_dialog = HelpDialog::new();
        self.model.app.mount(
            ComponentId::HelpDialog,
            Box::new(help_dialog),
            vec![]
        )?;

        // Set initial focus
        self.model.app.active(&ComponentId::SearchBar)?;

        Ok(())
    }

    fn convert_crossterm_event(&self, key_event: crossterm::event::KeyEvent) -> Event<NoUserEvent> {
        let code = match key_event.code {
            crossterm::event::KeyCode::Char(c) => Key::Char(c),
            crossterm::event::KeyCode::Enter => Key::Enter,
            crossterm::event::KeyCode::Esc => Key::Esc,
            crossterm::event::KeyCode::Tab => Key::Tab,
            crossterm::event::KeyCode::Backspace => Key::Backspace,
            crossterm::event::KeyCode::Delete => Key::Delete,
            crossterm::event::KeyCode::Up => Key::Up,
            crossterm::event::KeyCode::Down => Key::Down,
            crossterm::event::KeyCode::Left => Key::Left,
            crossterm::event::KeyCode::Right => Key::Right,
            crossterm::event::KeyCode::Home => Key::Home,
            crossterm::event::KeyCode::End => Key::End,
            crossterm::event::KeyCode::PageUp => Key::PageUp,
            crossterm::event::KeyCode::PageDown => Key::PageDown,
            crossterm::event::KeyCode::F(n) => Key::Function(n),
            _ => return Event::None,
        };

        let modifiers = KeyModifiers::from_bits_truncate(key_event.modifiers.bits());

        Event::Keyboard(KeyEvent { code, modifiers })
    }

    fn handle_global_shortcuts(&self, event: &Event<NoUserEvent>) -> Option<AppMessage> {
        match event {
            Event::Keyboard(KeyEvent { code: Key::Esc, modifiers: KeyModifiers::NONE }) => {
                match self.model.mode {
                    domain::models::Mode::Search => Some(AppMessage::Quit),
                    domain::models::Mode::ResultDetail => Some(AppMessage::ExitResultDetail),
                    domain::models::Mode::SessionViewer => Some(AppMessage::ExitSessionViewer),
                    domain::models::Mode::Help => Some(AppMessage::ExitHelp),
                }
            }
            Event::Keyboard(KeyEvent { code: Key::Char('?'), modifiers: KeyModifiers::NONE }) => {
                if self.model.mode == domain::models::Mode::Search {
                    Some(AppMessage::EnterHelp)
                } else {
                    None
                }
            }
            Event::Keyboard(KeyEvent { code: Key::Tab, modifiers: KeyModifiers::NONE }) => {
                if self.model.mode == domain::models::Mode::Search {
                    Some(AppMessage::ToggleRoleFilter)
                } else {
                    None
                }
            }
            Event::Keyboard(KeyEvent { code: Key::Char('s'), modifiers: KeyModifiers::NONE }) => {
                match self.model.mode {
                    domain::models::Mode::Search => {
                        if let Some(result) = self.model.search_state.results.get(self.model.search_state.selected_index) {
                            Some(AppMessage::EnterSessionViewer(result.session_id.clone()))
                        } else {
                            None
                        }
                    }
                    domain::models::Mode::ResultDetail => {
                        if let Some(result) = &self.model.ui_state.selected_result {
                            Some(AppMessage::EnterSessionViewer(result.session_id.clone()))
                        } else {
                            None
                        }
                    }
                    _ => None,
                }
            }
            _ => None,
        }
    }
}