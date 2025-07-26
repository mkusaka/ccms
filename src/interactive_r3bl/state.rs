use r3bl_tui::{HasEditorBuffers, EditorBuffer, FlexBoxId};
use std::collections::HashMap;
use crate::{SearchOptions, schemas::SessionMessage};

#[derive(Debug, Clone)]
pub struct AppState {
    pub query: String,
    pub search_options: SearchOptions,
    pub search_results: Vec<SessionMessage>,
    pub selected_index: usize,
    pub is_searching: bool,
    pub show_help: bool,
    pub editor_buffers: HashMap<FlexBoxId, EditorBuffer>,
    pub mode: ViewMode,
    pub selected_session_path: Option<String>,
    pub session_messages: Vec<SessionMessage>,
    pub session_scroll_offset: usize,
    pub result_detail_scroll_offset: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ViewMode {
    Search,
    ResultDetail,
    SessionViewer,
    Help,
}

impl AppState {
    pub fn new(initial_query: String, options: SearchOptions) -> Self {
        Self {
            query: initial_query,
            search_options: options,
            search_results: Vec::new(),
            selected_index: 0,
            is_searching: false,
            show_help: false,
            editor_buffers: HashMap::new(),
            mode: ViewMode::Search,
            selected_session_path: None,
            session_messages: Vec::new(),
            session_scroll_offset: 0,
            result_detail_scroll_offset: 0,
        }
    }
    
    pub fn get_selected_message(&self) -> Option<&SessionMessage> {
        if self.selected_index < self.search_results.len() {
            Some(&self.search_results[self.selected_index])
        } else {
            None
        }
    }
}

impl HasEditorBuffers for AppState {
    fn get_editor_buffer(&self, id: &FlexBoxId) -> Option<&EditorBuffer> {
        self.editor_buffers.get(id)
    }

    fn insert_editor_buffer(&mut self, id: FlexBoxId, buffer: EditorBuffer) {
        self.editor_buffers.insert(id, buffer);
    }

    fn contains_editor_buffer(&self, id: &FlexBoxId) -> bool {
        self.editor_buffers.contains_key(id)
    }
}

#[derive(Debug, Clone)]
pub enum AppSignal {
    SearchCompleted(Vec<SessionMessage>),
    LoadSession(String),
    SessionLoaded(Vec<SessionMessage>),
    UpdateQuery(String),
    NavigateUp,
    NavigateDown,
    EnterResultDetail,
    EnterSessionViewer,
    ExitCurrentView,
    ShowHelp,
    Quit,
}