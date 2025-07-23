pub mod components;

use std::sync::Arc;
use iocraft::prelude::*;
use crate::interactive_iocraft::application::SearchService;
use crate::interactive_iocraft::domain::{Mode, SearchRequest};
use crate::query::condition::SearchResult;
use crate::SearchOptions;
use self::components::search_view::SearchView;
use self::components::result_detail_view::ResultDetailView;
use self::components::session_viewer_view::SessionViewerView;
use self::components::help_view::HelpView;

// State split into smaller parts for use_state
#[derive(Clone)]
pub struct SearchState {
    pub query: String,
    pub role_filter: Option<String>,
    pub results: Vec<SearchResult>,
    pub selected_index: usize,
    pub scroll_offset: usize,
    pub is_searching: bool,
}

#[derive(Clone)]
pub struct DetailState {
    pub selected_result: Option<SearchResult>,
    pub scroll_offset: usize,
}

#[derive(Clone)]
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
}

impl Default for SearchState {
    fn default() -> Self {
        Self {
            query: String::new(),
            role_filter: None,
            results: vec![],
            selected_index: 0,
            scroll_offset: 0,
            is_searching: false,
        }
    }
}

impl Default for DetailState {
    fn default() -> Self {
        Self {
            selected_result: None,
            scroll_offset: 0,
        }
    }
}

impl Default for SessionState {
    fn default() -> Self {
        Self {
            messages: vec![],
            filtered_indices: vec![],
            selected_index: 0,
            scroll_offset: 0,
            query: String::new(),
            file_path: None,
            session_id: None,
        }
    }
}

impl Default for UIState {
    fn default() -> Self {
        Self {
            mode: Mode::Search,
            message: None,
            truncation_enabled: true,
        }
    }
}

impl Default for Mode {
    fn default() -> Self {
        Mode::Search
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
    
    // Perform initial search
    {
        let mut search_state_copy = search_state.clone();
        let search_service_copy = search_service.clone();
        let pattern_copy = props.pattern.clone();
        perform_search(&mut search_state_copy, &search_service_copy, &pattern_copy);
    }
    
    // Handle terminal events
    hooks.use_terminal_events({
        let mut search_state = search_state.clone();
        let mut detail_state = detail_state.clone();
        let mut session_state = session_state.clone();
        let mut ui_state = ui_state.clone();
        let search_service = search_service.clone();
        let pattern = props.pattern.clone();
        
        move |event| {
            match event {
                TerminalEvent::Key(key) => {
                    let current_mode = ui_state.read().mode;
                    match current_mode {
                        Mode::Search => handle_search_input(
                            &mut search_state,
                            &mut ui_state,
                            &mut detail_state,
                            key,
                            &search_service,
                            &pattern
                        ),
                        Mode::ResultDetail => handle_detail_input(
                            &mut detail_state,
                            &mut ui_state,
                            &mut session_state,
                            key
                        ),
                        Mode::SessionViewer => handle_session_input(
                            &mut session_state,
                            &mut ui_state,
                            key
                        ),
                        Mode::Help => handle_help_input(&mut ui_state, key),
                    }
                }
                _ => {}
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
            ui_write.message = Some(format!("Display mode: {}", mode));
        }
        KeyCode::Char('?') => {
            ui_state.write().mode = Mode::Help;
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
            if let Some(result) = search_read.results.get(search_read.selected_index) {
                let mut detail_write = detail_state.write();
                detail_write.selected_result = Some(result.clone());
                detail_write.scroll_offset = 0;
                drop(detail_write);
                ui_state.write().mode = Mode::ResultDetail;
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
) {
    use iocraft::KeyCode;
    
    match key.code {
        KeyCode::Esc => {
            let mut ui_write = ui_state.write();
            ui_write.mode = Mode::Search;
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
                drop(session_write);
                ui_state.write().mode = Mode::SessionViewer;
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
        KeyCode::Esc | KeyCode::Backspace => {
            ui_state.write().mode = Mode::ResultDetail;
        }
        KeyCode::Char(c) => {
            let mut session_write = session_state.write();
            session_write.query.push(c);
            // Update filtered indices
            let messages = session_write.messages.clone();
            let query = session_write.query.clone();
            session_write.filtered_indices = 
                crate::interactive_iocraft::domain::filter::SessionFilter::filter_messages(
                    &messages, &query
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

fn handle_help_input(ui_state: &mut iocraft::hooks::State<UIState>, _key: iocraft::KeyEvent) {
    ui_state.write().mode = Mode::Search;
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