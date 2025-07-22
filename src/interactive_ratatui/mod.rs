#[cfg(test)]
mod tests;

use anyhow::{Context, Result};
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers, poll},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
};
use std::collections::HashMap;
use std::io::{self, Stdout};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::{Duration, SystemTime};

use crate::{SearchOptions, SearchResult, SessionMessage, parse_query};

// Re-use cache structures from the original implementation
struct CachedFile {
    messages: Vec<SessionMessage>,
    raw_lines: Vec<String>,
    last_modified: SystemTime,
}

struct MessageCache {
    files: HashMap<PathBuf, CachedFile>,
}

impl MessageCache {
    fn new() -> Self {
        Self {
            files: HashMap::new(),
        }
    }

    fn get_messages(&mut self, path: &Path) -> Result<&CachedFile> {
        let metadata = std::fs::metadata(path)?;
        let modified = metadata.modified()?;

        let needs_reload = match self.files.get(path) {
            Some(cached) => cached.last_modified != modified,
            None => true,
        };

        if needs_reload {
            let file = std::fs::File::open(path)?;
            let reader = std::io::BufReader::with_capacity(32 * 1024, file);
            use std::io::BufRead;

            let mut messages = Vec::new();
            let mut raw_lines = Vec::new();

            for line in reader.lines() {
                let line = line?;
                if line.trim().is_empty() {
                    continue;
                }

                raw_lines.push(line.clone());

                let mut json_bytes = line.as_bytes().to_vec();
                if let Ok(message) = simd_json::serde::from_slice::<SessionMessage>(&mut json_bytes)
                {
                    messages.push(message);
                }
            }

            self.files.insert(
                path.to_path_buf(),
                CachedFile {
                    messages,
                    raw_lines,
                    last_modified: modified,
                },
            );
        }

        Ok(self.files.get(path).unwrap())
    }

    fn clear(&mut self) {
        self.files.clear();
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Mode {
    Search,
    ResultDetail,
    SessionViewer,
    Help,
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum SessionOrder {
    Ascending,
}

// Search request and response for async communication
#[derive(Clone)]
struct SearchRequest {
    id: u64,
    query: String,
    role_filter: Option<String>,
    pattern: String,
}

struct SearchResponse {
    id: u64,
    results: Vec<SearchResult>,
}

pub struct InteractiveSearch {
    base_options: SearchOptions,
    max_results: usize,
    cache: MessageCache,

    // UI state
    mode: Mode,
    query: String,
    selected_index: usize,
    results: Vec<SearchResult>,
    role_filter: Option<String>,
    message: Option<String>,

    // Session viewer state
    session_messages: Vec<String>,
    session_index: usize,
    session_order: Option<SessionOrder>,
    session_query: String,
    session_filtered_indices: Vec<usize>,
    session_scroll_offset: usize,
    session_selected_index: usize,

    // For result detail
    selected_result: Option<SearchResult>,
    detail_scroll_offset: usize, // Scroll offset for detail view

    // For search results scrolling
    scroll_offset: usize, // Scroll offset for search results list

    // For search performance
    is_searching: bool,

    // Async search support
    search_sender: Option<Sender<SearchRequest>>,
    search_receiver: Option<Receiver<SearchResponse>>,
    current_search_id: Arc<AtomicU64>,
    last_processed_search_id: u64,
}

impl InteractiveSearch {
    pub fn new(options: SearchOptions) -> Self {
        let max_results = options.max_results.unwrap_or(50);
        Self {
            base_options: options,
            max_results,
            cache: MessageCache::new(),
            mode: Mode::Search,
            query: String::new(),
            selected_index: 0,
            results: Vec::new(),
            role_filter: None,
            message: None,
            session_messages: Vec::new(),
            session_index: 0,
            session_order: None,
            session_query: String::new(),
            session_filtered_indices: Vec::new(),
            session_scroll_offset: 0,
            session_selected_index: 0,
            selected_result: None,
            detail_scroll_offset: 0,
            scroll_offset: 0,
            is_searching: false,
            search_sender: None,
            search_receiver: None,
            current_search_id: Arc::new(AtomicU64::new(0)),
            last_processed_search_id: 0,
        }
    }

    pub fn run(&mut self, pattern: &str) -> Result<()> {
        // Initialize async search channel
        let (sender, receiver) = mpsc::channel::<SearchRequest>();
        let (response_sender, response_receiver) = mpsc::channel::<SearchResponse>();
        self.search_sender = Some(sender);
        self.search_receiver = Some(response_receiver);

        // Start search worker thread
        let search_worker_handle = self.start_search_worker(receiver, response_sender, pattern);

        // Load initial results
        self.load_initial_results(pattern);

        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;

        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        let result = self.run_app(&mut terminal, pattern);
        disable_raw_mode()?;
        execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
        terminal.show_cursor()?;

        // Cleanup: drop sender to signal worker to stop
        drop(self.search_sender.take());

        // Wait for worker thread to finish
        let _ = search_worker_handle.join();

        if let Err(e) = result {
            eprintln!("Error: {e}");
            return Err(e);
        }

        Ok(())
    }

    fn run_app(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<Stdout>>,
        pattern: &str,
    ) -> Result<()> {
        loop {
            // Check for search results
            let mut need_scroll_adjust = false;
            if let Some(receiver) = self.search_receiver.as_ref() {
                // Try to receive without blocking
                while let Ok(response) = receiver.try_recv() {
                    // Only process if this is the latest search
                    if response.id > self.last_processed_search_id {
                        self.last_processed_search_id = response.id;
                        self.results = response.results;
                        self.is_searching = false;

                        // Maintain selected index if possible
                        if !self.results.is_empty() {
                            self.selected_index = self.selected_index.min(self.results.len() - 1);
                            need_scroll_adjust = true;
                        } else {
                            self.selected_index = 0;
                            self.scroll_offset = 0;
                        }
                    }
                }
            }

            // Adjust scroll offset if needed (outside of the borrow scope)
            if need_scroll_adjust {
                let (_, height) = crossterm::terminal::size().unwrap_or((80, 24));
                let available_height = height.saturating_sub(7);
                self.adjust_scroll_offset(available_height);
            }

            terminal.draw(|f| self.draw(f))?;

            // No debouncing - search executes immediately on input

            // Non-blocking event polling with 50ms timeout
            if poll(Duration::from_millis(50))? {
                if let Event::Key(key) = event::read()? {
                    match self.mode {
                        Mode::Search => {
                            if !self.handle_search_input(key, pattern)? {
                                break;
                            }
                        }
                        Mode::ResultDetail => {
                            self.handle_result_detail_input(key)?;
                        }
                        Mode::SessionViewer => {
                            self.handle_session_viewer_input(key)?;
                        }
                        Mode::Help => {
                            self.mode = Mode::Search;
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn draw(&mut self, f: &mut Frame) {
        match self.mode {
            Mode::Search => self.draw_search(f),
            Mode::ResultDetail => self.draw_result_detail(f),
            Mode::SessionViewer => self.draw_session_viewer(f),
            Mode::Help => self.draw_help(f),
        }
    }

    fn draw_search(&mut self, f: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Header
                Constraint::Length(3), // Search input
                Constraint::Min(0),    // Results
                Constraint::Length(1), // Status line
            ])
            .split(f.area());

        // Header
        let header = Paragraph::new("Interactive Claude Search")
            .style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::BOTTOM));
        f.render_widget(header, chunks[0]);

        // Search input
        let display_query = &self.query;
        let search_label = if let Some(ref role) = self.role_filter {
            format!("Search [{role}]: {display_query}")
        } else {
            format!("Search: {display_query}")
        };

        let title = if self.is_searching {
            "Query (searching...)"
        } else {
            "Query"
        };

        let input = Paragraph::new(search_label.as_str())
            .style(Style::default())
            .block(Block::default().borders(Borders::ALL).title(title));
        f.render_widget(input, chunks[1]);

        // Results - always show if we have any
        if !self.results.is_empty() {
            self.draw_results(f, chunks[2]);
        } else if !self.query.is_empty() {
            // Show "no results" only if user has typed something
            let no_results = Paragraph::new("No results found")
                .style(Style::default().fg(Color::Yellow))
                .alignment(Alignment::Center)
                .block(Block::default().borders(Borders::ALL).title("Results"));
            f.render_widget(no_results, chunks[2]);
        } else {
            let empty = Paragraph::new("Type to search...")
                .style(Style::default().fg(Color::DarkGray))
                .alignment(Alignment::Center)
                .block(Block::default().borders(Borders::ALL).title("Results"));
            f.render_widget(empty, chunks[2]);
        }

        // Status line
        let status = if let Some(ref msg) = self.message {
            msg.clone()
        } else {
            "Tab: Filter | ↑/↓: Navigate | Enter: Select | Ctrl+R: Reload | Esc: Exit".to_string()
        };
        let status_bar = Paragraph::new(status).style(Style::default().fg(Color::DarkGray));
        f.render_widget(status_bar, chunks[3]);

        // Position cursor
        let cursor_x = chunks[1].x
            + 1
            + if let Some(ref role) = self.role_filter {
                ("Search [".len() + role.len() + "]: ".len()) as u16
            } else {
                "Search: ".len() as u16
            }
            + self.query.len() as u16;
        let cursor_y = chunks[1].y + 1;
        f.set_cursor_position((cursor_x.min(chunks[1].x + chunks[1].width - 2), cursor_y));
    }

    fn draw_results(&mut self, f: &mut Frame, area: Rect) {
        let results_block = Block::default()
            .title(format!("Results ({})", self.results.len()))
            .borders(Borders::ALL);
        let inner = results_block.inner(area);
        f.render_widget(results_block, area);

        if self.results.is_empty() {
            // Don't show "No results found" for empty query
            if !self.query.is_empty() {
                let no_results = Paragraph::new("No results found")
                    .style(Style::default().fg(Color::Yellow))
                    .alignment(Alignment::Center);
                f.render_widget(no_results, inner);
            }
            return;
        }

        // Calculate visible range with scrolling
        let (start_idx, end_idx) = self.calculate_visible_range(inner.height);

        let items: Vec<ListItem> = self
            .results
            .iter()
            .skip(start_idx)
            .take(end_idx - start_idx)
            .enumerate()
            .map(|(idx, result)| {
                let actual_idx = start_idx + idx;
                let timestamp = Self::format_timestamp(&result.timestamp);
                let role_str = format!("[{:^9}]", result.role.to_uppercase());

                // Calculate available width for message
                // Format: "NN. [ROLE     ] MM/DD HH:MM <message>"
                let index_str = format!("{:2}. ", actual_idx + 1);
                let fixed_part = format!("{index_str}{role_str} {timestamp} ");
                let fixed_width = fixed_part.chars().count();

                // Get terminal width and calculate available space for message
                let terminal_width = inner.width as usize;
                let available_width = terminal_width.saturating_sub(fixed_width).saturating_sub(1); // -1 for safety

                // Truncate message to fit
                let truncated_message = self.truncate_message(&result.text, available_width);

                let line_content = format!("{fixed_part}{truncated_message}");

                let style = if actual_idx == self.selected_index {
                    Style::default()
                        .bg(Color::DarkGray)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };

                ListItem::new(Line::from(vec![Span::styled(line_content, style)]))
            })
            .collect();

        let list = List::new(items).highlight_style(Style::default());
        f.render_widget(list, inner);

        // Show scroll indicator
        let visible_count = end_idx - start_idx;
        if self.results.len() > visible_count {
            let scroll_text = if self.results.len() >= self.max_results {
                format!(
                    "Showing {}-{} of {} results (limit reached) ↑/↓ to scroll",
                    start_idx + 1,
                    end_idx,
                    self.results.len()
                )
            } else {
                format!(
                    "Showing {}-{} of {} results ↑/↓ to scroll, PgUp/PgDn for pages",
                    start_idx + 1,
                    end_idx,
                    self.results.len()
                )
            };

            let scroll_indicator = Paragraph::new(scroll_text)
                .style(Style::default().fg(Color::DarkGray))
                .alignment(Alignment::Center);

            let indicator_area = Rect {
                x: inner.x,
                y: inner.y + inner.height.saturating_sub(1),
                width: inner.width,
                height: 1,
            };
            f.render_widget(scroll_indicator, indicator_area);
        }
    }

    fn draw_result_detail(&mut self, f: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(2)
            .constraints([
                Constraint::Min(0),     // Content
                Constraint::Length(10), // Actions
                Constraint::Length(2),  // Status/Message
            ])
            .split(f.area());

        if let Some(ref result) = self.selected_result {
            let timestamp = Self::format_timestamp_long(&result.timestamp);

            let content = vec![
                Line::from(vec![
                    Span::styled("Role: ", Style::default().fg(Color::Yellow)),
                    Span::raw(&result.role),
                ]),
                Line::from(vec![
                    Span::styled("Time: ", Style::default().fg(Color::Yellow)),
                    Span::raw(&timestamp),
                ]),
                Line::from(vec![
                    Span::styled("File: ", Style::default().fg(Color::Yellow)),
                    Span::raw(&result.file),
                ]),
                Line::from(vec![
                    Span::styled("Project: ", Style::default().fg(Color::Yellow)),
                    Span::raw(&result.project_path),
                ]),
                Line::from(vec![
                    Span::styled("UUID: ", Style::default().fg(Color::Yellow)),
                    Span::raw(&result.uuid),
                ]),
                Line::from(vec![
                    Span::styled("Session: ", Style::default().fg(Color::Yellow)),
                    Span::raw(&result.session_id),
                ]),
                Line::from(""),
                Line::from("─".repeat(80)),
                Line::from(""),
            ];

            // Build all lines including the message content
            let header_lines = content.len();
            let mut all_lines = content;

            // Split message text into lines
            for line in result.text.lines() {
                all_lines.push(Line::from(line));
            }

            // Calculate visible area
            let inner_area = Block::default().borders(Borders::ALL).inner(chunks[0]);
            let visible_height = inner_area.height as usize;

            // Apply scroll offset
            let display_lines: Vec<Line> = all_lines
                .into_iter()
                .skip(self.detail_scroll_offset)
                .take(visible_height)
                .collect();

            let detail = Paragraph::new(display_lines).block(
                Block::default().borders(Borders::ALL).title(format!(
                    "Result Detail (↑/↓ or j/k to scroll, line {}/{})",
                    self.detail_scroll_offset + 1,
                    header_lines + result.text.lines().count()
                )),
            );
            f.render_widget(detail, chunks[0]);

            // Actions
            let actions = vec![
                Line::from(vec![Span::styled(
                    "Actions:",
                    Style::default().fg(Color::Cyan),
                )]),
                Line::from(vec![
                    Span::styled("[S]", Style::default().fg(Color::Yellow)),
                    Span::raw(" - View full session"),
                ]),
                Line::from(vec![
                    Span::styled("[F]", Style::default().fg(Color::Yellow)),
                    Span::raw(" - Copy file path"),
                ]),
                Line::from(vec![
                    Span::styled("[I]", Style::default().fg(Color::Yellow)),
                    Span::raw(" - Copy session ID"),
                ]),
                Line::from(vec![
                    Span::styled("[P]", Style::default().fg(Color::Yellow)),
                    Span::raw(" - Copy project path"),
                ]),
                Line::from(vec![
                    Span::styled("[M]", Style::default().fg(Color::Yellow)),
                    Span::raw(" - Copy message text"),
                ]),
                Line::from(vec![
                    Span::styled("[R]", Style::default().fg(Color::Yellow)),
                    Span::raw(" - Copy raw JSON"),
                ]),
                Line::from(vec![
                    Span::styled("[Esc]", Style::default().fg(Color::Yellow)),
                    Span::raw(" - Back to search"),
                ]),
                Line::from(vec![
                    Span::styled("[↑/↓ or j/k]", Style::default().fg(Color::Yellow)),
                    Span::raw(" - Scroll message"),
                ]),
            ];

            let actions_widget =
                Paragraph::new(actions).block(Block::default().borders(Borders::ALL));
            f.render_widget(actions_widget, chunks[1]);

            // Show message if any
            if let Some(ref msg) = self.message {
                let message_widget = Paragraph::new(msg.clone())
                    .style(
                        Style::default()
                            .fg(if msg.starts_with('✓') {
                                Color::Green
                            } else if msg.starts_with('⚠') {
                                Color::Yellow
                            } else {
                                Color::White
                            })
                            .add_modifier(Modifier::BOLD),
                    )
                    .alignment(Alignment::Center);
                f.render_widget(message_widget, chunks[2]);

                // Clear message after 2 seconds (will be cleared on next keypress)
                // For now, it stays until next action
            }
        }
    }

    fn draw_session_viewer(&mut self, f: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(5), // Header
                Constraint::Length(3), // Search box
                Constraint::Min(0),    // Message list
                Constraint::Length(2), // Status
            ])
            .split(f.area());

        // Header
        if let Some(ref result) = self.selected_result {
            let header_text = vec![
                Line::from(vec![Span::styled(
                    "Session Viewer",
                    Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                )]),
                Line::from(vec![
                    Span::styled("Session: ", Style::default().fg(Color::Yellow)),
                    Span::raw(&result.session_id),
                ]),
                Line::from(vec![
                    Span::styled("File: ", Style::default().fg(Color::Yellow)),
                    Span::raw(&result.file),
                ]),
            ];

            let header =
                Paragraph::new(header_text).block(Block::default().borders(Borders::BOTTOM));
            f.render_widget(header, chunks[0]);
        }

        // Search box
        let search_label = format!("Filter: {}", self.session_query);
        let search_box = Paragraph::new(search_label.as_str())
            .style(Style::default())
            .block(Block::default().borders(Borders::ALL).title("Search"));
        f.render_widget(search_box, chunks[1]);

        // Message list
        if !self.session_messages.is_empty() {
            self.draw_session_message_list(f, chunks[2]);
        } else {
            let empty_msg = Paragraph::new("No messages in session")
                .style(Style::default().fg(Color::Yellow))
                .alignment(Alignment::Center)
                .block(Block::default().borders(Borders::ALL));
            f.render_widget(empty_msg, chunks[2]);
        }

        // Status bar
        let status = if self.session_messages.is_empty() {
            "No messages | Q: Quit"
        } else {
            "Enter: View | ↑/↓: Navigate | /: Search | Esc: Clear search | Q: Back"
        };
        let status_bar = Paragraph::new(status)
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center);
        f.render_widget(status_bar, chunks[3]);

        // Position cursor in search box if typing
        if !self.session_query.is_empty() || self.mode == Mode::SessionViewer {
            let cursor_x = chunks[1].x + 1 + "Filter: ".len() as u16 + self.session_query.len() as u16;
            let cursor_y = chunks[1].y + 1;
            f.set_cursor_position((cursor_x.min(chunks[1].x + chunks[1].width - 2), cursor_y));
        }
    }

    fn draw_session_message_list(&mut self, f: &mut Frame, area: Rect) {
        // First, filter messages if there's a search query
        if self.session_query.is_empty() {
            // No filter, show all messages
            self.session_filtered_indices = (0..self.session_messages.len()).collect();
        } else {
            // Filter messages based on search query
            self.session_filtered_indices = self.session_messages
                .iter()
                .enumerate()
                .filter_map(|(idx, msg)| {
                    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(msg) {
                        // Extract text content
                        let content = parsed
                            .get("message")
                            .and_then(|m| m.get("content"))
                            .or_else(|| parsed.get("content"));
                        
                        if let Some(content_val) = content {
                            let text = if let Some(text_str) = content_val.as_str() {
                                text_str.to_lowercase()
                            } else if let Some(parts) = content_val.as_array() {
                                parts.iter()
                                    .filter_map(|part| part.get("text").and_then(|v| v.as_str()))
                                    .collect::<Vec<_>>()
                                    .join(" ")
                                    .to_lowercase()
                            } else {
                                String::new()
                            };
                            
                            if text.contains(&self.session_query.to_lowercase()) {
                                return Some(idx);
                            }
                        }
                    }
                    None
                })
                .collect();
        }

        let filtered_count = self.session_filtered_indices.len();
        let title = if self.session_query.is_empty() {
            format!("Messages ({} total)", self.session_messages.len())
        } else {
            format!("Messages ({} total, {} filtered)", self.session_messages.len(), filtered_count)
        };

        let messages_block = Block::default()
            .title(title)
            .borders(Borders::ALL);
        let inner = messages_block.inner(area);
        f.render_widget(messages_block, area);

        if filtered_count == 0 {
            let no_results = Paragraph::new("No messages match filter")
                .style(Style::default().fg(Color::Yellow))
                .alignment(Alignment::Center);
            f.render_widget(no_results, inner);
            return;
        }

        // Calculate visible range
        let visible_height = inner.height as usize;
        let start_idx = self.session_scroll_offset;
        let end_idx = (start_idx + visible_height).min(filtered_count);

        // Build list items
        let items: Vec<ListItem> = self.session_filtered_indices[start_idx..end_idx]
            .iter()
            .enumerate()
            .map(|(display_idx, &actual_idx)| {
                let list_idx = start_idx + display_idx;
                if let Ok(msg) = serde_json::from_str::<serde_json::Value>(&self.session_messages[actual_idx]) {
                    let role = msg.get("type").and_then(|v| v.as_str()).unwrap_or("unknown");
                    let timestamp = msg.get("timestamp").and_then(|v| v.as_str()).unwrap_or("");
                    
                    // Extract message preview
                    let content = msg
                        .get("message")
                        .and_then(|m| m.get("content"))
                        .or_else(|| msg.get("content"));
                    
                    let preview = if let Some(content_val) = content {
                        if let Some(text) = content_val.as_str() {
                            text.replace('\n', " ")
                        } else if let Some(parts) = content_val.as_array() {
                            parts.iter()
                                .filter_map(|part| part.get("text").and_then(|v| v.as_str()))
                                .collect::<Vec<_>>()
                                .join(" ")
                        } else {
                            "(no content)".to_string()
                        }
                    } else {
                        "(no content)".to_string()
                    };

                    // Format timestamp
                    let short_time = if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(timestamp) {
                        dt.format("%m/%d %H:%M").to_string()
                    } else {
                        timestamp.to_string()
                    };

                    // Format line
                    let index_str = format!("{:3}. ", actual_idx + 1);
                    let role_str = format!("[{:^9}]", role.to_uppercase());
                    let fixed_part = format!("{index_str}{role_str} {short_time} ");
                    
                    // Calculate available width for preview
                    let available_width = inner.width.saturating_sub(fixed_part.len() as u16).saturating_sub(1);
                    let truncated_preview = self.truncate_message(&preview, available_width as usize);
                    
                    let line_content = format!("{fixed_part}{truncated_preview}");

                    let style = if list_idx == self.session_selected_index {
                        Style::default()
                            .bg(Color::DarkGray)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default()
                    };

                    ListItem::new(Line::from(vec![Span::styled(line_content, style)]))
                } else {
                    ListItem::new(Line::from("(error parsing message)"))
                }
            })
            .collect();

        let list = List::new(items);
        f.render_widget(list, inner);

        // Show scroll indicator if needed
        if filtered_count > visible_height {
            let scroll_text = format!(
                "Showing {}-{} of {} messages ↑/↓ to scroll",
                start_idx + 1,
                end_idx,
                filtered_count
            );

            let scroll_indicator = Paragraph::new(scroll_text)
                .style(Style::default().fg(Color::DarkGray))
                .alignment(Alignment::Center);

            let indicator_area = Rect {
                x: inner.x,
                y: inner.y + inner.height.saturating_sub(1),
                width: inner.width,
                height: 1,
            };

            f.render_widget(scroll_indicator, indicator_area);
        }
    }

    fn draw_help(&mut self, f: &mut Frame) {
        let help_text = vec![
            Line::from(vec![Span::styled(
                "CCMS Help",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from(""),
            Line::from(vec![Span::styled(
                "Search Mode:",
                Style::default().fg(Color::Yellow),
            )]),
            Line::from("  Type        - Search for text"),
            Line::from("  Tab         - Cycle role filter"),
            Line::from("  ↑/↓         - Navigate results"),
            Line::from("  Enter       - View result detail"),
            Line::from("  Ctrl+R      - Clear cache & reload"),
            Line::from("  Esc         - Exit"),
            Line::from(""),
            Line::from(vec![Span::styled(
                "Result Detail:",
                Style::default().fg(Color::Yellow),
            )]),
            Line::from("  S           - View full session"),
            Line::from("  F/I/P/M/R   - Copy to clipboard"),
            Line::from("  ↑/↓ or j/k  - Scroll content"),
            Line::from("  Esc         - Back to search"),
            Line::from(""),
            Line::from(vec![Span::styled(
                "Session Viewer:",
                Style::default().fg(Color::Yellow),
            )]),
            Line::from("  Type        - Filter messages"),
            Line::from("  /           - Start search"),
            Line::from("  ↑/↓         - Navigate messages"),
            Line::from("  Enter       - View message detail"),
            Line::from("  Esc         - Clear search/Go back"),
            Line::from("  Q           - Back to result detail"),
            Line::from(""),
            Line::from(vec![Span::styled(
                "Query Syntax:",
                Style::default().fg(Color::Yellow),
            )]),
            Line::from("  word        - Search for word"),
            Line::from("  \"phrase\"    - Search for exact phrase"),
            Line::from("  AND/OR/NOT  - Boolean operators"),
            Line::from("  /regex/i    - Regular expression"),
            Line::from("  ()          - Grouping"),
            Line::from(""),
            Line::from("Press any key to return..."),
        ];

        let help = Paragraph::new(help_text)
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: false })
            .block(Block::default().borders(Borders::ALL).title("Help"));

        let area = self.centered_rect(60, 80, f.area());
        f.render_widget(Clear, area);
        f.render_widget(help, area);
    }

    fn centered_rect(&self, percent_x: u16, percent_y: u16, area: Rect) -> Rect {
        let popup_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage((100 - percent_y) / 2),
                Constraint::Percentage(percent_y),
                Constraint::Percentage((100 - percent_y) / 2),
            ])
            .split(area);

        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage((100 - percent_x) / 2),
                Constraint::Percentage(percent_x),
                Constraint::Percentage((100 - percent_x) / 2),
            ])
            .split(popup_layout[1])[1]
    }

    fn truncate_message(&self, text: &str, max_width: usize) -> String {
        if max_width == 0 {
            return String::new();
        }

        let cleaned = text.replace('\n', " ");

        if cleaned.chars().count() <= max_width {
            cleaned
        } else if max_width <= 3 {
            // Not enough room for ellipsis
            cleaned.chars().take(max_width).collect()
        } else {
            // Leave room for "..."
            let truncate_at = max_width.saturating_sub(3);
            let truncated: String = cleaned.chars().take(truncate_at).collect();
            format!("{truncated}...")
        }
    }

    fn calculate_visible_range(&self, available_height: u16) -> (usize, usize) {
        // Reserve 1 line for scroll indicator if needed
        let height_for_items = if self.results.len() > available_height as usize {
            available_height.saturating_sub(1)
        } else {
            available_height
        };

        let visible_count = (height_for_items as usize).min(self.results.len());
        let start = self.scroll_offset;
        let end = (start + visible_count).min(self.results.len());
        (start, end)
    }

    fn adjust_scroll_offset(&mut self, available_height: u16) {
        // Reserve 1 line for scroll indicator if needed
        let height_for_items = if self.results.len() > available_height as usize {
            available_height.saturating_sub(1)
        } else {
            available_height
        };

        let visible_count = (height_for_items as usize).min(self.results.len());

        // If selected index is above the visible range, scroll up
        if self.selected_index < self.scroll_offset {
            self.scroll_offset = self.selected_index;
        }
        // If selected index is below the visible range, scroll down
        else if self.selected_index >= self.scroll_offset + visible_count {
            self.scroll_offset = self.selected_index.saturating_sub(visible_count - 1);
        }
    }

    fn handle_search_input(&mut self, key: KeyEvent, pattern: &str) -> Result<bool> {
        match key.code {
            KeyCode::Esc => {
                return Ok(false);
            }
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                return Ok(false);
            }
            KeyCode::Char('r') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.cache.clear();
                self.execute_search(pattern);
                self.message = Some("Cache cleared and reloaded".to_string());
            }
            KeyCode::Char('?') => {
                self.mode = Mode::Help;
            }
            KeyCode::Tab => {
                self.role_filter = match self.role_filter {
                    None => Some("user".to_string()),
                    Some(ref r) if r == "user" => Some("assistant".to_string()),
                    Some(ref r) if r == "assistant" => Some("system".to_string()),
                    Some(ref r) if r == "system" => Some("summary".to_string()),
                    Some(ref r) if r == "summary" => None,
                    _ => None,
                };
                self.selected_index = 0;
                self.scroll_offset = 0;
                self.execute_search(pattern);
                self.message = None; // Clear any message when changing role filter
            }
            KeyCode::Char(c) => {
                self.query.push(c);
                self.execute_search(pattern);
                // Don't reset selection - it will be adjusted when results arrive
            }
            KeyCode::Backspace => {
                self.query.pop();
                self.execute_search(pattern);
                // Don't reset selection - it will be adjusted when results arrive
            }
            KeyCode::Up => {
                if self.selected_index > 0 {
                    self.selected_index -= 1;
                    // Get actual terminal size
                    let (_, height) = crossterm::terminal::size().unwrap_or((80, 24));
                    // Account for UI chrome (headers, borders, etc.)
                    let available_height = height.saturating_sub(7);
                    self.adjust_scroll_offset(available_height);
                }
            }
            KeyCode::Down => {
                if self.selected_index < self.results.len().saturating_sub(1) {
                    self.selected_index += 1;
                    // Get actual terminal size
                    let (_, height) = crossterm::terminal::size().unwrap_or((80, 24));
                    let available_height = height.saturating_sub(7);
                    self.adjust_scroll_offset(available_height);
                }
            }
            KeyCode::PageDown => {
                let page_size = 10;
                self.selected_index =
                    (self.selected_index + page_size).min(self.results.len().saturating_sub(1));
                let (_, height) = crossterm::terminal::size().unwrap_or((80, 24));
                let available_height = height.saturating_sub(7);
                self.adjust_scroll_offset(available_height);
            }
            KeyCode::PageUp => {
                let page_size = 10;
                self.selected_index = self.selected_index.saturating_sub(page_size);
                let (_, height) = crossterm::terminal::size().unwrap_or((80, 24));
                let available_height = height.saturating_sub(7);
                self.adjust_scroll_offset(available_height);
            }
            KeyCode::Home => {
                self.selected_index = 0;
                self.scroll_offset = 0;
            }
            KeyCode::End => {
                if !self.results.is_empty() {
                    self.selected_index = self.results.len() - 1;
                    let (_, height) = crossterm::terminal::size().unwrap_or((80, 24));
                    let available_height = height.saturating_sub(7);
                    self.adjust_scroll_offset(available_height);
                }
            }
            KeyCode::Enter => {
                if !self.results.is_empty() && self.selected_index < self.results.len() {
                    self.selected_result = Some(self.results[self.selected_index].clone());
                    self.mode = Mode::ResultDetail;
                    self.detail_scroll_offset = 0; // Reset scroll when entering detail
                    self.message = None; // Clear any previous message
                }
            }
            _ => {}
        }
        Ok(true)
    }

    fn handle_result_detail_input(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Esc => {
                self.mode = Mode::Search;
                self.message = None; // Clear message when returning to search
                self.detail_scroll_offset = 0;
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.detail_scroll_offset = self.detail_scroll_offset.saturating_sub(1);
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if let Some(ref result) = self.selected_result {
                    let total_lines = 9 + result.text.lines().count(); // 9 header lines
                    self.detail_scroll_offset = self
                        .detail_scroll_offset
                        .saturating_add(1)
                        .min(total_lines.saturating_sub(10)); // Keep some lines visible
                }
            }
            KeyCode::PageUp => {
                self.detail_scroll_offset = self.detail_scroll_offset.saturating_sub(10);
            }
            KeyCode::PageDown => {
                if let Some(ref result) = self.selected_result {
                    let total_lines = 9 + result.text.lines().count();
                    self.detail_scroll_offset = self
                        .detail_scroll_offset
                        .saturating_add(10)
                        .min(total_lines.saturating_sub(10));
                }
            }
            KeyCode::Char('s') | KeyCode::Char('S') => {
                if let Some(result) = &self.selected_result {
                    // Try to find the full file path from search results
                    let file_path = if let Some(matching_result) = self
                        .results
                        .iter()
                        .find(|r| r.uuid == result.uuid && r.session_id == result.session_id)
                    {
                        // Use the file path from the matching result
                        matching_result.file.clone()
                    } else {
                        // Fallback to the stored file name
                        result.file.clone()
                    };

                    // Search for the actual file in the default pattern
                    use crate::search::discover_claude_files;
                    let files = discover_claude_files(None).unwrap_or_default();

                    // Find the file that matches our session
                    let full_path = files
                        .iter()
                        .find(|f| f.to_string_lossy().contains(&result.session_id))
                        .map(|f| f.to_string_lossy().to_string())
                        .unwrap_or(file_path);

                    match self.load_session_messages(&full_path) {
                        Ok(_) => {
                            self.session_index = 0;
                            self.session_order = Some(SessionOrder::Ascending); // Default to ascending
                            self.session_query.clear();
                            self.session_filtered_indices = (0..self.session_messages.len()).collect();
                            self.session_scroll_offset = 0;
                            self.session_selected_index = 0;
                            self.mode = Mode::SessionViewer;
                        }
                        Err(e) => {
                            self.message = Some(format!("⚠ Failed to load session: {e}"));
                        }
                    }
                }
            }
            KeyCode::Char('f') | KeyCode::Char('F') => {
                if let Some(ref result) = self.selected_result {
                    self.copy_to_clipboard(&result.file)?;
                    self.message = Some("✓ File path copied to clipboard!".to_string());
                    // Stay in detail view
                }
            }
            KeyCode::Char('i') | KeyCode::Char('I') => {
                if let Some(ref result) = self.selected_result {
                    self.copy_to_clipboard(&result.session_id)?;
                    self.message = Some("✓ Session ID copied to clipboard!".to_string());
                    // Stay in detail view
                }
            }
            KeyCode::Char('p') | KeyCode::Char('P') => {
                if let Some(ref result) = self.selected_result {
                    self.copy_to_clipboard(&result.project_path)?;
                    self.message = Some("✓ Project path copied to clipboard!".to_string());
                    // Stay in detail view
                }
            }
            KeyCode::Char('m') | KeyCode::Char('M') => {
                if let Some(ref result) = self.selected_result {
                    self.copy_to_clipboard(&result.text)?;
                    self.message = Some("✓ Message text copied to clipboard!".to_string());
                    // Stay in detail view
                }
            }
            KeyCode::Char('r') | KeyCode::Char('R') => {
                if let Some(ref result) = self.selected_result {
                    if let Some(ref raw_json) = result.raw_json {
                        self.copy_to_clipboard(raw_json)?;
                        self.message = Some("✓ Raw JSON copied to clipboard!".to_string());
                    } else {
                        self.message = Some("⚠ No raw JSON available".to_string());
                    }
                    // Stay in detail view
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_session_viewer_input(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Char('q') | KeyCode::Char('Q') => {
                self.mode = Mode::ResultDetail;
                // Clear session viewer state
                self.session_messages.clear();
                self.session_query.clear();
                self.session_filtered_indices.clear();
                self.session_scroll_offset = 0;
                self.session_selected_index = 0;
            }
            KeyCode::Esc => {
                if !self.session_query.is_empty() {
                    // Clear search if active
                    self.session_query.clear();
                    self.session_selected_index = 0;
                    self.session_scroll_offset = 0;
                } else {
                    // Go back to result detail
                    self.mode = Mode::ResultDetail;
                    self.session_messages.clear();
                    self.session_query.clear();
                    self.session_filtered_indices.clear();
                    self.session_scroll_offset = 0;
                    self.session_selected_index = 0;
                }
            }
            KeyCode::Char('/') => {
                // Start search mode - cursor will be positioned automatically
            }
            KeyCode::Char(c) => {
                self.session_query.push(c);
                self.session_selected_index = 0;
                self.session_scroll_offset = 0;
            }
            KeyCode::Backspace => {
                self.session_query.pop();
                self.session_selected_index = 0;
                self.session_scroll_offset = 0;
            }
            KeyCode::Up => {
                if !self.session_filtered_indices.is_empty()
                    && self.session_selected_index > 0 {
                        self.session_selected_index -= 1;
                        self.adjust_session_scroll_offset();
                    }
            }
            KeyCode::Down => {
                if !self.session_filtered_indices.is_empty() {
                    let max_index = self.session_filtered_indices.len().saturating_sub(1);
                    if self.session_selected_index < max_index {
                        self.session_selected_index += 1;
                        self.adjust_session_scroll_offset();
                    }
                }
            }
            KeyCode::PageUp => {
                if !self.session_filtered_indices.is_empty() {
                    self.session_selected_index = self.session_selected_index.saturating_sub(10);
                    self.adjust_session_scroll_offset();
                }
            }
            KeyCode::PageDown => {
                if !self.session_filtered_indices.is_empty() {
                    let max_index = self.session_filtered_indices.len().saturating_sub(1);
                    self.session_selected_index = (self.session_selected_index + 10).min(max_index);
                    self.adjust_session_scroll_offset();
                }
            }
            KeyCode::Enter => {
                // View selected message in detail
                if !self.session_filtered_indices.is_empty() {
                    let actual_idx = self.session_filtered_indices[self.session_selected_index];
                    if let Ok(msg) = serde_json::from_str::<serde_json::Value>(&self.session_messages[actual_idx]) {
                        // Create a pseudo SearchResult for detail view
                        let role = msg.get("type").and_then(|v| v.as_str()).unwrap_or("unknown");
                        let timestamp = msg.get("timestamp").and_then(|v| v.as_str()).unwrap_or("");
                        let uuid = msg.get("uuid").and_then(|v| v.as_str()).unwrap_or("");
                        
                        let content = msg
                            .get("message")
                            .and_then(|m| m.get("content"))
                            .or_else(|| msg.get("content"));
                        
                        let text = if let Some(content_val) = content {
                            if let Some(text) = content_val.as_str() {
                                text.to_string()
                            } else if let Some(parts) = content_val.as_array() {
                                parts.iter()
                                    .filter_map(|part| part.get("text").and_then(|v| v.as_str()))
                                    .collect::<Vec<_>>()
                                    .join("\n")
                            } else {
                                "(no content)".to_string()
                            }
                        } else {
                            "(no content)".to_string()
                        };

                        if let Some(ref mut result) = self.selected_result {
                            // Update the selected result with this message's details
                            result.role = role.to_string();
                            result.timestamp = timestamp.to_string();
                            result.text = text;
                            result.uuid = uuid.to_string();
                            result.raw_json = Some(self.session_messages[actual_idx].clone());
                        }
                        
                        self.mode = Mode::ResultDetail;
                        self.detail_scroll_offset = 0;
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn adjust_session_scroll_offset(&mut self) {
        // This will be called after updating session_selected_index
        // to ensure the selected item is visible
        // We'll implement this similar to adjust_scroll_offset but for session viewer
        let visible_height = 20; // Approximate visible height, will be calculated properly in draw
        
        if self.session_selected_index < self.session_scroll_offset {
            self.session_scroll_offset = self.session_selected_index;
        } else if self.session_selected_index >= self.session_scroll_offset + visible_height {
            self.session_scroll_offset = self.session_selected_index.saturating_sub(visible_height - 1);
        }
    }

    fn execute_search(&mut self, pattern: &str) {
        if self.query.is_empty() {
            self.results.clear();
            self.is_searching = false;
            return;
        }

        // Send search request to worker thread
        if let Some(ref sender) = self.search_sender {
            let search_id = self.current_search_id.fetch_add(1, Ordering::SeqCst);
            let request = SearchRequest {
                id: search_id,
                query: self.query.clone(),
                role_filter: self.role_filter.clone(),
                pattern: pattern.to_string(),
            };

            // Mark as searching before sending request
            self.is_searching = true;

            // Send request, ignore if channel is disconnected
            let _ = sender.send(request);
        }
    }

    #[allow(dead_code)]
    fn execute_cached_search(
        &mut self,
        pattern: &str,
        query: &crate::query::QueryCondition,
        role_filter: &Option<String>,
    ) -> Result<Vec<SearchResult>> {
        use crate::search::{discover_claude_files, expand_tilde};

        let expanded_pattern = expand_tilde(pattern);
        let files = if expanded_pattern.is_file() {
            vec![expanded_pattern]
        } else {
            discover_claude_files(Some(pattern))?
        };

        let mut results = Vec::new();

        for file_path in &files {
            let cached_file = self.cache.get_messages(file_path)?;
            let file_name = file_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
                .to_string();

            for (idx, message) in cached_file.messages.iter().enumerate() {
                let text = message.get_content_text();

                if let Ok(matches) = query.evaluate(&text) {
                    if matches {
                        if let Some(role) = role_filter {
                            if message.get_type() != role {
                                continue;
                            }
                        }

                        if let Some(session_id) = &self.base_options.session_id {
                            if message.get_session_id() != Some(session_id) {
                                continue;
                            }
                        }

                        let timestamp = message.get_timestamp().unwrap_or("").to_string();

                        results.push(SearchResult {
                            file: file_name.clone(),
                            uuid: message.get_uuid().unwrap_or("").to_string(),
                            timestamp,
                            session_id: message.get_session_id().unwrap_or("").to_string(),
                            role: message.get_type().to_string(),
                            text: text.clone(),
                            has_tools: message.has_tool_use(),
                            has_thinking: message.has_thinking(),
                            message_type: message.get_type().to_string(),
                            query: query.clone(),
                            project_path: Self::extract_project_path(file_path),
                            raw_json: Some(cached_file.raw_lines[idx].clone()),
                        });
                    }
                }
            }
        }

        self.apply_filters(&mut results)?;
        results.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        results.truncate(self.max_results);

        Ok(results)
    }

    fn extract_project_path(file_path: &Path) -> String {
        // Try to extract project path from ~/.claude/projects/encoded-path/session.jsonl
        let path_str = file_path.to_string_lossy();
        if let Some(projects_idx) = path_str.find("/.claude/projects/") {
            let after_projects = &path_str[projects_idx + "/.claude/projects/".len()..];
            if let Some(slash_idx) = after_projects.find('/') {
                let encoded_path = &after_projects[..slash_idx];
                return encoded_path.replace('-', "/");
            }
        }

        // Fallback to parent directory
        if let Some(parent) = file_path.parent() {
            parent.to_string_lossy().to_string()
        } else {
            file_path.to_string_lossy().to_string()
        }
    }

    fn apply_filters(&self, results: &mut Vec<SearchResult>) -> Result<()> {
        use chrono::DateTime;

        if let Some(before) = &self.base_options.before {
            let before_time =
                DateTime::parse_from_rfc3339(before).context("Invalid 'before' timestamp")?;
            results.retain(|r| {
                if let Ok(time) = DateTime::parse_from_rfc3339(&r.timestamp) {
                    time < before_time
                } else {
                    false
                }
            });
        }

        if let Some(after) = &self.base_options.after {
            let after_time =
                DateTime::parse_from_rfc3339(after).context("Invalid 'after' timestamp")?;
            results.retain(|r| {
                if let Ok(time) = DateTime::parse_from_rfc3339(&r.timestamp) {
                    time > after_time
                } else {
                    false
                }
            });
        }

        Ok(())
    }

    fn load_initial_results(&mut self, pattern: &str) {
        // Load all messages without any query filter
        // Use a query that matches everything
        match self.load_all_messages(pattern, &None) {
            Ok(mut results) => {
                // Sort by timestamp (newest first) and limit
                results.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
                results.truncate(self.max_results);
                self.results = results;
            }
            Err(_) => {
                self.results = Vec::new();
            }
        }
    }

    fn load_all_messages(
        &mut self,
        pattern: &str,
        role_filter: &Option<String>,
    ) -> Result<Vec<SearchResult>> {
        use crate::search::{discover_claude_files, expand_tilde};

        let expanded_pattern = expand_tilde(pattern);
        let files = if expanded_pattern.is_file() {
            vec![expanded_pattern]
        } else {
            discover_claude_files(Some(pattern))?
        };

        let mut results = Vec::new();

        for file_path in &files {
            let cached_file = self.cache.get_messages(file_path)?;
            let file_name = file_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
                .to_string();

            for (idx, message) in cached_file.messages.iter().enumerate() {
                let text = message.get_content_text();

                // Apply role filter only
                if let Some(role) = role_filter {
                    if message.get_type() != role {
                        continue;
                    }
                }

                if let Some(session_id) = &self.base_options.session_id {
                    if message.get_session_id() != Some(session_id) {
                        continue;
                    }
                }

                let timestamp = message.get_timestamp().unwrap_or("").to_string();

                results.push(SearchResult {
                    file: file_name.clone(),
                    uuid: message.get_uuid().unwrap_or("").to_string(),
                    timestamp,
                    session_id: message.get_session_id().unwrap_or("").to_string(),
                    role: message.get_type().to_string(),
                    text: text.clone(),
                    has_tools: message.has_tool_use(),
                    has_thinking: message.has_thinking(),
                    message_type: message.get_type().to_string(),
                    query: crate::query::QueryCondition::Literal {
                        pattern: String::new(),
                        case_sensitive: false,
                    },
                    project_path: Self::extract_project_path(file_path),
                    raw_json: Some(cached_file.raw_lines[idx].clone()),
                });
            }
        }

        self.apply_filters(&mut results)?;

        Ok(results)
    }

    fn load_session_messages(&mut self, file_path: &str) -> Result<()> {
        use std::fs::File;
        use std::io::{BufRead, BufReader};

        let file = File::open(file_path)?;
        let reader = BufReader::new(file);
        self.session_messages.clear();

        for line in reader.lines() {
            let line = line?;
            if !line.trim().is_empty() {
                self.session_messages.push(line);
            }
        }

        Ok(())
    }

    fn format_timestamp(timestamp: &str) -> String {
        use chrono::DateTime;

        if let Ok(dt) = DateTime::parse_from_rfc3339(timestamp) {
            dt.format("%m/%d %H:%M").to_string()
        } else {
            timestamp.chars().take(16).collect()
        }
    }

    fn format_timestamp_long(timestamp: &str) -> String {
        use chrono::DateTime;

        if let Ok(dt) = DateTime::parse_from_rfc3339(timestamp) {
            dt.format("%Y-%m-%d %H:%M:%S").to_string()
        } else {
            timestamp.to_string()
        }
    }

    fn copy_to_clipboard(&self, text: &str) -> Result<()> {
        use std::process::Command;

        #[cfg(target_os = "macos")]
        {
            let mut child = Command::new("pbcopy")
                .stdin(std::process::Stdio::piped())
                .spawn()?;

            if let Some(stdin) = child.stdin.as_mut() {
                use std::io::Write;
                stdin.write_all(text.as_bytes())?;
            }

            child.wait()?;
        }

        #[cfg(target_os = "linux")]
        {
            let result = Command::new("xclip")
                .arg("-selection")
                .arg("clipboard")
                .stdin(std::process::Stdio::piped())
                .spawn();

            match result {
                Ok(mut child) => {
                    if let Some(stdin) = child.stdin.as_mut() {
                        use std::io::Write;
                        stdin.write_all(text.as_bytes())?;
                    }
                    child.wait()?;
                }
                Err(_) => {
                    let mut child = Command::new("xsel")
                        .arg("--clipboard")
                        .arg("--input")
                        .stdin(std::process::Stdio::piped())
                        .spawn()?;

                    if let Some(stdin) = child.stdin.as_mut() {
                        use std::io::Write;
                        stdin.write_all(text.as_bytes())?;
                    }

                    child.wait()?;
                }
            }
        }

        #[cfg(target_os = "windows")]
        {
            let mut child = Command::new("clip")
                .stdin(std::process::Stdio::piped())
                .spawn()?;

            if let Some(stdin) = child.stdin.as_mut() {
                use std::io::Write;
                stdin.write_all(text.as_bytes())?;
            }

            child.wait()?;
        }

        Ok(())
    }

    // Synchronous search method for testing
    #[cfg(test)]
    pub fn execute_search_sync(&mut self, pattern: &str) {
        if self.query.is_empty() {
            self.results.clear();
            return;
        }

        let Ok(parsed_query) = parse_query(&self.query) else {
            self.results.clear();
            return;
        };

        let role_filter = self.role_filter.clone();
        match self.execute_cached_search(pattern, &parsed_query, &role_filter) {
            Ok(results) => self.results = results,
            Err(_) => self.results.clear(),
        }
    }

    fn start_search_worker(
        &self,
        receiver: Receiver<SearchRequest>,
        sender: Sender<SearchResponse>,
        pattern: &str,
    ) -> thread::JoinHandle<()> {
        let base_options = self.base_options.clone();
        let max_results = self.max_results;
        let _pattern_owned = pattern.to_string();

        thread::spawn(move || {
            let mut cache = MessageCache::new();

            loop {
                // Use recv_timeout to avoid blocking forever
                match receiver.recv_timeout(Duration::from_millis(100)) {
                    Ok(request) => {
                        // Execute search in worker thread
                        let results = match parse_query(&request.query) {
                            Ok(parsed_query) => Self::execute_cached_search_static(
                                &mut cache,
                                &request.pattern,
                                &parsed_query,
                                &request.role_filter,
                                &base_options,
                                max_results,
                            )
                            .unwrap_or_else(|_| Vec::new()),
                            Err(_) => Vec::new(),
                        };

                        let response = SearchResponse {
                            id: request.id,
                            results,
                        };

                        // Send response back, stop worker if channel is disconnected
                        if sender.send(response).is_err() {
                            break;
                        }
                    }
                    Err(mpsc::RecvTimeoutError::Timeout) => {
                        // Continue checking
                        continue;
                    }
                    Err(mpsc::RecvTimeoutError::Disconnected) => {
                        // Sender was dropped, exit worker
                        break;
                    }
                }
            }
        })
    }

    // Static version of execute_cached_search for use in worker thread
    fn execute_cached_search_static(
        cache: &mut MessageCache,
        pattern: &str,
        query: &crate::query::QueryCondition,
        role_filter: &Option<String>,
        base_options: &SearchOptions,
        max_results: usize,
    ) -> Result<Vec<SearchResult>> {
        use crate::search::{discover_claude_files, expand_tilde};

        let expanded_pattern = expand_tilde(pattern);
        let files = if expanded_pattern.is_file() {
            vec![expanded_pattern]
        } else {
            discover_claude_files(Some(pattern))?
        };

        let mut results = Vec::new();

        for file_path in &files {
            let cached_file = cache.get_messages(file_path)?;
            let file_name = file_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
                .to_string();

            for (idx, message) in cached_file.messages.iter().enumerate() {
                let text = message.get_content_text();

                if let Ok(matches) = query.evaluate(&text) {
                    if matches {
                        if let Some(role) = role_filter {
                            if message.get_type() != role {
                                continue;
                            }
                        }

                        if let Some(session_id) = &base_options.session_id {
                            if message.get_session_id() != Some(session_id) {
                                continue;
                            }
                        }

                        let timestamp = message.get_timestamp().unwrap_or("").to_string();

                        results.push(SearchResult {
                            file: file_name.clone(),
                            uuid: message.get_uuid().unwrap_or("").to_string(),
                            timestamp,
                            session_id: message.get_session_id().unwrap_or("").to_string(),
                            role: message.get_type().to_string(),
                            text: text.clone(),
                            has_tools: message.has_tool_use(),
                            has_thinking: message.has_thinking(),
                            message_type: message.get_type().to_string(),
                            query: query.clone(),
                            project_path: InteractiveSearch::extract_project_path(file_path),
                            raw_json: Some(cached_file.raw_lines[idx].clone()),
                        });
                    }
                }
            }
        }

        Self::apply_filters_static(&mut results, base_options)?;
        results.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        results.truncate(max_results);

        Ok(results)
    }

    fn apply_filters_static(
        results: &mut Vec<SearchResult>,
        options: &SearchOptions,
    ) -> Result<()> {
        use chrono::DateTime;

        if let Some(before) = &options.before {
            let before_time =
                DateTime::parse_from_rfc3339(before).context("Invalid 'before' timestamp")?;
            results.retain(|r| {
                if let Ok(time) = DateTime::parse_from_rfc3339(&r.timestamp) {
                    time < before_time
                } else {
                    false
                }
            });
        }

        if let Some(after) = &options.after {
            let after_time =
                DateTime::parse_from_rfc3339(after).context("Invalid 'after' timestamp")?;
            results.retain(|r| {
                if let Ok(time) = DateTime::parse_from_rfc3339(&r.timestamp) {
                    time > after_time
                } else {
                    false
                }
            });
        }

        Ok(())
    }
}
