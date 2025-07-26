use r3bl_tui::*;
use std::collections::HashMap;

#[derive(Debug, Clone, Default)]
pub struct AppState {
    pub query: String,
    pub results: Vec<String>,
    pub selected_index: usize,
    pub is_searching: bool,
    pub show_help: bool,
    pub editor_buffers: HashMap<ComponentId, EditorBuffer>,
}

impl AppState {
    pub fn new() -> Self {
        Self::default()
    }
}

impl HasEditorBuffers for AppState {
    fn get_editor_buffer(&self, id: &ComponentId) -> Option<&EditorBuffer> {
        self.editor_buffers.get(id)
    }

    fn insert_editor_buffer(&mut self, id: ComponentId, buffer: EditorBuffer) {
        self.editor_buffers.insert(id, buffer);
    }

    fn contains_editor_buffer(&self, id: &ComponentId) -> bool {
        self.editor_buffers.contains_key(id)
    }
}

#[derive(Debug, Clone)]
pub enum AppSignal {
    SearchCompleted(Vec<String>),
    UpdateQuery(String),
    NavigateUp,
    NavigateDown,
    ShowHelp,
    Quit,
}