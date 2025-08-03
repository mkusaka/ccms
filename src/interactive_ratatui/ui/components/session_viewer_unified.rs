use crate::interactive_ratatui::domain::models::SessionOrder;
use crate::interactive_ratatui::ui::components::{
    Component, is_exit_prompt,
    result_list::ResultList,
    text_input::TextInput,
    view_layout::{ColorScheme, ViewLayout},
    message_preview::MessagePreview,
};
use crate::interactive_ratatui::ui::events::{CopyContent, Message};
use crate::query::condition::SearchResult;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::Line,
    widgets::{Block, Borders, Paragraph},
};

pub struct SessionViewerUnified {
    result_list: ResultList,
    message_preview: MessagePreview,
    text_input: TextInput,
    order: SessionOrder,
    is_searching: bool,
    file_path: Option<String>,
    cwd: Option<String>,
    session_id: Option<String>,
    message: Option<String>,
    role_filter: Option<String>,
    preview_enabled: bool,
}

impl Default for SessionViewerUnified {
    fn default() -> Self {
        Self::new()
    }
}

impl SessionViewerUnified {
    pub fn new() -> Self {
        Self {
            result_list: ResultList::new().with_status_bar(false),
            message_preview: MessagePreview::new(),
            text_input: TextInput::new(),
            order: SessionOrder::Ascending,
            is_searching: false,
            file_path: None,
            cwd: None,
            session_id: None,
            message: None,
            role_filter: None,
            preview_enabled: false,
        }
    }

    pub fn set_results(&mut self, results: Vec<SearchResult>) {
        // Extract cwd from the first result if not yet set
        if self.cwd.is_none() && !results.is_empty() {
            self.cwd = Some(results[0].cwd.clone());
        }
        self.result_list.set_results(results);
    }

    pub fn set_query(&mut self, query: String) {
        self.text_input.set_text(query);
    }

    pub fn set_order(&mut self, order: SessionOrder) {
        self.order = order;
    }

    pub fn set_file_path(&mut self, file_path: Option<String>) {
        self.file_path = file_path;
    }

    pub fn set_session_id(&mut self, session_id: Option<String>) {
        self.session_id = session_id;
    }

    pub fn set_selected_index(&mut self, index: usize) {
        self.result_list.set_selected_index(index);
    }

    pub fn get_selected_index(&self) -> usize {
        self.result_list.get_selected_index()
    }

    pub fn get_scroll_offset(&self) -> usize {
        self.result_list.get_scroll_offset()
    }

    pub fn set_truncation_enabled(&mut self, enabled: bool) {
        self.result_list.set_truncation_enabled(enabled);
    }

    pub fn set_message(&mut self, message: Option<String>) {
        self.message = message;
    }

    pub fn set_role_filter(&mut self, role_filter: Option<String>) {
        self.role_filter = role_filter;
    }

    pub fn set_preview_enabled(&mut self, enabled: bool) {
        let _ = crate::interactive_ratatui::debug::write_debug_log(
            &format!("SessionViewerUnified::set_preview_enabled: {} -> {}", self.preview_enabled, enabled)
        );
        self.preview_enabled = enabled;
        self.result_list.set_preview_enabled(enabled);
    }

    pub fn start_search(&mut self) {
        self.is_searching = true;
        self.text_input.set_text(String::new());
    }

    pub fn stop_search(&mut self) {
        self.is_searching = false;
    }
    
    #[cfg(test)]
    pub fn is_preview_enabled(&self) -> bool {
        self.preview_enabled
    }
    
    pub fn get_result_list(&self) -> &ResultList {
        &self.result_list
    }

    fn format_order(order: &SessionOrder) -> &'static str {
        match order {
            SessionOrder::Ascending => "Asc",
            SessionOrder::Descending => "Desc",
        }
    }

    fn render_content(&mut self, f: &mut Frame, area: Rect) {
        let _ = crate::interactive_ratatui::debug::write_debug_log(
            &format!("SessionViewerUnified::render_content: area = {:?}", area)
        );
        
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Search bar or info bar
                Constraint::Min(0),    // Results
            ])
            .split(area);

        // Render search bar
        if self.is_searching {
            let search_text = self.text_input.render_cursor_spans();
            let order_text = Self::format_order(&self.order);

            // Build status text with order and optional role filter
            let status_text = if let Some(role) = &self.role_filter {
                format!("Order: {order_text}, Role: {role}")
            } else {
                format!("Order: {order_text}")
            };

            let search_bar = Paragraph::new(Line::from(search_text)).block(
                Block::default()
                    .title(format!("Search in session ({status_text}) | Tab: Role Filter | Ctrl+O: Sort | Esc to cancel"))
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(ColorScheme::SECONDARY)),
            );
            f.render_widget(search_bar, chunks[0]);
        } else {
            let order_text = Self::format_order(&self.order);
            let order_part = format!(" | Order: {order_text}");

            let role_part = if let Some(role) = &self.role_filter {
                format!(" | Role: {role}")
            } else {
                String::new()
            };

            let total_count = self.result_list.items_count();

            let info_text = if total_count == 0 {
                format!(
                    "No messages{}{} | Press '/' to search",
                    order_part,
                    role_part
                )
            } else {
                format!(
                    "Total: {} messages{}{} | Press '/' to search",
                    total_count,
                    order_part,
                    role_part
                )
            };
            let info_bar = Paragraph::new(info_text).block(Block::default().borders(Borders::ALL));
            f.render_widget(info_bar, chunks[0]);
        }

        // Split the content area if preview is enabled
        let _ = crate::interactive_ratatui::debug::write_debug_log(
            &format!("SessionViewerUnified::render_content: preview_enabled = {}, has_selected_result = {}", 
                self.preview_enabled, 
                self.result_list.selected_result().is_some())
        );
        if self.preview_enabled && self.result_list.selected_result().is_some() {
            // Split content area into list and preview
            let content_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Percentage(40), // Results list
                    Constraint::Percentage(60), // Preview
                ])
                .split(chunks[1]);

            // Update preview state
            let selected_result = self.result_list.selected_result().cloned();
            self.message_preview.set_result(selected_result);

            // Render both components
            self.result_list.render(f, content_chunks[0]);
            self.message_preview.render(f, content_chunks[1]);
        } else {
            // No preview - use full width for results
            self.result_list.render(f, chunks[1]);
        }
    }
}

impl Component for SessionViewerUnified {
    fn render(&mut self, f: &mut Frame, area: Rect) {
        let mut subtitle_parts = Vec::new();

        if let Some(cwd) = &self.cwd {
            subtitle_parts.push(format!("CWD: {cwd}"));
        }

        if let Some(session) = &self.session_id {
            subtitle_parts.push(format!("Session: {session}"));
        }

        if let Some(file) = &self.file_path {
            subtitle_parts.push(format!("File: {file}"));
        }

        let subtitle = subtitle_parts.join("\n");

        // Check if message is exit prompt
        let is_exit = is_exit_prompt(&self.message);
        let non_exit_message = if is_exit { None } else { self.message.clone() };

        // Layout with message area but WITHOUT status bar (ViewLayout will handle it)
        let chunks = if is_exit {
            Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Min(0),    // Main content
                    Constraint::Length(1), // Exit prompt at bottom
                ])
                .split(area)
        } else if non_exit_message.is_some() {
            Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Min(0),    // Main content
                    Constraint::Length(1), // Message
                ])
                .split(area)
        } else {
            Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(0)]) // Full area
                .split(area)
        };

        // Render main content with ViewLayout
        let layout = ViewLayout::new("Session Viewer".to_string())
            .with_subtitle(subtitle)
            .with_status_bar(true) // Let ViewLayout handle the status bar
            .with_status_text("↑/↓ Ctrl+P/N Ctrl+U/D: Navigate | Tab: Filter | Enter: Detail | Ctrl+O: Sort | Ctrl+T: Preview | c/C: Copy text/JSON | i/f/p: Copy IDs/paths | /: Search | Alt+←/→: History | Esc: Back".to_string());

        layout.render(f, chunks[0], |f, content_area| {
            self.render_content(f, content_area);
        });

        // Render non-exit message if present
        if let Some(ref msg) = non_exit_message {
            let style = if msg.starts_with('✓') {
                Style::default()
                    .fg(ColorScheme::SUCCESS)
                    .add_modifier(Modifier::BOLD)
            } else if msg.starts_with('⚠') {
                Style::default().fg(ColorScheme::WARNING)
            } else {
                Style::default().add_modifier(Modifier::BOLD)
            };

            let message_widget = Paragraph::new(msg.clone())
                .style(style)
                .alignment(ratatui::layout::Alignment::Center);
            if chunks.len() > 1 {
                f.render_widget(message_widget, chunks[1]);
            }
        }

        // Render exit prompt at the very bottom if needed
        if is_exit && chunks.len() > 1 {
            let exit_prompt = Paragraph::new("Press Ctrl+C again to exit")
                .style(
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                )
                .alignment(ratatui::layout::Alignment::Center);
            f.render_widget(exit_prompt, chunks[chunks.len() - 1]);
        }
    }

    fn handle_key(&mut self, key: KeyEvent) -> Option<Message> {
        if self.is_searching {
            match key.code {
                KeyCode::Esc => {
                    self.is_searching = false;
                    self.text_input.set_text(String::new());
                    Some(Message::SessionQueryChanged(String::new()))
                }
                KeyCode::Enter => {
                    // Navigate to result detail for selected message (keep search mode active)
                    if let Some(result) = self.result_list.selected_result() {
                        Some(Message::EnterMessageDetailFromSession(
                            result.raw_json.clone().unwrap_or_default(),
                            self.file_path.clone().unwrap_or_default(),
                            self.session_id.clone(),
                        ))
                    } else {
                        None
                    }
                }
                KeyCode::Up => {
                    self.result_list.handle_key(key);
                    Some(Message::SessionNavigated(
                        self.result_list.get_selected_index(),
                        self.result_list.get_scroll_offset(),
                    ))
                }
                KeyCode::Down => {
                    self.result_list.handle_key(key);
                    Some(Message::SessionNavigated(
                        self.result_list.get_selected_index(),
                        self.result_list.get_scroll_offset(),
                    ))
                }
                KeyCode::Char('p') if key.modifiers == KeyModifiers::CONTROL => {
                    self.result_list.handle_key(key);
                    Some(Message::SessionNavigated(
                        self.result_list.get_selected_index(),
                        self.result_list.get_scroll_offset(),
                    ))
                }
                KeyCode::Char('n') if key.modifiers == KeyModifiers::CONTROL => {
                    self.result_list.handle_key(key);
                    Some(Message::SessionNavigated(
                        self.result_list.get_selected_index(),
                        self.result_list.get_scroll_offset(),
                    ))
                }
                KeyCode::Char('u') if key.modifiers == KeyModifiers::CONTROL => {
                    self.result_list.handle_key(key);
                    Some(Message::SessionNavigated(
                        self.result_list.get_selected_index(),
                        self.result_list.get_scroll_offset(),
                    ))
                }
                KeyCode::Char('d') if key.modifiers == KeyModifiers::CONTROL => {
                    self.result_list.handle_key(key);
                    Some(Message::SessionNavigated(
                        self.result_list.get_selected_index(),
                        self.result_list.get_scroll_offset(),
                    ))
                }
                KeyCode::PageUp | KeyCode::PageDown | KeyCode::Home | KeyCode::End => {
                    self.result_list.handle_key(key);
                    Some(Message::SessionNavigated(
                        self.result_list.get_selected_index(),
                        self.result_list.get_scroll_offset(),
                    ))
                }
                KeyCode::Tab if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                    Some(Message::ToggleSessionRoleFilter)
                }
                KeyCode::Char('o') if key.modifiers == KeyModifiers::CONTROL => {
                    Some(Message::ToggleSessionOrder)
                }
                KeyCode::Char('t') if key.modifiers == KeyModifiers::CONTROL => {
                    let _ = crate::interactive_ratatui::debug::write_debug_log(
                        "SessionViewerUnified (searching): Ctrl+T pressed, sending ToggleSessionPreview"
                    );
                    Some(Message::ToggleSessionPreview)
                }
                // Copy shortcuts during search
                KeyCode::Char('c') if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                    self.result_list.selected_result().map(|result| {
                        Message::CopyToClipboard(CopyContent::MessageContent(result.text.clone()))
                    })
                }
                KeyCode::Char('C') if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                    self.result_list.selected_result().map(|result| {
                        Message::CopyToClipboard(CopyContent::JsonData(
                            result.raw_json.clone().unwrap_or_default()
                        ))
                    })
                }
                KeyCode::Char('i') if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                    self.session_id
                        .clone()
                        .map(|id| Message::CopyToClipboard(CopyContent::SessionId(id)))
                }
                KeyCode::Char('p') if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                    self.cwd
                        .clone()
                        .map(|path| Message::CopyToClipboard(CopyContent::ProjectPath(path)))
                }
                KeyCode::Char('f') if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                    self.file_path
                        .clone()
                        .map(|path| Message::CopyToClipboard(CopyContent::FilePath(path)))
                }
                _ => {
                    let changed = self.text_input.handle_key(key);
                    if changed {
                        Some(Message::SessionQueryChanged(
                            self.text_input.text().to_string(),
                        ))
                    } else {
                        None
                    }
                }
            }
        } else {
            match key.code {
                KeyCode::Up => {
                    self.result_list.handle_key(key);
                    Some(Message::SessionNavigated(
                        self.result_list.get_selected_index(),
                        self.result_list.get_scroll_offset(),
                    ))
                }
                KeyCode::Down => {
                    self.result_list.handle_key(key);
                    Some(Message::SessionNavigated(
                        self.result_list.get_selected_index(),
                        self.result_list.get_scroll_offset(),
                    ))
                }
                KeyCode::Enter => {
                    // Navigate to result detail for selected message
                    if let Some(result) = self.result_list.selected_result() {
                        Some(Message::EnterMessageDetailFromSession(
                            result.raw_json.clone().unwrap_or_default(),
                            self.file_path.clone().unwrap_or_default(),
                            self.session_id.clone(),
                        ))
                    } else {
                        None
                    }
                }
                KeyCode::Char('p') if key.modifiers == KeyModifiers::CONTROL => {
                    self.result_list.handle_key(key);
                    Some(Message::SessionNavigated(
                        self.result_list.get_selected_index(),
                        self.result_list.get_scroll_offset(),
                    ))
                }
                KeyCode::Char('n') if key.modifiers == KeyModifiers::CONTROL => {
                    self.result_list.handle_key(key);
                    Some(Message::SessionNavigated(
                        self.result_list.get_selected_index(),
                        self.result_list.get_scroll_offset(),
                    ))
                }
                KeyCode::Char('u') if key.modifiers == KeyModifiers::CONTROL => {
                    self.result_list.handle_key(key);
                    Some(Message::SessionNavigated(
                        self.result_list.get_selected_index(),
                        self.result_list.get_scroll_offset(),
                    ))
                }
                KeyCode::Char('d') if key.modifiers == KeyModifiers::CONTROL => {
                    self.result_list.handle_key(key);
                    Some(Message::SessionNavigated(
                        self.result_list.get_selected_index(),
                        self.result_list.get_scroll_offset(),
                    ))
                }
                KeyCode::PageUp | KeyCode::PageDown | KeyCode::Home | KeyCode::End => {
                    self.result_list.handle_key(key);
                    Some(Message::SessionNavigated(
                        self.result_list.get_selected_index(),
                        self.result_list.get_scroll_offset(),
                    ))
                }
                KeyCode::Tab if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                    Some(Message::ToggleSessionRoleFilter)
                }
                KeyCode::Char('/') => {
                    self.is_searching = true;
                    None
                }
                KeyCode::Char('o') if key.modifiers == KeyModifiers::CONTROL => {
                    Some(Message::ToggleSessionOrder)
                }
                KeyCode::Char('t') if key.modifiers == KeyModifiers::CONTROL => {
                    let _ = crate::interactive_ratatui::debug::write_debug_log(
                        "SessionViewerUnified: Ctrl+T pressed, sending ToggleSessionPreview"
                    );
                    Some(Message::ToggleSessionPreview)
                }
                // Unified copy operations
                KeyCode::Char('c') => self.result_list.selected_result().map(|result| {
                    Message::CopyToClipboard(CopyContent::MessageContent(result.text.clone()))
                }),
                KeyCode::Char('C') => self.result_list.selected_result().map(|result| {
                    Message::CopyToClipboard(CopyContent::JsonData(
                        result.raw_json.clone().unwrap_or_default()
                    ))
                }),
                KeyCode::Char('i') => self
                    .session_id
                    .clone()
                    .map(|id| Message::CopyToClipboard(CopyContent::SessionId(id))),
                KeyCode::Char('p') => self
                    .cwd
                    .clone()
                    .map(|path| Message::CopyToClipboard(CopyContent::ProjectPath(path))),
                KeyCode::Char('f') => self
                    .file_path
                    .clone()
                    .map(|path| Message::CopyToClipboard(CopyContent::FilePath(path))),
                KeyCode::Esc => Some(Message::ExitToSearch),
                _ => None,
            }
        }
    }
}