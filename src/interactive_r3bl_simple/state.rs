use crate::query::SearchResult;

#[derive(Debug, Clone)]
pub struct AppState {
    pub query: String,
    pub search_results: Vec<SearchResult>,
    pub selected_index: usize,
    pub is_searching: bool,
    pub scroll_offset: usize,
    pub show_help: bool,
    pub current_mode: ViewMode,
    pub status_message: Option<String>,
    pub needs_render: bool,
    pub ctrl_c_count: u8,
    pub last_ctrl_c_time: Option<std::time::Instant>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ViewMode {
    Search,
    ResultDetail,
    Help,
}

#[derive(Debug, Clone)]
pub enum SearchSignal {
    SearchCompleted(Vec<SearchResult>),
    SearchError(String),
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

impl AppState {
    pub fn new() -> Self {
        Self {
            query: String::new(),
            search_results: Vec::new(),
            selected_index: 0,
            is_searching: false,
            scroll_offset: 0,
            show_help: false,
            current_mode: ViewMode::Search,
            status_message: None,
            needs_render: true,
            ctrl_c_count: 0,
            last_ctrl_c_time: None,
        }
    }
    
    pub fn get_selected_result(&self) -> Option<&SearchResult> {
        if self.selected_index < self.search_results.len() {
            Some(&self.search_results[self.selected_index])
        } else {
            None
        }
    }
    
    pub fn navigate_up(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
            // Adjust scroll if needed
            if self.selected_index < self.scroll_offset {
                self.scroll_offset = self.selected_index;
            }
            self.needs_render = true;
        }
    }
    
    pub fn navigate_down(&mut self) {
        if self.selected_index < self.search_results.len().saturating_sub(1) {
            self.selected_index += 1;
            // Adjust scroll if needed
            let visible_height = 20; // Approximate visible items
            if self.selected_index >= self.scroll_offset + visible_height {
                self.scroll_offset = self.selected_index - visible_height + 1;
            }
            self.needs_render = true;
        }
    }
    
    pub fn set_status(&mut self, message: String) {
        self.status_message = Some(message);
        self.needs_render = true;
    }
    
    pub fn clear_status(&mut self) {
        self.status_message = None;
        self.needs_render = true;
    }
    
    pub fn handle_ctrl_c(&mut self) -> bool {
        let now = std::time::Instant::now();
        
        // Check if this is a consecutive Ctrl+C (within 500ms)
        if let Some(last_time) = self.last_ctrl_c_time {
            if now.duration_since(last_time).as_millis() < 500 {
                // Second Ctrl+C within 500ms, exit
                return true;
            }
        }
        
        // First Ctrl+C or too much time has passed
        self.ctrl_c_count = 1;
        self.last_ctrl_c_time = Some(now);
        self.set_status("Press Ctrl+C again to exit".to_string());
        
        false
    }
}