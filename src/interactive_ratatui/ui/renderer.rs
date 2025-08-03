use crate::interactive_ratatui::constants::*;
use crate::interactive_ratatui::domain::models::SearchTab;
use crate::interactive_ratatui::ui::app_state::{AppState, Mode};
use crate::interactive_ratatui::ui::components::{
    Component, help_dialog::HelpDialog, is_exit_prompt, message_detail::MessageDetail,
    message_preview::MessagePreview, result_list::ResultList, search_bar::SearchBar,
    session_list::SessionList, session_preview::SessionPreview,
    session_viewer_unified::SessionViewerUnified, tab_bar::TabBar,
};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
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
    session_list: SessionList,
    session_preview: SessionPreview,
    tab_bar: TabBar,
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
            session_list: SessionList::new(),
            session_preview: SessionPreview::new(),
            tab_bar: TabBar::new(),
            help_dialog: HelpDialog::new(),
        }
    }

    pub fn render(&mut self, f: &mut Frame, state: &AppState) {
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
                    Constraint::Length(3),                  // Tab bar (with borders)
                    Constraint::Length(SEARCH_BAR_HEIGHT),  // Search bar
                    Constraint::Min(0),                     // Results
                    Constraint::Length(EXIT_PROMPT_HEIGHT), // Exit prompt
                ])
                .split(f.area())
        } else {
            Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3),                 // Tab bar (with borders)
                    Constraint::Length(SEARCH_BAR_HEIGHT), // Search bar
                    Constraint::Min(0),                    // Results
                ])
                .split(f.area())
        };

        // Update and render tab bar
        self.tab_bar.set_current_tab(state.search.current_tab);
        self.tab_bar.render(f, chunks[0]);

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

        // Render search bar (only for Search tab)
        if state.search.current_tab == SearchTab::Search {
            self.search_bar.render(f, chunks[1]);
        }

        // Render content based on current tab
        match state.search.current_tab {
            SearchTab::Search => {
                // For Search tab, content is in chunks[2]
                let content_area = chunks[2];

                if state.search.preview_enabled && !state.search.results.is_empty() {
                    // Split content area into list and preview
                    let content_chunks = Layout::default()
                        .direction(Direction::Horizontal)
                        .constraints([
                            Constraint::Percentage(40), // Results list
                            Constraint::Percentage(60), // Preview
                        ])
                        .split(content_area);

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
                    self.result_list.render(f, content_area);
                }
            }
            SearchTab::SessionList => {
                // Update session list state
                self.session_list
                    .set_sessions(state.session_list.filtered_sessions.clone());
                self.session_list
                    .set_filtered_sessions(state.session_list.filtered_sessions.clone());
                self.session_list
                    .set_query(state.session_list.query.clone());
                self.session_list
                    .set_selected_index(state.session_list.selected_index);
                self.session_list
                    .set_is_loading(state.session_list.is_loading);
                self.session_list
                    .set_is_searching(state.session_list.is_searching);
                self.session_list
                    .set_preview_enabled(state.session_list.preview_enabled);
                self.session_list
                    .set_query(state.session_list.query.clone());
                self.session_list
                    .set_is_searching(state.session_list.is_searching);

                // For SessionList tab, combine the search bar area and content area
                // This uses chunks[1] (search bar area) and chunks[2] (content area)
                let combined_area = Rect {
                    x: chunks[1].x,
                    y: chunks[1].y,
                    width: chunks[1].width,
                    height: chunks[1].height + chunks[2].height,
                };

                if state.session_list.preview_enabled
                    && !state.session_list.filtered_sessions.is_empty()
                {
                    // Split the combined area into list and preview
                    let preview_chunks = Layout::default()
                        .direction(Direction::Horizontal)
                        .constraints([
                            Constraint::Percentage(50), // Session list
                            Constraint::Percentage(50), // Preview
                        ])
                        .split(combined_area);

                    // Update preview state
                    self.session_preview
                        .set_session(self.session_list.get_selected_session().cloned());
                    self.session_preview
                        .set_query(state.session_list.query.clone());

                    // Render both components
                    self.session_list.render(f, preview_chunks[0]);
                    self.session_preview.render(f, preview_chunks[1]);
                } else {
                    // No preview - use full width for session list
                    self.session_list.render(f, combined_area);
                }
            }
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
            f.render_widget(exit_prompt, chunks[3]);
        }
    }

    fn render_detail_mode(&mut self, f: &mut Frame, state: &AppState) {
        if let Some(result) = &state.ui.selected_result {
            self.message_detail.set_result(result.clone());
            self.message_detail.set_message(state.ui.message.clone());
            self.message_detail.render(f, f.area());
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

    pub fn get_session_list_mut(&mut self) -> &mut SessionList {
        &mut self.session_list
    }

    pub fn get_tab_bar_mut(&mut self) -> &mut TabBar {
        &mut self.tab_bar
    }
}
