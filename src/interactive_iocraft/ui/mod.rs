pub mod clipboard;
#[cfg(test)]
mod clipboard_operations_test;
pub mod components;
#[cfg(test)]
mod cursor_management_test;
#[cfg(test)]
mod display_size_test;
#[cfg(test)]
mod dynamic_size_test;
#[cfg(test)]
mod keyboard_actions_test;
#[cfg(test)]
mod keyboard_navigation_test;
#[cfg(test)]
mod multibyte_test;
#[cfg(test)]
mod session_viewer_test;
#[cfg(test)]
mod text_wrapping_test;

use self::clipboard::copy_to_clipboard;
use self::components::help_view::HelpView;
use self::components::result_detail_view::ResultDetailView;
use self::components::search_view::SearchView;
use self::components::session_viewer_view::SessionViewerView;
use crate::SearchOptions;
use crate::interactive_iocraft::application::{CacheService, SearchService, SessionService};
use crate::interactive_iocraft::domain::{Mode, SearchRequest};
use crate::query::condition::SearchResult;
use iocraft::prelude::*;
use iocraft::{KeyCode, KeyModifiers};
use std::sync::{Arc, Mutex};

// State split into smaller parts for use_state
#[derive(Clone, Default)]
pub struct SearchState {
    pub query: String,
    pub role_filter: Option<String>,
    pub results: Vec<SearchResult>,
    pub selected_index: usize,
    pub scroll_offset: usize,
    pub is_searching: bool,
    pub cursor_position: usize,
}

impl SearchState {
    pub fn move_cursor_left(&mut self) {
        if self.cursor_position > 0 {
            self.cursor_position -= 1;
        }
    }

    pub fn move_cursor_right(&mut self) {
        let char_count = self.query.chars().count();
        if self.cursor_position < char_count {
            self.cursor_position += 1;
        }
    }

    pub fn move_cursor_to_start(&mut self) {
        self.cursor_position = 0;
    }

    pub fn move_cursor_to_end(&mut self) {
        self.cursor_position = self.query.chars().count();
    }

    pub fn move_cursor_word_left(&mut self) {
        if self.cursor_position == 0 {
            return;
        }

        let chars: Vec<char> = self.query.chars().collect();
        let mut pos = self.cursor_position - 1;

        // Skip whitespace backwards
        while pos > 0 && chars[pos].is_whitespace() {
            pos -= 1;
        }

        // Skip word backwards
        while pos > 0 && !chars[pos - 1].is_whitespace() {
            pos -= 1;
        }

        self.cursor_position = pos;
    }

    pub fn move_cursor_word_right(&mut self) {
        let chars: Vec<char> = self.query.chars().collect();
        let len = chars.len();

        if self.cursor_position >= len {
            return;
        }

        let mut pos = self.cursor_position;

        // Skip current word
        while pos < len && !chars[pos].is_whitespace() {
            pos += 1;
        }

        // Skip whitespace
        while pos < len && chars[pos].is_whitespace() {
            pos += 1;
        }

        self.cursor_position = pos;
    }

    pub fn insert_char_at_cursor(&mut self, ch: char) {
        let chars: Vec<char> = self.query.chars().collect();
        let mut new_chars = Vec::with_capacity(chars.len() + 1);

        new_chars.extend_from_slice(&chars[..self.cursor_position]);
        new_chars.push(ch);
        new_chars.extend_from_slice(&chars[self.cursor_position..]);

        self.query = new_chars.into_iter().collect();
        self.cursor_position += 1;
    }

    pub fn delete_char_before_cursor(&mut self) {
        if self.cursor_position == 0 {
            return;
        }

        let chars: Vec<char> = self.query.chars().collect();
        let mut new_chars = Vec::with_capacity(chars.len() - 1);

        new_chars.extend_from_slice(&chars[..self.cursor_position - 1]);
        new_chars.extend_from_slice(&chars[self.cursor_position..]);

        self.query = new_chars.into_iter().collect();
        self.cursor_position -= 1;
    }

    pub fn delete_char_at_cursor(&mut self) {
        let chars: Vec<char> = self.query.chars().collect();
        if self.cursor_position >= chars.len() {
            return;
        }

        let mut new_chars = Vec::with_capacity(chars.len() - 1);
        new_chars.extend_from_slice(&chars[..self.cursor_position]);
        new_chars.extend_from_slice(&chars[self.cursor_position + 1..]);

        self.query = new_chars.into_iter().collect();
    }

    // Calculate dynamic scroll offset
    pub fn calculate_scroll_offset(&self, visible_height: usize) -> usize {
        if self.selected_index < visible_height / 2 {
            0
        } else if self.selected_index >= self.results.len().saturating_sub(visible_height / 2) {
            self.results.len().saturating_sub(visible_height)
        } else {
            self.selected_index.saturating_sub(visible_height / 2)
        }
    }

    // Calculate dynamic visible range
    pub fn calculate_visible_range(&self, terminal_height: usize) -> (usize, usize) {
        let header_lines = 7; // Based on actual header line count
        let footer_lines = 3; // Based on actual footer line count
        let visible_height = terminal_height
            .saturating_sub(header_lines + footer_lines)
            .max(1);

        let scroll_offset = self.calculate_scroll_offset(visible_height);
        let end_index = (scroll_offset + visible_height).min(self.results.len());

        (scroll_offset, end_index)
    }

    // Wrap text to fit within specified width
    pub fn wrap_text(text: &str, width: usize) -> Vec<String> {
        if text.is_empty() {
            return vec![String::new()];
        }

        if width == 0 {
            return vec![text.to_string()];
        }

        let mut wrapped_lines = Vec::new();

        // First split by existing newlines
        for line in text.split('\n') {
            if line.chars().count() <= width {
                wrapped_lines.push(line.to_string());
            } else {
                // Need to wrap this line
                let chars: Vec<char> = line.chars().collect();
                let mut start = 0;

                while start < chars.len() {
                    // Find the end position for this line
                    let mut end = (start + width).min(chars.len());

                    // If we're not at the end of the text and the break point is not a space,
                    // try to find the last space before the break point
                    if end < chars.len() && end > start {
                        let mut last_space = None;

                        // Look for the last space in the current segment
                        for (i, &ch) in chars[start..end].iter().enumerate() {
                            if ch == ' ' {
                                last_space = Some(start + i + 1); // Include the space in the line
                            }
                        }

                        // If we found a space, use it as the break point
                        if let Some(space_pos) = last_space {
                            end = space_pos;
                        } else if chars.get(end) == Some(&' ') {
                            // If the break point is exactly on a space, include it
                            end += 1;
                        }
                    }

                    // Collect the characters for this line
                    let line_chars: String = chars[start..end].iter().collect();
                    wrapped_lines.push(line_chars.trim().to_string());

                    // Move start position, skipping any leading spaces
                    start = end;
                    while start < chars.len() && chars[start] == ' ' {
                        start += 1;
                    }
                }
            }
        }

        wrapped_lines
    }
}

#[derive(Clone, Default)]
pub struct DetailState {
    pub selected_result: Option<SearchResult>,
    pub scroll_offset: usize,
}

#[derive(Clone, Default)]
pub struct SessionState {
    pub messages: Vec<String>,
    pub filtered_indices: Vec<usize>,
    pub selected_index: usize,
    pub scroll_offset: usize,
    pub query: String,
    pub file_path: Option<String>,
    pub session_id: Option<String>,
}

#[derive(Clone)]
pub struct UIState {
    pub mode: Mode,
    pub message: Option<String>,
    pub truncation_enabled: bool,
    pub mode_stack: Vec<Mode>,
    pub terminal_height: usize,
}
impl Default for UIState {
    fn default() -> Self {
        Self {
            mode: Mode::Search,
            message: None,
            truncation_enabled: true,
            mode_stack: vec![],
            terminal_height: 30, // Default value
        }
    }
}

#[derive(Default, Props)]
pub struct AppProps {
    pub pattern: String,
    pub options: SearchOptions,
}

#[component]
pub fn App(props: &AppProps, mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
    let mut search_state = hooks.use_state(SearchState::default);
    let detail_state = hooks.use_state(DetailState::default);
    let session_state = hooks.use_state(SessionState::default);
    let ui_state = hooks.use_state(UIState::default);

    let search_service = Arc::new(SearchService::new(props.options.clone()));
    let cache_service = Arc::new(Mutex::new(CacheService::new()));
    let session_service = Arc::new(SessionService::new(cache_service.clone()));

    // Perform initial search with the query passed as pattern
    if !props.pattern.is_empty() {
        let mut search_write = search_state.write();
        search_write.query = props.pattern.clone();
        search_write.is_searching = true;
        drop(search_write);

        let request = SearchRequest {
            id: 0,
            query: props.pattern.clone(),
            role_filter: None,
            pattern: crate::default_claude_pattern(),
        };

        match search_service.search(request) {
            Ok(response) => {
                let mut search_write = search_state.write();
                search_write.results = response.results;
                search_write.selected_index = 0;
                search_write.scroll_offset = 0;
                search_write.is_searching = false;
            }
            Err(_) => {
                let mut search_write = search_state.write();
                search_write.results = vec![];
                search_write.is_searching = false;
            }
        }
    }

    // Handle terminal events
    {
        let search_service = search_service.clone();
        let session_service = session_service.clone();
        let mut search_state_copy = search_state.clone();
        let mut detail_state_copy = detail_state.clone();
        let mut session_state_copy = session_state.clone();
        let mut ui_state_copy = ui_state.clone();

        hooks.use_terminal_events(move |event| {
            if let TerminalEvent::Key(key) = event {
                // Global key handling - works in all modes
                match key.code {
                    KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        std::process::exit(0);
                    }
                    _ => {}
                }

                let current_mode = ui_state_copy.read().mode;
                match current_mode {
                    Mode::Search => handle_search_input(
                        &mut search_state_copy,
                        &mut ui_state_copy,
                        &mut detail_state_copy,
                        &mut session_state_copy,
                        key,
                        &search_service,
                        &session_service,
                    ),
                    Mode::ResultDetail => handle_detail_input(
                        &mut detail_state_copy,
                        &mut ui_state_copy,
                        &mut session_state_copy,
                        key,
                        &session_service,
                    ),
                    Mode::SessionViewer => {
                        handle_session_input(&mut session_state_copy, &mut ui_state_copy, key)
                    }
                    Mode::Help => handle_help_input(&mut ui_state_copy, key),
                }
            }
        });
    }

    let current_mode = ui_state.read().mode;

    element! {
        View(flex_direction: FlexDirection::Column, width: Size::Length(100), height: Size::Length(100)) {
            #(vec![match current_mode {
                Mode::Search => {
                    let search_state_clone = search_state.read().clone();
                    let ui_state_clone = ui_state.read().clone();
                    element! { SearchView(search_state: search_state_clone, ui_state: ui_state_clone) }.into_any()
                }
                Mode::ResultDetail => {
                    let detail_state_clone = detail_state.read().clone();
                    let ui_state_clone = ui_state.read().clone();
                    element! { ResultDetailView(detail_state: detail_state_clone, ui_state: ui_state_clone) }.into_any()
                }
                Mode::SessionViewer => {
                    let session_state_clone = session_state.read().clone();
                    let ui_state_clone = ui_state.read().clone();
                    element! { SessionViewerView(session_state: session_state_clone, ui_state: ui_state_clone) }.into_any()
                }
                Mode::Help => {
                    let ui_state_clone = ui_state.read().clone();
                    element! { HelpView(ui_state: ui_state_clone) }.into_any()
                }
            }])
        }
    }
}

fn handle_search_input(
    search_state: &mut iocraft::hooks::State<SearchState>,
    ui_state: &mut iocraft::hooks::State<UIState>,
    detail_state: &mut iocraft::hooks::State<DetailState>,
    session_state: &mut iocraft::hooks::State<SessionState>,
    key: iocraft::KeyEvent,
    search_service: &Arc<SearchService>,
    session_service: &Arc<SessionService>,
) {
    use iocraft::{KeyCode, KeyModifiers};

    match key.code {
        KeyCode::Char('r') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            ui_state.write().message = Some("Cache cleared. Reloading...".to_string());
            perform_search(search_state, search_service);
        }
        KeyCode::Char('t') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            let mut ui_write = ui_state.write();
            ui_write.truncation_enabled = !ui_write.truncation_enabled;
            let mode = if ui_write.truncation_enabled {
                "Truncated"
            } else {
                "Full Text"
            };
            ui_write.message = Some(format!("Display mode: {mode}"));
        }
        KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            // Ctrl+S: Enter session viewer for selected result
            let search_read = search_state.read();
            if let Some(result) = search_read.results.get(search_read.selected_index) {
                let mut session_write = session_state.write();
                session_write.file_path = Some(result.file.clone());
                session_write.session_id = Some(result.session_id.clone());
                session_write.query.clear();
                session_write.selected_index = 0;
                session_write.scroll_offset = 0;

                // Load session messages
                match session_service.get_raw_lines(&result.file) {
                    Ok(lines) => {
                        session_write.messages = lines;
                        session_write.filtered_indices =
                            (0..session_write.messages.len()).collect();
                    }
                    Err(e) => {
                        ui_state.write().message = Some(format!("Failed to load session: {e}"));
                        return;
                    }
                }

                drop(session_write);
                drop(search_read);

                let mut ui_write = ui_state.write();
                let current_mode = ui_write.mode;
                ui_write.mode_stack.push(current_mode);
                ui_write.mode = Mode::SessionViewer;
            } else {
                ui_state.write().message = Some("No result selected".to_string());
            }
        }
        KeyCode::Char('?') => {
            let mut ui_write = ui_state.write();
            let current_mode = ui_write.mode;
            ui_write.mode_stack.push(current_mode);
            ui_write.mode = Mode::Help;
        }
        KeyCode::Tab => {
            let mut search_write = search_state.write();
            search_write.role_filter = match search_write.role_filter.as_deref() {
                None => Some("user".to_string()),
                Some("user") => Some("assistant".to_string()),
                Some("assistant") => Some("system".to_string()),
                Some("system") => Some("summary".to_string()),
                Some("summary") => None,
                _ => None,
            };
            drop(search_write);
            perform_search(search_state, search_service);
        }
        KeyCode::Char('y') | KeyCode::Char('Y') => {
            let search_read = search_state.read();
            if let Some(result) = search_read.results.get(search_read.selected_index) {
                if let Err(e) = copy_to_clipboard(&result.text) {
                    ui_state.write().message = Some(format!("Failed to copy: {e}"));
                } else {
                    ui_state.write().message = Some("✓ Copied message text".to_string());
                }
            }
        }
        KeyCode::Char('f') | KeyCode::Char('F') => {
            let search_read = search_state.read();
            if let Some(result) = search_read.results.get(search_read.selected_index) {
                if let Err(e) = copy_to_clipboard(&result.file) {
                    ui_state.write().message = Some(format!("Failed to copy: {e}"));
                } else {
                    ui_state.write().message = Some("✓ Copied file path".to_string());
                }
            }
        }
        KeyCode::Char('u') | KeyCode::Char('U') => {
            let search_read = search_state.read();
            if let Some(result) = search_read.results.get(search_read.selected_index) {
                if let Err(e) = copy_to_clipboard(&result.session_id) {
                    ui_state.write().message = Some(format!("Failed to copy: {e}"));
                } else {
                    ui_state.write().message = Some("✓ Copied session ID".to_string());
                }
            }
        }
        KeyCode::Char(c) => {
            search_state.write().insert_char_at_cursor(c);
            perform_search(search_state, search_service);
        }
        KeyCode::Backspace => {
            search_state.write().delete_char_before_cursor();
            perform_search(search_state, search_service);
        }
        KeyCode::Delete => {
            search_state.write().delete_char_at_cursor();
            perform_search(search_state, search_service);
        }
        KeyCode::Left => {
            if key.modifiers.contains(KeyModifiers::CONTROL) {
                search_state.write().move_cursor_word_left();
            } else {
                search_state.write().move_cursor_left();
            }
        }
        KeyCode::Right => {
            if key.modifiers.contains(KeyModifiers::CONTROL) {
                search_state.write().move_cursor_word_right();
            } else {
                search_state.write().move_cursor_right();
            }
        }
        KeyCode::Up => {
            let mut search_write = search_state.write();
            if search_write.selected_index > 0 {
                search_write.selected_index -= 1;
                // Dynamic scroll adjustment
                let ui_read = ui_state.read();
                let visible_height = ui_read.terminal_height.saturating_sub(10).max(1);
                drop(ui_read);
                search_write.scroll_offset = search_write.calculate_scroll_offset(visible_height);
            }
        }
        KeyCode::Down => {
            let mut search_write = search_state.write();
            if search_write.selected_index < search_write.results.len().saturating_sub(1) {
                search_write.selected_index += 1;
                // Dynamic scroll adjustment
                let ui_read = ui_state.read();
                let visible_height = ui_read.terminal_height.saturating_sub(10).max(1);
                drop(ui_read);
                search_write.scroll_offset = search_write.calculate_scroll_offset(visible_height);
            }
        }
        KeyCode::Enter => {
            let search_read = search_state.read();
            if search_read.results.is_empty() {
                ui_state.write().message = Some("No results to select".to_string());
            } else if let Some(result) = search_read.results.get(search_read.selected_index) {
                let mut detail_write = detail_state.write();
                detail_write.selected_result = Some(result.clone());
                detail_write.scroll_offset = 0;
                drop(detail_write);
                let mut ui_write = ui_state.write();
                let current_mode = ui_write.mode;
                ui_write.mode_stack.push(current_mode);
                ui_write.mode = Mode::ResultDetail;
            }
        }
        KeyCode::Home => {
            if key.modifiers.contains(KeyModifiers::CONTROL) || key.modifiers.is_empty() {
                // Ctrl+Home or Home: Move cursor to start for text editing
                search_state.write().move_cursor_to_start();
            } else if key.modifiers.contains(KeyModifiers::SHIFT) {
                // Shift+Home: Jump to start of results list
                let mut search_write = search_state.write();
                search_write.selected_index = 0;
                search_write.scroll_offset = 0;
            }
        }
        KeyCode::End => {
            if key.modifiers.contains(KeyModifiers::CONTROL) || key.modifiers.is_empty() {
                // Ctrl+End or End: Move cursor to end for text editing
                search_state.write().move_cursor_to_end();
            } else if key.modifiers.contains(KeyModifiers::SHIFT) {
                // Shift+End: Jump to end of results list
                let mut search_write = search_state.write();
                if !search_write.results.is_empty() {
                    search_write.selected_index = search_write.results.len() - 1;
                    let ui_read = ui_state.read();
                    let visible_height = ui_read.terminal_height.saturating_sub(10).max(1);
                    drop(ui_read);
                    search_write.scroll_offset =
                        search_write.calculate_scroll_offset(visible_height);
                }
            }
        }
        KeyCode::PageUp => {
            let mut search_write = search_state.write();
            let ui_read = ui_state.read();
            let visible_height = ui_read.terminal_height.saturating_sub(10).max(1);
            drop(ui_read);
            let page_size = visible_height;
            search_write.selected_index = search_write.selected_index.saturating_sub(page_size);
            search_write.scroll_offset = search_write.calculate_scroll_offset(visible_height);
        }
        KeyCode::PageDown => {
            let mut search_write = search_state.write();
            let ui_read = ui_state.read();
            let visible_height = ui_read.terminal_height.saturating_sub(10).max(1);
            drop(ui_read);
            let page_size = visible_height;
            let max_index = search_write.results.len().saturating_sub(1);
            search_write.selected_index = (search_write.selected_index + page_size).min(max_index);
            search_write.scroll_offset = search_write.calculate_scroll_offset(visible_height);
        }
        KeyCode::Esc => {
            // In Search mode, Esc exits the application
            std::process::exit(0);
        }
        _ => {}
    }
}

fn handle_detail_input(
    detail_state: &mut iocraft::hooks::State<DetailState>,
    ui_state: &mut iocraft::hooks::State<UIState>,
    session_state: &mut iocraft::hooks::State<SessionState>,
    key: iocraft::KeyEvent,
    session_service: &Arc<SessionService>,
) {
    use iocraft::KeyCode;

    match key.code {
        KeyCode::Esc => {
            let mut ui_write = ui_state.write();
            if let Some(prev_mode) = ui_write.mode_stack.pop() {
                ui_write.mode = prev_mode;
            } else {
                ui_write.mode = Mode::Search;
            }
            ui_write.message = None;
            drop(ui_write);
            detail_state.write().scroll_offset = 0;
        }
        KeyCode::Char('s') | KeyCode::Char('S') => {
            let detail_read = detail_state.read();
            if let Some(ref result) = detail_read.selected_result {
                let mut session_write = session_state.write();
                session_write.file_path = Some(result.file.clone());
                session_write.session_id = Some(result.session_id.clone());
                session_write.query.clear();
                session_write.selected_index = 0;
                session_write.scroll_offset = 0;

                // Load session messages
                match session_service.get_raw_lines(&result.file) {
                    Ok(lines) => {
                        session_write.messages = lines;
                        session_write.filtered_indices =
                            (0..session_write.messages.len()).collect();
                    }
                    Err(e) => {
                        ui_state.write().message = Some(format!("Failed to load session: {e}"));
                        return;
                    }
                }

                drop(session_write);
                let mut ui_write = ui_state.write();
                let current_mode = ui_write.mode;
                ui_write.mode_stack.push(current_mode);
                ui_write.mode = Mode::SessionViewer;
            }
        }
        KeyCode::Char('j') | KeyCode::Down => {
            detail_state.write().scroll_offset += 1;
        }
        KeyCode::Char('k') | KeyCode::Up => {
            let mut detail_write = detail_state.write();
            detail_write.scroll_offset = detail_write.scroll_offset.saturating_sub(1);
        }
        KeyCode::PageDown => {
            detail_state.write().scroll_offset += 10;
        }
        KeyCode::PageUp => {
            let mut detail_write = detail_state.write();
            detail_write.scroll_offset = detail_write.scroll_offset.saturating_sub(10);
        }
        KeyCode::Char('y') | KeyCode::Char('Y') => {
            let detail_read = detail_state.read();
            if let Some(ref result) = detail_read.selected_result {
                if let Err(e) = copy_to_clipboard(&result.text) {
                    ui_state.write().message = Some(format!("Failed to copy: {e}"));
                } else {
                    ui_state.write().message = Some("✓ Copied message text".to_string());
                }
            }
        }
        KeyCode::Char('f') | KeyCode::Char('F') => {
            let detail_read = detail_state.read();
            if let Some(ref result) = detail_read.selected_result {
                if let Err(e) = copy_to_clipboard(&result.file) {
                    ui_state.write().message = Some(format!("Failed to copy: {e}"));
                } else {
                    ui_state.write().message = Some("✓ Copied file path".to_string());
                }
            }
        }
        KeyCode::Char('u') | KeyCode::Char('U') => {
            let detail_read = detail_state.read();
            if let Some(ref result) = detail_read.selected_result {
                if let Err(e) = copy_to_clipboard(&result.session_id) {
                    ui_state.write().message = Some(format!("Failed to copy: {e}"));
                } else {
                    ui_state.write().message = Some("✓ Copied session ID".to_string());
                }
            }
        }
        KeyCode::Char('a') | KeyCode::Char('A') => {
            let detail_read = detail_state.read();
            if let Some(ref result) = detail_read.selected_result {
                let full_details = format!(
                    "File: {}\nUUID: {}\nRole: {}\nTimestamp: {}\n\n{}",
                    result.file, result.session_id, result.role, result.timestamp, result.text
                );
                if let Err(e) = copy_to_clipboard(&full_details) {
                    ui_state.write().message = Some(format!("Failed to copy: {e}"));
                } else {
                    ui_state.write().message = Some("✓ Copied full result details".to_string());
                }
            }
        }
        _ => {}
    }
}

fn handle_session_input(
    session_state: &mut iocraft::hooks::State<SessionState>,
    ui_state: &mut iocraft::hooks::State<UIState>,
    key: iocraft::KeyEvent,
) {
    use iocraft::KeyCode;

    match key.code {
        KeyCode::Esc => {
            let mut ui_write = ui_state.write();
            if let Some(prev_mode) = ui_write.mode_stack.pop() {
                ui_write.mode = prev_mode;
            } else {
                ui_write.mode = Mode::Search;
            }
        }
        KeyCode::Backspace => {
            let mut session_write = session_state.write();
            if session_write.query.is_empty() {
                // If query is empty, go back to previous mode
                drop(session_write);
                let mut ui_write = ui_state.write();
                if let Some(prev_mode) = ui_write.mode_stack.pop() {
                    ui_write.mode = prev_mode;
                } else {
                    ui_write.mode = Mode::Search;
                }
            } else {
                // Otherwise, remove last character from query
                session_write.query.pop();
                // Update filtered indices
                let messages = session_write.messages.clone();
                let query = session_write.query.clone();
                session_write.filtered_indices =
                    crate::interactive_iocraft::domain::filter::SessionFilter::filter_messages(
                        &messages, &query,
                    );
                session_write.selected_index = 0;
                session_write.scroll_offset = 0;
            }
        }
        KeyCode::Char('c') => {
            let session_read = session_state.read();
            if let Some(&msg_idx) = session_read
                .filtered_indices
                .get(session_read.selected_index)
            {
                if let Some(msg) = session_read.messages.get(msg_idx) {
                    if let Err(e) = copy_to_clipboard(msg) {
                        ui_state.write().message = Some(format!("Failed to copy: {e}"));
                    } else {
                        let preview = if msg.chars().count() <= 50 {
                            format!("✓ Copied: {msg}")
                        } else {
                            "✓ Copied message text".to_string()
                        };
                        ui_state.write().message = Some(preview);
                    }
                }
            }
        }
        KeyCode::Char('C') => {
            let session_read = session_state.read();
            let all_messages = session_read.messages.join("\n\n");
            if let Err(e) = copy_to_clipboard(&all_messages) {
                ui_state.write().message = Some(format!("Failed to copy: {e}"));
            } else {
                ui_state.write().message = Some("✓ Copied all messages".to_string());
            }
        }
        KeyCode::Char('i') | KeyCode::Char('I') => {
            let session_read = session_state.read();
            if let Some(ref session_id) = session_read.session_id {
                if let Err(e) = copy_to_clipboard(session_id) {
                    ui_state.write().message = Some(format!("Failed to copy: {e}"));
                } else {
                    ui_state.write().message = Some("✓ Copied session ID".to_string());
                }
            }
        }
        KeyCode::Char(c) => {
            let mut session_write = session_state.write();
            session_write.query.push(c);
            // Update filtered indices
            let messages = session_write.messages.clone();
            let query = session_write.query.clone();
            session_write.filtered_indices =
                crate::interactive_iocraft::domain::filter::SessionFilter::filter_messages(
                    &messages, &query,
                );
            session_write.selected_index = 0;
            session_write.scroll_offset = 0;
        }
        KeyCode::Up => {
            let mut session_write = session_state.write();
            if session_write.selected_index > 0 {
                session_write.selected_index -= 1;
                if session_write.selected_index < session_write.scroll_offset {
                    session_write.scroll_offset = session_write.selected_index;
                }
            }
        }
        KeyCode::Down => {
            let mut session_write = session_state.write();
            let max_index = session_write.filtered_indices.len().saturating_sub(1);
            if session_write.selected_index < max_index {
                session_write.selected_index += 1;
                if session_write.selected_index >= session_write.scroll_offset + 10 {
                    session_write.scroll_offset = session_write.selected_index - 9;
                }
            }
        }
        _ => {}
    }
}

fn handle_help_input(ui_state: &mut iocraft::hooks::State<UIState>, key: iocraft::KeyEvent) {
    use iocraft::KeyCode;

    match key.code {
        KeyCode::Esc | KeyCode::Char('?') | KeyCode::Enter => {
            let mut ui_write = ui_state.write();
            if let Some(prev_mode) = ui_write.mode_stack.pop() {
                ui_write.mode = prev_mode;
            } else {
                ui_write.mode = Mode::Search;
            }
        }
        _ => {}
    }
}

fn perform_search(
    search_state: &mut iocraft::hooks::State<SearchState>,
    search_service: &Arc<SearchService>,
) {
    search_state.write().is_searching = true;

    let search_read = search_state.read();
    let request = SearchRequest {
        id: 0,
        query: search_read.query.clone(),
        role_filter: search_read.role_filter.clone(),
        pattern: crate::default_claude_pattern(),
    };
    drop(search_read);

    match search_service.search(request) {
        Ok(response) => {
            let mut search_write = search_state.write();
            let _result_count = response.results.len();
            search_write.results = response.results;
            search_write.selected_index = 0;
            search_write.scroll_offset = 0;
            search_write.is_searching = false;
        }
        Err(_e) => {
            let mut search_write = search_state.write();
            search_write.results = vec![];
            search_write.is_searching = false;
        }
    }
}
