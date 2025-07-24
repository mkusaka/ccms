use anyhow::Result;
use tuirealm::application::PollStrategy;
use tuirealm::Update;

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
        }

        Ok(())
    }

    fn mount_components(&mut self) -> Result<()> {
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


    fn render_search_mode(&mut self) -> Result<()> {
        if let Some(terminal) = &mut self.model.terminal {
            terminal.raw_mut().draw(|f| {
                use ratatui::layout::{Constraint, Direction, Layout};
                
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Length(3),    // Search bar
                        Constraint::Min(0),       // Results
                        Constraint::Length(1),    // Status
                    ])
                    .split(f.area());

                // TODO: Update components with data through proper tui-realm mechanisms
                // For now, components should manage their own state or we need to refactor
                // to use the attribute system properly

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
                // TODO: Update component with data through proper tui-realm mechanisms
                let _ = self.model.app.view(&ComponentId::ResultDetail, f, f.area());
            })?;
        }
        Ok(())
    }

    fn render_session_viewer_mode(&mut self) -> Result<()> {
        if let Some(terminal) = &mut self.model.terminal {
            terminal.raw_mut().draw(|f| {
                // TODO: Update component with data through proper tui-realm mechanisms
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