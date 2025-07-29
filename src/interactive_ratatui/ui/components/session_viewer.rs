use crate::interactive_ratatui::domain::models::SessionOrder;
use crate::interactive_ratatui::domain::session_list_item::SessionListItem;
use crate::interactive_ratatui::ui::components::{
    Component, is_exit_prompt,
    list_viewer::ListViewer,
    text_input::TextInput,
    view_layout::{ColorScheme, ViewLayout},
};
use crate::interactive_ratatui::ui::events::{CopyContent, Message};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Text},
    widgets::{Block, Borders, Paragraph, Wrap},
};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::Path;

pub struct SessionViewer {
    #[cfg(test)]
    pub list_viewer: ListViewer<SessionListItem>,
    #[cfg(not(test))]
    list_viewer: ListViewer<SessionListItem>,
    raw_messages: Vec<String>,
    text_input: TextInput,
    order: SessionOrder,
    is_searching: bool,
    file_path: Option<String>,
    project_path: Option<String>,
    session_id: Option<String>,
    messages_hash: u64,
    message: Option<String>,
    role_filter: Option<String>,
}

impl Default for SessionViewer {
    fn default() -> Self {
        Self::new()
    }
}

impl SessionViewer {
    pub fn new() -> Self {
        Self {
            list_viewer: ListViewer::new(
                "Session Messages".to_string(),
                "No messages in session".to_string(),
            ),
            raw_messages: Vec::new(),
            text_input: TextInput::new(),
            order: SessionOrder::Ascending,
            is_searching: false,
            file_path: None,
            project_path: None,
            session_id: None,
            messages_hash: 0,
            message: None,
            role_filter: None,
        }
    }

    pub fn set_messages(&mut self, messages: Vec<String>) {
        // Calculate hash of new messages to check if they changed
        let mut hasher = DefaultHasher::new();
        messages.hash(&mut hasher);
        let new_hash = hasher.finish();

        // Only update if messages have changed
        if new_hash != self.messages_hash {
            self.messages_hash = new_hash;
            self.raw_messages = messages;

            // Convert raw messages to SessionListItems
            let items: Vec<SessionListItem> = self
                .raw_messages
                .iter()
                .filter_map(|line| SessionListItem::from_json_line(line))
                .collect();

            self.list_viewer.set_items(items);
        }
    }

    pub fn set_filtered_indices(&mut self, indices: Vec<usize>) {
        self.list_viewer.set_filtered_indices(indices);
    }

    pub fn set_query(&mut self, query: String) {
        self.text_input.set_text(query.clone());
        self.list_viewer.set_query(query);
    }

    pub fn set_order(&mut self, order: SessionOrder) {
        self.order = order;
    }

    pub fn set_file_path(&mut self, file_path: Option<String>) {
        self.file_path = file_path.clone();
        // Extract project path from file path
        self.project_path = file_path.and_then(|path| Self::extract_project_path(&path));
    }

    pub fn set_session_id(&mut self, session_id: Option<String>) {
        self.session_id = session_id;
    }

    pub fn set_selected_index(&mut self, index: usize) {
        // Use set_filtered_position since we're dealing with filtered indices
        self.list_viewer.set_filtered_position(index);
    }

    pub fn set_scroll_offset(&mut self, offset: usize) {
        self.list_viewer.set_scroll_offset(offset);
    }

    pub fn set_truncation_enabled(&mut self, enabled: bool) {
        self.list_viewer.set_truncation_enabled(enabled);
    }

    pub fn set_message(&mut self, message: Option<String>) {
        self.message = message;
    }

    pub fn set_role_filter(&mut self, role_filter: Option<String>) {
        self.role_filter = role_filter;
    }

    pub fn get_selected_index(&self) -> usize {
        self.list_viewer.selected_index
    }

    pub fn get_scroll_offset(&self) -> usize {
        self.list_viewer.scroll_offset
    }

    pub fn start_search(&mut self) {
        self.is_searching = true;
        self.text_input.set_text(String::new());
    }

    pub fn stop_search(&mut self) {
        self.is_searching = false;
    }

    #[cfg(test)]
    pub fn set_cursor_position(&mut self, pos: usize) {
        self.text_input.set_cursor_position(pos);
    }

    #[cfg(test)]
    pub fn cursor_position(&self) -> usize {
        self.text_input.cursor_position()
    }

    #[cfg(test)]
    pub fn query(&self) -> &str {
        self.text_input.text()
    }

    fn format_order(order: &SessionOrder) -> &'static str {
        match order {
            SessionOrder::Ascending => "Asc",
            SessionOrder::Descending => "Desc",
        }
    }

    fn extract_project_path(file_path: &str) -> Option<String> {
        // Extract project path from file path
        // Format: ~/.claude/projects/{encoded-project-path}/{session-id}.jsonl
        let path = Path::new(file_path);

        // Check if this is a Claude project path
        if !file_path.contains(".claude/projects/") {
            return None;
        }

        if let Some(parent) = path.parent() {
            if let Some(project_name) = parent.file_name() {
                if let Some(project_str) = project_name.to_str() {
                    // Decode the project path (replace hyphens with slashes)
                    let decoded = project_str.replace('-', "/");
                    if !decoded.is_empty() && decoded != "/" {
                        return Some(decoded);
                    }
                }
            }
        }
        None
    }

    fn render_content(&mut self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Search bar or info bar
                Constraint::Min(0),    // Messages
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
                    .title(format!("Search in session ({status_text}) | Tab: Role Filter | Ctrl+O: Sort | Esc to cancel | ↑/↓ or Ctrl+P/N to scroll"))
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

            let info_text = format!(
                "Messages: {} (filtered: {}){}{} | Press '/' to search",
                self.list_viewer.items_count(),
                self.list_viewer.filtered_count(),
                order_part,
                role_part
            );
            let info_bar = Paragraph::new(info_text).block(Block::default().borders(Borders::ALL));
            f.render_widget(info_bar, chunks[0]);
        }

        // Render message list using ListViewer
        self.list_viewer.render(f, chunks[1]);
    }
}

impl Component for SessionViewer {
    fn render(&mut self, f: &mut Frame, area: Rect) {
        let mut subtitle_parts = Vec::new();

        if let Some(project) = &self.project_path {
            subtitle_parts.push(format!("Project: {project}"));
        }

        if let Some(session) = &self.session_id {
            subtitle_parts.push(format!("Session: {session}"));
        }

        if let Some(file) = &self.file_path {
            subtitle_parts.push(format!("File: {file}"));
        }

        let subtitle = subtitle_parts.join("\n");

        // Calculate status bar height based on terminal width using Ratatui's line wrapping
        let status_text = "↑/↓ Ctrl+P/N Ctrl+U/D: Navigate | Tab: Filter | Enter: Detail | Ctrl+O: Sort | c/C: Copy text/JSON | i/f/p: Copy IDs/paths | /: Search | Alt+←/→: History | Esc: Back";
        let status_bar_height = {
            // Create a temporary paragraph to calculate actual line count
            let paragraph = Paragraph::new(status_text).wrap(Wrap { trim: true });

            // Calculate the actual number of lines with proper text wrapping
            let lines_needed = paragraph.line_count(area.width) as u16;

            // Ensure minimum of 3 lines, max of 8 lines
            lines_needed.clamp(3, 8)
        };

        // Check if message is exit prompt
        let is_exit = is_exit_prompt(&self.message);
        let non_exit_message = if is_exit { None } else { self.message.clone() };

        // Layout with message area
        let chunks = if is_exit {
            Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Min(0),                    // Main content
                    Constraint::Length(status_bar_height), // Status bar with dynamic height
                    Constraint::Length(1),                 // Exit prompt at bottom
                ])
                .split(area)
        } else if non_exit_message.is_some() {
            Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Min(0),                    // Main content
                    Constraint::Length(1),                 // Message
                    Constraint::Length(status_bar_height), // Status bar with dynamic height
                ])
                .split(area)
        } else {
            Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Min(0),                    // Main content
                    Constraint::Length(status_bar_height), // Status bar with dynamic height
                ])
                .split(area)
        };

        // Render main content with ViewLayout
        let layout = ViewLayout::new("Session Viewer".to_string())
            .with_subtitle(subtitle)
            .with_status_bar(false); // We'll render our own status bar

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
            f.render_widget(message_widget, chunks[1]);
        }

        // Render status bar
        let status_idx = if is_exit {
            1
        } else if non_exit_message.is_some() {
            2
        } else {
            1
        };
        if chunks.len() > status_idx {
            let status_bar = Paragraph::new(Text::from(status_text))
                .style(Style::default().fg(Color::DarkGray))
                .alignment(ratatui::layout::Alignment::Left)
                .wrap(Wrap { trim: true });
            f.render_widget(status_bar, chunks[status_idx]);
        }

        // Render exit prompt at the very bottom if needed
        if is_exit {
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

    fn handle_key(&mut self, key: KeyEvent) -> Option<Message> {
        if self.is_searching {
            match key.code {
                KeyCode::Esc => {
                    self.is_searching = false;
                    self.text_input.set_text(String::new());
                    Some(Message::SessionQueryChanged(String::new()))
                }
                KeyCode::Enter => {
                    self.is_searching = false;
                    // Navigate to result detail for selected message
                    self.list_viewer.get_selected_item().and_then(|item| {
                        self.file_path.as_ref().map(|path| {
                            Message::EnterResultDetailFromSession(
                                item.raw_json.clone(),
                                path.clone(),
                                self.session_id.clone(),
                            )
                        })
                    })
                }
                KeyCode::Up => {
                    self.list_viewer.move_up();
                    Some(Message::SessionNavigated(
                        self.list_viewer.selected_index,
                        self.list_viewer.scroll_offset,
                    ))
                }
                KeyCode::Down => {
                    self.list_viewer.move_down();
                    Some(Message::SessionNavigated(
                        self.list_viewer.selected_index,
                        self.list_viewer.scroll_offset,
                    ))
                }
                KeyCode::Char('p') if key.modifiers == KeyModifiers::CONTROL => {
                    self.list_viewer.move_up();
                    Some(Message::SessionNavigated(
                        self.list_viewer.selected_index,
                        self.list_viewer.scroll_offset,
                    ))
                }
                KeyCode::Char('n') if key.modifiers == KeyModifiers::CONTROL => {
                    self.list_viewer.move_down();
                    Some(Message::SessionNavigated(
                        self.list_viewer.selected_index,
                        self.list_viewer.scroll_offset,
                    ))
                }
                KeyCode::Tab if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                    Some(Message::ToggleSessionRoleFilter)
                }
                KeyCode::Char('o') if key.modifiers == KeyModifiers::CONTROL => {
                    Some(Message::ToggleSessionOrder)
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
                    self.list_viewer.move_up();
                    Some(Message::SessionNavigated(
                        self.list_viewer.selected_index,
                        self.list_viewer.scroll_offset,
                    ))
                }
                KeyCode::Down => {
                    self.list_viewer.move_down();
                    Some(Message::SessionNavigated(
                        self.list_viewer.selected_index,
                        self.list_viewer.scroll_offset,
                    ))
                }
                KeyCode::Char('p') if key.modifiers == KeyModifiers::CONTROL => {
                    self.list_viewer.move_up();
                    Some(Message::SessionNavigated(
                        self.list_viewer.selected_index,
                        self.list_viewer.scroll_offset,
                    ))
                }
                KeyCode::Char('n') if key.modifiers == KeyModifiers::CONTROL => {
                    self.list_viewer.move_down();
                    Some(Message::SessionNavigated(
                        self.list_viewer.selected_index,
                        self.list_viewer.scroll_offset,
                    ))
                }
                KeyCode::Char('u') if key.modifiers == KeyModifiers::CONTROL => {
                    self.list_viewer.half_page_up();
                    Some(Message::SessionNavigated(
                        self.list_viewer.selected_index,
                        self.list_viewer.scroll_offset,
                    ))
                }
                KeyCode::Char('d') if key.modifiers == KeyModifiers::CONTROL => {
                    self.list_viewer.half_page_down();
                    Some(Message::SessionNavigated(
                        self.list_viewer.selected_index,
                        self.list_viewer.scroll_offset,
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
                // Unified copy operations
                KeyCode::Char('c') => self.list_viewer.get_selected_item().map(|item| {
                    Message::CopyToClipboard(CopyContent::MessageContent(item.content.clone()))
                }),
                KeyCode::Char('C') => self.list_viewer.get_selected_item().map(|item| {
                    Message::CopyToClipboard(CopyContent::JsonData(item.raw_json.clone()))
                }),
                KeyCode::Char('i') => self
                    .session_id
                    .clone()
                    .map(|id| Message::CopyToClipboard(CopyContent::SessionId(id))),
                KeyCode::Char('p') => self
                    .project_path
                    .clone()
                    .map(|path| Message::CopyToClipboard(CopyContent::ProjectPath(path))),
                KeyCode::Char('f') => self
                    .file_path
                    .clone()
                    .map(|path| Message::CopyToClipboard(CopyContent::FilePath(path))),
                KeyCode::Enter => self.list_viewer.get_selected_item().and_then(|item| {
                    self.file_path.as_ref().map(|path| {
                        Message::EnterMessageDetailFromSession(
                            item.raw_json.clone(),
                            path.clone(),
                            self.session_id.clone(),
                        )
                    })
                }),
                KeyCode::Esc => Some(Message::ExitToSearch),
                _ => None,
            }
        }
    }
}
