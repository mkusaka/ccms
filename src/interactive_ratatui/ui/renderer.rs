use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
};
use crate::interactive_ratatui::ui::app_state::{AppState, Mode};
use crate::interactive_ratatui::ui::components::{
    Component,
    search_bar::SearchBar,
    result_list::ResultList,
    result_detail::ResultDetail,
    session_viewer::SessionViewer,
    help_dialog::HelpDialog,
};

pub struct Renderer {
    search_bar: SearchBar,
    result_list: ResultList,
    result_detail: ResultDetail,
    session_viewer: SessionViewer,
    help_dialog: HelpDialog,
}

impl Renderer {
    pub fn new() -> Self {
        Self {
            search_bar: SearchBar::new(),
            result_list: ResultList::new(),
            result_detail: ResultDetail::new(),
            session_viewer: SessionViewer::new(),
            help_dialog: HelpDialog::new(),
        }
    }

    pub fn render(&mut self, f: &mut Frame, state: &AppState) {
        match state.mode {
            Mode::Search => self.render_search_mode(f, state),
            Mode::ResultDetail => self.render_detail_mode(f, state),
            Mode::SessionViewer => self.render_session_mode(f, state),
            Mode::Help => self.render_help_mode(f, state),
        }
    }

    fn render_search_mode(&mut self, f: &mut Frame, state: &AppState) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Search bar
                Constraint::Min(0),    // Results
            ])
            .split(f.area());

        // Update search bar state
        self.search_bar.set_query(state.search.query.clone());
        self.search_bar.set_searching(state.search.is_searching);
        self.search_bar.set_message(state.ui.message.clone());
        self.search_bar.set_role_filter(state.search.role_filter.clone());

        // Update result list state
        self.result_list.set_results(state.search.results.clone());
        self.result_list.set_selected_index(state.search.selected_index);

        // Render components
        self.search_bar.render(f, chunks[0]);
        self.result_list.render(f, chunks[1]);
    }

    fn render_detail_mode(&mut self, f: &mut Frame, state: &AppState) {
        if let Some(result) = &state.ui.selected_result {
            self.result_detail.set_result(result.clone());
            self.result_detail.render(f, f.area());
        }
    }

    fn render_session_mode(&mut self, f: &mut Frame, state: &AppState) {
        // Update session viewer state
        self.session_viewer.set_messages(state.session.messages.clone());
        self.session_viewer.set_filtered_indices(state.session.filtered_indices.clone());
        self.session_viewer.set_query(state.session.query.clone());
        self.session_viewer.set_order(state.session.order);

        self.session_viewer.render(f, f.area());
    }

    fn render_help_mode(&mut self, f: &mut Frame, state: &AppState) {
        // First render the search mode underneath
        self.render_search_mode(f, state);
        
        // Then render the help dialog on top
        self.help_dialog.render(f, f.area());
    }

    pub fn get_search_bar_mut(&mut self) -> &mut SearchBar {
        &mut self.search_bar
    }

    pub fn get_result_list_mut(&mut self) -> &mut ResultList {
        &mut self.result_list
    }

    pub fn get_result_detail_mut(&mut self) -> &mut ResultDetail {
        &mut self.result_detail
    }

    pub fn get_session_viewer_mut(&mut self) -> &mut SessionViewer {
        &mut self.session_viewer
    }

    pub fn get_help_dialog_mut(&mut self) -> &mut HelpDialog {
        &mut self.help_dialog
    }
}