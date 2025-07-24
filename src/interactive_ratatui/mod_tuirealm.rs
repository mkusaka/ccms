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
    pattern: String,
}

impl InteractiveSearch {
    pub fn new(options: SearchOptions) -> Result<Self> {
        let max_results = options.max_results.unwrap_or(100);
        let model = Model::new(options, max_results)?;
        
        Ok(Self { 
            model,
            pattern: String::new(),
        })
    }

    pub fn run(&mut self, pattern: &str) -> Result<()> {
        self.pattern = pattern.to_string();
        
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

            // Render based on current mode
            match self.model.mode {
                domain::models::Mode::Search => {
                    self.model.app.active(&ComponentId::SearchBar)?;
                    self.render_search_mode()?;
                }
                domain::models::Mode::ResultDetail => {
                    self.model.app.active(&ComponentId::ResultDetail)?;
                    self.render_result_detail_mode()?;
                }
                domain::models::Mode::SessionViewer => {
                    self.model.app.active(&ComponentId::SessionViewer)?;
                    self.render_session_viewer_mode()?;
                }
                domain::models::Mode::Help => {
                    self.model.app.active(&ComponentId::HelpDialog)?;
                    self.render_help_mode()?;
                }
            }

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

    fn render_search_mode(&mut self) -> Result<()> {
        if let Some(terminal) = &mut self.model.terminal {
            terminal.raw_mut().draw(|f| {
                use tuirealm::tui::layout::{Constraint, Direction, Layout};
                
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Length(3),    // Search bar
                        Constraint::Min(0),       // Results
                        Constraint::Length(1),    // Status
                    ])
                    .split(f.area());

                // Update components with data
                if let Ok(search_bar) = self.model.app.query_from_id(&ComponentId::SearchBar) {
                    if let Some(search_bar) = search_bar.downcast_ref::<SearchBar>() {
                        let mut search_bar_clone = search_bar.clone();
                        search_bar_clone.set_query(self.model.search_state.query.clone());
                        search_bar_clone.set_searching(self.model.search_state.is_searching);
                        search_bar_clone.set_role_filter(self.model.search_state.role_filter.clone());
                        if let Some(msg) = &self.model.ui_state.message {
                            search_bar_clone.set_message(Some(msg.clone()));
                        }
                        let _ = self.model.app.remount(ComponentId::SearchBar, Box::new(search_bar_clone), vec![]);
                    }
                }

                if let Ok(result_list) = self.model.app.query_from_id(&ComponentId::ResultList) {
                    if let Some(result_list) = result_list.downcast_ref::<ResultList>() {
                        let mut result_list_clone = result_list.clone();
                        result_list_clone.set_results(self.model.search_state.results.clone());
                        result_list_clone.set_selected_index(self.model.search_state.selected_index);
                        result_list_clone.set_truncation_enabled(self.model.ui_state.truncation_enabled);
                        let _ = self.model.app.remount(ComponentId::ResultList, Box::new(result_list_clone), vec![]);
                    }
                }

                // Render components
                let _ = self.model.app.view(&ComponentId::SearchBar, f, chunks[0]);
                let _ = self.model.app.view(&ComponentId::ResultList, f, chunks[1]);
            })?;
        }
        Ok(())
    }

    fn render_result_detail_mode(&mut self) -> Result<()> {
        if let Some(terminal) = &mut self.model.terminal {
            terminal.raw_mut().draw(|f| {
                if let Ok(result_detail) = self.model.app.query_from_id(&ComponentId::ResultDetail) {
                    if let Some(result_detail) = result_detail.downcast_ref::<ResultDetail>() {
                        let mut result_detail_clone = result_detail.clone();
                        if let Some(result) = &self.model.ui_state.selected_result {
                            result_detail_clone.set_result(Some(result.clone()));
                        }
                        if let Some(msg) = &self.model.ui_state.message {
                            result_detail_clone.set_message(Some(msg.clone()));
                        }
                        let _ = self.model.app.remount(ComponentId::ResultDetail, Box::new(result_detail_clone), vec![]);
                    }
                }
                let _ = self.model.app.view(&ComponentId::ResultDetail, f, f.area());
            })?;
        }
        Ok(())
    }

    fn render_session_viewer_mode(&mut self) -> Result<()> {
        if let Some(terminal) = &mut self.model.terminal {
            terminal.raw_mut().draw(|f| {
                if let Ok(session_viewer) = self.model.app.query_from_id(&ComponentId::SessionViewer) {
                    if let Some(session_viewer) = session_viewer.downcast_ref::<SessionViewer>() {
                        let mut session_viewer_clone = session_viewer.clone();
                        session_viewer_clone.set_messages(self.model.session_state.messages.clone());
                        session_viewer_clone.set_filtered_indices(self.model.session_state.filtered_indices.clone());
                        session_viewer_clone.set_selected_index(self.model.session_state.selected_index);
                        if let Some(order) = &self.model.session_state.order {
                            session_viewer_clone.set_order(Some(order.clone()));
                        }
                        let _ = self.model.app.remount(ComponentId::SessionViewer, Box::new(session_viewer_clone), vec![]);
                    }
                }
                let _ = self.model.app.view(&ComponentId::SessionViewer, f, f.area());
            })?;
        }
        Ok(())
    }

    fn render_help_mode(&mut self) -> Result<()> {
        if let Some(terminal) = &mut self.model.terminal {
            terminal.raw_mut().draw(|f| {
                let _ = self.model.app.view(&ComponentId::HelpDialog, f, f.area());
            })?;
        }
        Ok(())
    }
}