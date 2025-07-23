pub mod clipboard;
pub mod components;

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
}
impl Default for UIState {
    fn default() -> Self {
        Self {
            mode: Mode::Search,
            message: None,
            truncation_enabled: true,
            mode_stack: vec![],
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
    let search_state = hooks.use_state(SearchState::default);
    let detail_state = hooks.use_state(DetailState::default);
    let session_state = hooks.use_state(SessionState::default);
    let ui_state = hooks.use_state(UIState::default);

    let search_service = Arc::new(SearchService::new(props.options.clone()));
    let cache_service = Arc::new(Mutex::new(CacheService::new()));
    let session_service = Arc::new(SessionService::new(cache_service.clone()));

    // Perform initial search
    {
        let mut search_state_copy = search_state;
        let search_service_copy = search_service.clone();
        let pattern_copy = props.pattern.clone();
        perform_search(&mut search_state_copy, &search_service_copy, &pattern_copy);
    }

    // Handle terminal events
    hooks.use_terminal_events({
        let mut search_state = search_state;
        let mut detail_state = detail_state;
        let mut session_state = session_state;
        let mut ui_state = ui_state;
        let search_service = search_service.clone();
        let session_service = session_service.clone();
        let pattern = props.pattern.clone();

        move |event| {
            if let TerminalEvent::Key(key) = event {
                let current_mode = ui_state.read().mode;
                match current_mode {
                    Mode::Search => handle_search_input(
                        &mut search_state,
                        &mut ui_state,
                        &mut detail_state,
                        key,
                        &search_service,
                        &pattern,
                    ),
                    Mode::ResultDetail => handle_detail_input(
                        &mut detail_state,
                        &mut ui_state,
                        &mut session_state,
                        key,
                        &session_service,
                    ),
                    Mode::SessionViewer => {
                        handle_session_input(&mut session_state, &mut ui_state, key)
                    }
                    Mode::Help => handle_help_input(&mut ui_state, key),
                }
            }
        }
    });

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
    key: iocraft::KeyEvent,
    search_service: &Arc<SearchService>,
    pattern: &str,
) {
    use iocraft::{KeyCode, KeyModifiers};

    match key.code {
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            std::process::exit(0);
        }
        KeyCode::Char('r') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            ui_state.write().message = Some("Cache cleared. Reloading...".to_string());
            perform_search(search_state, search_service, pattern);
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
            perform_search(search_state, search_service, pattern);
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
            search_state.write().query.push(c);
            perform_search(search_state, search_service, pattern);
        }
        KeyCode::Backspace => {
            search_state.write().query.pop();
            perform_search(search_state, search_service, pattern);
        }
        KeyCode::Up => {
            let mut search_write = search_state.write();
            if search_write.selected_index > 0 {
                search_write.selected_index -= 1;
                if search_write.selected_index < search_write.scroll_offset {
                    search_write.scroll_offset = search_write.selected_index;
                }
            }
        }
        KeyCode::Down => {
            let mut search_write = search_state.write();
            if search_write.selected_index < search_write.results.len().saturating_sub(1) {
                search_write.selected_index += 1;
                if search_write.selected_index >= search_write.scroll_offset + 10 {
                    search_write.scroll_offset = search_write.selected_index - 9;
                }
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
            let mut search_write = search_state.write();
            search_write.selected_index = 0;
            search_write.scroll_offset = 0;
        }
        KeyCode::End => {
            let mut search_write = search_state.write();
            if !search_write.results.is_empty() {
                search_write.selected_index = search_write.results.len() - 1;
                search_write.scroll_offset = search_write.selected_index.saturating_sub(9);
            }
        }
        KeyCode::PageUp => {
            let mut search_write = search_state.write();
            let page_size = 10;
            search_write.selected_index = search_write.selected_index.saturating_sub(page_size);
            search_write.scroll_offset = search_write.scroll_offset.saturating_sub(page_size);
        }
        KeyCode::PageDown => {
            let mut search_write = search_state.write();
            let page_size = 10;
            let max_index = search_write.results.len().saturating_sub(1);
            search_write.selected_index = (search_write.selected_index + page_size).min(max_index);
            if search_write.selected_index >= search_write.scroll_offset + 10 {
                search_write.scroll_offset = search_write.selected_index.saturating_sub(9);
            }
        }
        KeyCode::Esc => {
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
                        let preview = if msg.len() <= 50 {
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
    pattern: &str,
) {
    search_state.write().is_searching = true;

    let search_read = search_state.read();
    let request = SearchRequest {
        id: 0,
        query: search_read.query.clone(),
        role_filter: search_read.role_filter.clone(),
        pattern: pattern.to_string(),
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
