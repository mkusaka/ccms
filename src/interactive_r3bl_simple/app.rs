use anyhow::Result;
use std::sync::Arc;
use tokio::sync::{Mutex, mpsc};
use tokio::time::Duration;

use crate::{SearchOptions, SearchEngine, parse_query};
use super::state::{AppState, ViewMode, SearchSignal};
use super::utils::truncate_str;

pub struct SearchApp {
    file_pattern: String,
    options: SearchOptions,
    _state: Arc<Mutex<AppState>>,
    #[cfg(test)]
    pub search_tx: mpsc::Sender<SearchSignal>,
    #[cfg(not(test))]
    search_tx: mpsc::Sender<SearchSignal>,
    search_rx: mpsc::Receiver<SearchSignal>,
    debounce_handle: Option<tokio::task::JoinHandle<()>>,
    previous_render: Arc<Mutex<String>>,
}

impl SearchApp {
    pub fn new(file_pattern: String, options: SearchOptions, state: Arc<Mutex<AppState>>) -> Self {
        let (search_tx, search_rx) = mpsc::channel(100);
        
        Self {
            file_pattern,
            options,
            _state: state,
            search_tx,
            search_rx,
            debounce_handle: None,
            previous_render: Arc::new(Mutex::new(String::new())),
        }
    }
    
    pub async fn render(&self, state: &mut AppState) -> Result<String> {
        let mut buffer = String::new();
        
        // Render to buffer first
        match state.current_mode {
            ViewMode::Search => self.render_search_view(&mut buffer, state).await?,
            ViewMode::ResultDetail => self.render_detail_view(&mut buffer, state).await?,
            ViewMode::Help => self.render_help_view(&mut buffer, state).await?,
        }
        
        // Compare with previous render
        let mut previous = self.previous_render.lock().await;
        if *previous == buffer {
            // No changes, return empty string
            return Ok(String::new());
        }
        
        // Store new render
        *previous = buffer.clone();
        
        // Build complete output with cursor management
        let mut output = String::new();
        output.push_str("\x1b[?25l");  // Hide cursor
        output.push_str("\x1b[H");      // Move to home
        output.push_str(&buffer);       // Add rendered content
        output.push_str("\x1b[?25h");  // Show cursor
        
        Ok(output)
    }
    
    async fn render_search_view(&self, output: &mut String, state: &AppState) -> Result<()> {
        // Get terminal size
        let (width, height) = crossterm::terminal::size().unwrap_or((80, 24));
        let width = width as usize;
        let height = height as usize;
        
        // Create a screen buffer (r3bl_tui inspired approach)
        let mut screen_lines = vec![String::new(); height];
        let mut current_line = 0;
        
        // Title
        if current_line < height {
            screen_lines[current_line] = "\x1b[1mCCMS Search (R3BL TUI)\x1b[0m".to_string();
        }
        current_line += 2;
        
        // Search bar
        if current_line < height {
            let mut search_line = String::from("Search: \x1b[33m");
            search_line.push_str(&state.query);
            if state.is_searching {
                search_line.push_str(" [searching...]");
            }
            search_line.push_str("\x1b[0m");
            screen_lines[current_line] = search_line;
        }
        current_line += 2;
        
        // Results count
        if current_line < height {
            screen_lines[current_line] = format!("Results: {} found", state.search_results.len());
        }
        current_line += 2;
        
        // Results list
        let visible_height = height.saturating_sub(8); // Leave room for header and footer
        let visible_results = state.search_results
            .iter()
            .skip(state.scroll_offset)
            .take(visible_height)
            .enumerate();
        
        for (idx, result) in visible_results {
            if current_line >= height - 1 {
                break;
            }
            
            let is_selected = state.scroll_offset + idx == state.selected_index;
            
            // Format the message
            let first_line = result.text
                .lines()
                .next()
                .map(|line| truncate_str(line, width.saturating_sub(15)))
                .unwrap_or_else(|| "[No content]".to_string());
            
            let mut line_content = format!(
                "{:>2} [{}] {}",
                state.scroll_offset + idx + 1,
                truncate_str(&result.role, 10),
                first_line
            );
            
            if is_selected {
                line_content = format!("\x1b[7m{}\x1b[0m", line_content);
            }
            
            screen_lines[current_line] = line_content;
            current_line += 1;
        }
        
        // Status bar at bottom
        if height > 0 {
            let status_bar = if let Some(status) = &state.status_message {
                format!("\x1b[90m{}\x1b[0m", truncate_str(status, width - 1))
            } else {
                "\x1b[90m↑/↓: Navigate | Enter: View | ?: Help | q: Quit\x1b[0m".to_string()
            };
            screen_lines[height - 1] = status_bar;
        }
        
        // Render the screen buffer
        for (line_num, line) in screen_lines.iter().enumerate() {
            output.push_str(&format!("\x1b[{};1H\x1b[K{}", line_num + 1, line));
        }
        
        // Position cursor at the end of search query
        let cursor_col = 9 + state.query.chars().count(); // "Search: " is 8 chars + 1
        output.push_str(&format!("\x1b[3;{cursor_col}H")); // Line 3, after query
        
        Ok(())
    }
    
    async fn render_detail_view(&self, output: &mut String, state: &AppState) -> Result<()> {
        // Get terminal size
        let (width, height) = crossterm::terminal::size().unwrap_or((80, 24));
        let width = width as usize;
        let height = height as usize;
        
        let mut current_line = 1;
        
        // Title
        output.push_str(&format!("\x1b[{current_line};1H"));
        output.push_str("\x1b[K");
        output.push_str("\x1b[1mMessage Detail\x1b[0m");
        current_line += 2;
        
        if let Some(result) = state.get_selected_result() {
            // Role
            output.push_str(&format!("\x1b[{current_line};1H"));
            output.push_str("\x1b[K");
            output.push_str(&format!("Role: {}", result.role));
            current_line += 1;
            
            // Timestamp
            output.push_str(&format!("\x1b[{current_line};1H"));
            output.push_str("\x1b[K");
            output.push_str(&format!("Timestamp: {}", result.timestamp));
            current_line += 1;
            
            // Session
            output.push_str(&format!("\x1b[{current_line};1H"));
            output.push_str("\x1b[K");
            output.push_str(&format!("Session: {}", truncate_str(&result.session_id, width - 10)));
            current_line += 2;
            
            // Content
            output.push_str(&format!("\x1b[{current_line};1H"));
            output.push_str("\x1b[K");
            output.push_str("\x1b[1mContent:\x1b[0m");
            current_line += 1;
            
            // Display content with scrolling
            let content_height = height.saturating_sub(current_line + 2);
            for line in result.text.lines().skip(state.scroll_offset).take(content_height) {
                output.push_str(&format!("\x1b[{current_line};1H"));
                output.push_str("\x1b[K");
                output.push_str(&truncate_str(line, width - 1));
                current_line += 1;
                if current_line >= height - 1 {
                    break;
                }
            }
        }
        
        // Clear remaining lines
        while current_line < height - 1 {
            output.push_str(&format!("\x1b[{current_line};1H\x1b[K"));
            current_line += 1;
        }
        
        // Help
        output.push_str(&format!("\x1b[{height};1H"));
        output.push_str("\x1b[K");
        output.push_str("\x1b[90m"); // Gray
        output.push_str("Esc: Back to search | q: Quit");
        output.push_str("\x1b[0m");
        
        Ok(())
    }
    
    async fn render_help_view(&self, output: &mut String, _state: &AppState) -> Result<()> {
        // Get terminal size
        let (_, height) = crossterm::terminal::size().unwrap_or((80, 24));
        let height = height as usize;
        
        let mut current_line = 1;
        
        // Title
        output.push_str(&format!("\x1b[{current_line};1H"));
        output.push_str("\x1b[K");
        output.push_str("\x1b[1mCCMS Help\x1b[0m");
        current_line += 2;
        
        let help_lines = vec![
            "Navigation:",
            "  ↑/k      Move up",
            "  ↓/j      Move down", 
            "  Enter    View message details",
            "",
            "Search:",
            "  Type     Enter search query",
            "  Ctrl+U   Clear search",
            "",
            "General:",
            "  ?        Show this help",
            "  Esc      Go back",
            "  q        Quit application",
            "  Ctrl+C   Press twice to exit",
            "",
            "Note: Terminal is in raw mode, so normal text selection/copy is disabled.",
            "      Exit the app (q) to restore normal terminal functionality.",
        ];
        
        for line in help_lines {
            if current_line < height - 2 {
                output.push_str(&format!("\x1b[{current_line};1H"));
                output.push_str("\x1b[K");
                output.push_str(line);
                current_line += 1;
            }
        }
        
        // Clear remaining lines
        while current_line < height - 1 {
            output.push_str(&format!("\x1b[{current_line};1H\x1b[K"));
            current_line += 1;
        }
        
        // Footer
        output.push_str(&format!("\x1b[{height};1H"));
        output.push_str("\x1b[K");
        output.push_str("\x1b[90m"); // Gray
        output.push_str("Press any key to continue");
        output.push_str("\x1b[0m");
        
        Ok(())
    }
    
    pub async fn handle_input(&mut self, key: char, state: &mut AppState) -> Result<bool> {
        match key {
            'q' => {
                return Ok(true); // Exit
            }
            '\x03' => { // Ctrl+C
                if state.handle_ctrl_c() {
                    return Ok(true); // Exit on second Ctrl+C
                }
            }
            '?' => {
                state.current_mode = ViewMode::Help;
                state.needs_render = true;
            }
            '\x1b' => { // ESC
                match state.current_mode {
                    ViewMode::ResultDetail | ViewMode::Help => {
                        state.current_mode = ViewMode::Search;
                        state.needs_render = true;
                    }
                    ViewMode::Search => {}
                }
            }
            '\n' | '\r' => { // Enter
                if state.current_mode == ViewMode::Search && state.get_selected_result().is_some() {
                    state.current_mode = ViewMode::ResultDetail;
                    state.scroll_offset = 0;
                    state.needs_render = true;
                }
            }
            'k' => { // Up
                match state.current_mode {
                    ViewMode::Search => state.navigate_up(),
                    ViewMode::ResultDetail => {
                        if state.scroll_offset > 0 {
                            state.scroll_offset -= 1;
                            state.needs_render = true;
                        }
                    }
                    _ => {}
                }
            }
            'j' => { // Down
                match state.current_mode {
                    ViewMode::Search => state.navigate_down(),
                    ViewMode::ResultDetail => {
                        state.scroll_offset += 1;
                        state.needs_render = true;
                    }
                    _ => {}
                }
            }
            '\x15' => { // Ctrl+U
                if matches!(state.current_mode, ViewMode::Search) {
                    state.query.clear();
                    state.needs_render = true;
                    self.trigger_search(state).await?;
                }
            }
            '\x08' | '\x7f' => { // Backspace
                if matches!(state.current_mode, ViewMode::Search) {
                    state.query.pop();
                    state.needs_render = true;
                    self.trigger_search(state).await?;
                }
            }
            c if c.is_ascii() && !c.is_control() => {
                if matches!(state.current_mode, ViewMode::Search) {
                    state.query.push(c);
                    state.needs_render = true;
                    self.trigger_search(state).await?;
                }
            }
            _ => {}
        }
        
        Ok(false)
    }
    
    async fn trigger_search(&mut self, state: &mut AppState) -> Result<()> {
        // Cancel previous debounce
        if let Some(handle) = self.debounce_handle.take() {
            handle.abort();
        }
        
        state.is_searching = true;
        state.needs_render = true;
        
        let query = state.query.clone();
        let file_pattern = self.file_pattern.clone();
        let options = self.options.clone();
        let tx = self.search_tx.clone();
        
        // Debounce search
        self.debounce_handle = Some(tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(300)).await;
            
            // Perform search
            let engine = SearchEngine::new(options);
            
            let query_condition = match parse_query(&query) {
                Ok(q) => q,
                Err(e) => {
                    let _ = tx.send(SearchSignal::SearchError(format!("Query error: {e}"))).await;
                    return;
                }
            };
            
            match engine.search(&file_pattern, query_condition) {
                Ok((results, _, _)) => {
                    let _ = tx.send(SearchSignal::SearchCompleted(results)).await;
                }
                Err(e) => {
                    let _ = tx.send(SearchSignal::SearchError(format!("Search error: {e}"))).await;
                }
            }
        }));
        
        Ok(())
    }
    
    pub async fn process_signals(&mut self, state: &mut AppState) -> Result<()> {
        while let Ok(signal) = self.search_rx.try_recv() {
            match signal {
                SearchSignal::SearchCompleted(results) => {
                    state.search_results = results;
                    state.is_searching = false;
                    state.selected_index = 0;
                    state.scroll_offset = 0;
                    state.set_status(format!("Found {} results", state.search_results.len()));
                    state.needs_render = true;
                }
                SearchSignal::SearchError(error) => {
                    state.is_searching = false;
                    state.set_status(format!("Error: {error}"));
                    state.needs_render = true;
                }
            }
        }
        
        Ok(())
    }
}