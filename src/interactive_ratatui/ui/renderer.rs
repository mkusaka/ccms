use crate::interactive_ratatui::constants::*;
use crate::interactive_ratatui::ui::app_state::{AppState, Mode};
use crate::interactive_ratatui::ui::components::{
    Component, help_dialog::HelpDialog, is_exit_prompt, message_detail::MessageDetail,
    message_preview::MessagePreview, result_list::ResultList, search_bar::SearchBar,
    session_viewer_unified::SessionViewerUnified,
};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::Paragraph,
};

#[derive(Default)]
pub struct Renderer {
    search_bar: SearchBar,
    result_list: ResultList,
    message_detail: MessageDetail,
    message_preview: MessagePreview,
    session_viewer: SessionViewerUnified,
    help_dialog: HelpDialog,
}

impl Renderer {
    pub fn new() -> Self {
        Self {
            search_bar: SearchBar::new(),
            result_list: ResultList::new(),
            message_detail: MessageDetail::new(),
            message_preview: MessagePreview::new(),
            session_viewer: SessionViewerUnified::new(),
            help_dialog: HelpDialog::new(),
        }
    }

    pub fn render(&mut self, f: &mut Frame, state: &AppState) {
        let _ = crate::interactive_ratatui::debug::write_debug_log(
            &format!("Renderer::render called with mode: {:?}", state.mode)
        );
        match state.mode {
            Mode::Search => self.render_search_mode(f, state),
            Mode::MessageDetail => self.render_detail_mode(f, state),
            Mode::SessionViewer => self.render_session_mode(f, state),
            Mode::Help => self.render_help_mode(f, state),
        }
    }

    fn render_search_mode(&mut self, f: &mut Frame, state: &AppState) {
        // Check if we need to display exit prompt at bottom
        let show_exit_prompt = is_exit_prompt(&state.ui.message);

        let chunks = if show_exit_prompt {
            Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(SEARCH_BAR_HEIGHT),  // Search bar
                    Constraint::Min(0),                     // Results
                    Constraint::Length(EXIT_PROMPT_HEIGHT), // Exit prompt
                ])
                .split(f.area())
        } else {
            Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(SEARCH_BAR_HEIGHT), // Search bar
                    Constraint::Min(0),                    // Results
                ])
                .split(f.area())
        };

        // Update search bar state
        self.search_bar.set_query(state.search.query.clone());
        self.search_bar.set_searching(state.search.is_searching);
        // Don't pass exit prompt to search bar
        if show_exit_prompt {
            self.search_bar.set_message(None);
        } else {
            self.search_bar.set_message(state.ui.message.clone());
        }
        self.search_bar
            .set_role_filter(state.search.role_filter.clone());
        self.search_bar.set_search_order(state.search.order);

        // Render search bar
        self.search_bar.render(f, chunks[0]);

        // Split the content area if preview is enabled
        if state.search.preview_enabled && !state.search.results.is_empty() {
            // Split content area into list and preview
            let content_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Percentage(40), // Results list
                    Constraint::Percentage(60), // Preview
                ])
                .split(chunks[1]);

            // Update result list state
            self.result_list.set_results(state.search.results.clone());
            self.result_list
                .set_selected_index(state.search.selected_index);
            self.result_list
                .set_truncation_enabled(state.ui.truncation_enabled);
            self.result_list.set_preview_enabled(true);

            // Update preview state
            let selected_result = state
                .search
                .results
                .get(state.search.selected_index)
                .cloned();
            self.message_preview.set_result(selected_result);

            // Render both components
            self.result_list.render(f, content_chunks[0]);
            self.message_preview.render(f, content_chunks[1]);
        } else {
            // No preview - use full width for results
            self.result_list.set_results(state.search.results.clone());
            self.result_list
                .set_selected_index(state.search.selected_index);
            self.result_list
                .set_truncation_enabled(state.ui.truncation_enabled);
            self.result_list.set_preview_enabled(false);
            self.result_list.render(f, chunks[1]);
        }

        // Render exit prompt at bottom if needed
        if show_exit_prompt {
            let exit_prompt = Paragraph::new("Press Ctrl+C again to exit")
                .style(
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                )
                .alignment(ratatui::layout::Alignment::Center);
            f.render_widget(exit_prompt, chunks[2]);
        }
    }

    fn render_detail_mode(&mut self, f: &mut Frame, state: &AppState) {
        let _ = crate::interactive_ratatui::debug::write_debug_log(
            &format!("Renderer::render_detail_mode called, selected_result: {}", 
                state.ui.selected_result.is_some())
        );
        if let Some(result) = &state.ui.selected_result {
            let _ = crate::interactive_ratatui::debug::write_debug_log(
                &format!("Renderer::render_detail_mode: rendering result with uuid: {}", result.uuid)
            );
            self.message_detail.set_result(result.clone());
            self.message_detail.set_message(state.ui.message.clone());
            self.message_detail.render(f, f.area());
        } else {
            let _ = crate::interactive_ratatui::debug::write_debug_log(
                "Renderer::render_detail_mode: No selected_result to render!"
            );
        }
    }

    fn render_session_mode(&mut self, f: &mut Frame, state: &AppState) {
        // Update session viewer state with search results
        self.session_viewer
            .set_results(state.session.search_results.clone());
        self.session_viewer.set_query(state.session.query.clone());
        self.session_viewer.set_order(state.session.order);
        self.session_viewer
            .set_file_path(state.session.file_path.clone());
        self.session_viewer
            .set_session_id(state.session.session_id.clone());
        self.session_viewer.set_message(state.ui.message.clone());
        self.session_viewer
            .set_role_filter(state.session.role_filter.clone());
        let _ = crate::interactive_ratatui::debug::write_debug_log(
            &format!("Renderer::render_session_mode: setting preview_enabled = {}", state.session.preview_enabled)
        );
        self.session_viewer
            .set_preview_enabled(state.session.preview_enabled);
        // Restore the selected index
        self.session_viewer
            .set_selected_index(state.session.selected_index);
        self.session_viewer
            .set_truncation_enabled(state.ui.truncation_enabled);

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

    pub fn get_message_detail_mut(&mut self) -> &mut MessageDetail {
        &mut self.message_detail
    }

    pub fn get_session_viewer_mut(&mut self) -> &mut SessionViewerUnified {
        &mut self.session_viewer
    }

    pub fn get_help_dialog_mut(&mut self) -> &mut HelpDialog {
        &mut self.help_dialog
    }
}
