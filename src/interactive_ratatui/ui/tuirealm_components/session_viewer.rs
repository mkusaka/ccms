use tuirealm::command::{Cmd, CmdResult, Direction};
use tuirealm::event::{Key, KeyEvent};
use tuirealm::props::{AttrValue, Attribute, Props};
use tuirealm::{Component, Event, MockComponent, NoUserEvent, State, StateValue};

use ratatui::layout::{Constraint, Direction as RatatuiDirection, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, List, ListItem as TuiListItem, Paragraph};
use ratatui::Frame;

use crate::interactive_ratatui::ui::tuirealm_components::messages::AppMessage;
use crate::interactive_ratatui::ui::tuirealm_components::text_input::TextInput;
use crate::interactive_ratatui::domain::models::SessionOrder;
use crate::interactive_ratatui::domain::session_list_item::SessionListItem;
use crate::interactive_ratatui::ui::components::list_item::ListItem;

/// Session viewer component for tui-realm
pub struct SessionViewer {
    props: Props,
    /// List of session messages
    items: Vec<SessionListItem>,
    /// Raw JSON messages
    raw_messages: Vec<String>,
    /// Filtered indices to display
    filtered_indices: Vec<usize>,
    /// Currently selected index
    selected_index: usize,
    /// Scroll offset for visible items
    scroll_offset: usize,
    /// Whether truncation is enabled
    truncation_enabled: bool,
    /// Search input component
    text_input: TextInput,
    /// Current sort order
    order: Option<SessionOrder>,
    /// Whether we're in search mode
    is_searching: bool,
    /// File path of the session
    file_path: Option<String>,
    /// Session ID
    session_id: Option<String>,
}

impl Default for SessionViewer {
    fn default() -> Self {
        Self::new()
    }
}

impl SessionViewer {
    pub fn new() -> Self {
        let mut text_input = TextInput::default();
        text_input.attr(
            Attribute::Custom("id"),
            AttrValue::String("session_search".to_string()),
        );

        Self {
            props: Props::default(),
            items: Vec::new(),
            raw_messages: Vec::new(),
            filtered_indices: Vec::new(),
            selected_index: 0,
            scroll_offset: 0,
            truncation_enabled: true,
            text_input,
            order: None,
            is_searching: false,
            file_path: None,
            session_id: None,
        }
    }

    pub fn set_messages(&mut self, messages: Vec<String>) {
        self.raw_messages = messages;

        // Convert raw messages to SessionListItems
        let items: Vec<SessionListItem> = self
            .raw_messages
            .iter()
            .enumerate()
            .filter_map(|(idx, line)| SessionListItem::from_json_line(idx, line))
            .collect();

        self.items = items;
        self.filtered_indices = (0..self.items.len()).collect();
        self.selected_index = 0;
        self.scroll_offset = 0;
    }

    pub fn set_filtered_indices(&mut self, indices: Vec<usize>) {
        self.filtered_indices = indices;
        if self.selected_index >= self.filtered_indices.len() && !self.filtered_indices.is_empty() {
            self.selected_index = 0;
            self.scroll_offset = 0;
        }
    }

    pub fn set_query(&mut self, query: String) {
        self.text_input.set_text(query);
    }

    pub fn set_order(&mut self, order: Option<SessionOrder>) {
        self.order = order;
    }

    pub fn set_file_path(&mut self, file_path: Option<String>) {
        self.file_path = file_path;
    }

    pub fn set_session_id(&mut self, session_id: Option<String>) {
        self.session_id = session_id;
    }

    pub fn set_selected_index(&mut self, index: usize) {
        // If the index is within the items range
        if index < self.items.len() {
            // Find the position of this index in filtered_indices
            if let Some(pos) = self.filtered_indices.iter().position(|&i| i == index) {
                self.selected_index = pos;
            }
        }
    }

    pub fn set_scroll_offset(&mut self, offset: usize) {
        self.scroll_offset = offset;
    }

    pub fn set_truncation_enabled(&mut self, enabled: bool) {
        self.truncation_enabled = enabled;
    }

    pub fn start_search(&mut self) {
        self.is_searching = true;
        self.text_input.set_text(String::new());
    }

    pub fn stop_search(&mut self) {
        self.is_searching = false;
    }

    pub fn is_searching(&self) -> bool {
        self.is_searching
    }

    pub fn get_query(&self) -> &str {
        self.text_input.text()
    }

    pub fn items_count(&self) -> usize {
        self.items.len()
    }

    pub fn filtered_count(&self) -> usize {
        self.filtered_indices.len()
    }

    fn get_selected_item(&self) -> Option<&SessionListItem> {
        self.filtered_indices
            .get(self.selected_index)
            .and_then(|&idx| self.items.get(idx))
    }

    fn move_up(&mut self) -> bool {
        if self.selected_index > 0 {
            self.selected_index -= 1;
            true
        } else {
            false
        }
    }

    fn move_down(&mut self) -> bool {
        if self.selected_index + 1 < self.filtered_indices.len() {
            self.selected_index += 1;
            true
        } else {
            false
        }
    }

    fn calculate_visible_range(&self, available_height: u16, available_width: u16) -> (usize, usize) {
        if self.truncation_enabled {
            // In truncated mode, each item takes 1 line
            let visible_count = available_height as usize;
            let start = self.scroll_offset;
            let end = (start + visible_count).min(self.filtered_indices.len());
            (start, end)
        } else {
            // In full text mode, calculate how many items fit
            let start = self.scroll_offset;
            let mut current_height = 0;
            let mut end = start;

            // Calculate available width for text (accounting for timestamp and role)
            let available_text_width = available_width.saturating_sub(35) as usize;

            while end < self.filtered_indices.len() && current_height < available_height as usize {
                if let Some(&item_idx) = self.filtered_indices.get(end) {
                    if let Some(item) = self.items.get(item_idx) {
                        let lines = item.create_full_lines(available_text_width);
                        let item_height = lines.len();

                        if current_height + item_height <= available_height as usize {
                            current_height += item_height;
                            end += 1;
                        } else {
                            break;
                        }
                    }
                }
            }

            (start, end)
        }
    }

    fn adjust_scroll_offset(&mut self, available_height: u16, available_width: u16) {
        if self.truncation_enabled {
            // In truncated mode, each item takes 1 line
            let visible_count = available_height as usize;
            if self.selected_index < self.scroll_offset {
                self.scroll_offset = self.selected_index;
            } else if self.selected_index >= self.scroll_offset + visible_count {
                self.scroll_offset = self.selected_index - visible_count + 1;
            }
        } else {
            // In full text mode, ensure selected item is visible
            let (start, end) = self.calculate_visible_range(available_height, available_width);

            // If selected item is not visible, adjust scroll offset
            if self.selected_index < start {
                self.scroll_offset = self.selected_index;
            } else if self.selected_index >= end {
                // Need to scroll down - find appropriate scroll offset
                let mut test_offset = self.scroll_offset;
                while test_offset < self.filtered_indices.len() {
                    // Temporarily update scroll_offset to test if selected item would be visible
                    let original_offset = self.scroll_offset;
                    self.scroll_offset = test_offset;
                    let (_, test_end) = self.calculate_visible_range(available_height, available_width);

                    if self.selected_index < test_end {
                        // Found the right offset, keep it
                        break;
                    }

                    // Restore original offset and try next
                    self.scroll_offset = original_offset;
                    test_offset += 1;
                }

                // Update to the final test offset
                self.scroll_offset = test_offset;
            }
        }
    }

    fn create_list_items(&self, start: usize, end: usize, available_text_width: usize) -> Vec<TuiListItem> {
        (start..end)
            .filter_map(|i| {
                self.filtered_indices.get(i).and_then(|&item_idx| {
                    self.items.get(item_idx).map(|item| {
                        let is_selected = i == self.selected_index;

                        let style = if is_selected {
                            Style::default()
                                .bg(Color::DarkGray)
                                .add_modifier(Modifier::BOLD)
                        } else {
                            Style::default()
                        };

                        if self.truncation_enabled {
                            TuiListItem::new(item.create_truncated_line(available_text_width))
                                .style(style)
                        } else {
                            TuiListItem::new(item.create_full_lines(available_text_width))
                                .style(style)
                        }
                    })
                })
            })
            .collect()
    }
}

impl MockComponent for SessionViewer {
    fn view(&mut self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(RatatuiDirection::Vertical)
            .constraints([
                Constraint::Length(3), // Search bar or info bar
                Constraint::Min(0),    // Messages
            ])
            .split(area);

        // Render search bar
        if self.is_searching {
            let search_text = self.text_input.render_cursor_spans();

            let search_bar = Paragraph::new(Line::from(search_text)).block(
                Block::default()
                    .title("Search in session (Esc to cancel)")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Yellow)),
            );
            frame.render_widget(search_bar, chunks[0]);
        } else {
            let info_text = format!(
                "Messages: {} (filtered: {}) | Order: {} | Press '/' to search",
                self.items_count(),
                self.filtered_count(),
                match self.order {
                    Some(SessionOrder::Ascending) => "Ascending",
                    Some(SessionOrder::Descending) => "Descending",
                    Some(SessionOrder::Original) => "Original",
                    None => "Default",
                }
            );
            let info_bar = Paragraph::new(info_text).block(Block::default().borders(Borders::ALL));
            frame.render_widget(info_bar, chunks[0]);
        }

        // Render message list
        if self.items.is_empty() || self.filtered_indices.is_empty() {
            let empty_message = Paragraph::new("No messages in session")
                .block(
                    Block::default()
                        .title("Session Messages")
                        .borders(Borders::ALL),
                )
                .style(Style::default().fg(Color::DarkGray));
            frame.render_widget(empty_message, chunks[1]);
            return;
        }

        let available_height = chunks[1].height.saturating_sub(2); // Account for borders
        self.adjust_scroll_offset(available_height, chunks[1].width);
        let (start, end) = self.calculate_visible_range(available_height, chunks[1].width);

        let available_text_width = chunks[1].width.saturating_sub(35) as usize;
        let items = self.create_list_items(start, end, available_text_width);

        let title = format!(
            "Session Messages ({}/{}) - Showing {}-{}",
            self.selected_index + 1,
            self.filtered_indices.len(),
            start + 1,
            end
        );

        let list = List::new(items)
            .block(Block::default().title(title).borders(Borders::ALL))
            .style(Style::default());

        frame.render_widget(list, chunks[1]);
    }

    fn query(&self, attr: Attribute) -> Option<AttrValue> {
        self.props.get(attr)
    }

    fn attr(&mut self, attr: Attribute, value: AttrValue) {
        self.props.set(attr, value);
    }

    fn state(&self) -> State {
        State::One(StateValue::Usize(self.selected_index))
    }

    fn perform(&mut self, cmd: Cmd) -> CmdResult {
        if self.is_searching {
            // Delegate to text input when searching
            self.text_input.perform(cmd)
        } else {
            match cmd {
                Cmd::Move(Direction::Up) => {
                    if self.move_up() {
                        CmdResult::Changed(self.state())
                    } else {
                        CmdResult::None
                    }
                }
                Cmd::Move(Direction::Down) => {
                    if self.move_down() {
                        CmdResult::Changed(self.state())
                    } else {
                        CmdResult::None
                    }
                }
                _ => CmdResult::None,
            }
        }
    }
}

impl Component<AppMessage, NoUserEvent> for SessionViewer {
    fn on(&mut self, ev: Event<NoUserEvent>) -> Option<AppMessage> {
        if self.is_searching {
            match ev {
                Event::Keyboard(KeyEvent {
                    code: Key::Esc, ..
                }) => {
                    self.is_searching = false;
                    self.text_input.set_text(String::new());
                    Some(AppMessage::SessionQueryChanged(String::new()))
                }
                Event::Keyboard(KeyEvent {
                    code: Key::Enter, ..
                }) => {
                    self.is_searching = false;
                    None
                }
                Event::Keyboard(KeyEvent {
                    code: Key::Backspace, ..
                }) => {
                    // Handle special case for Backspace when query becomes empty
                    if self.text_input.text().len() == 1 {
                        self.is_searching = false;
                    }
                    // Delegate to text input
                    self.text_input.on(ev)
                }
                _ => {
                    // Delegate all other keyboard events to text input
                    self.text_input.on(ev)
                }
            }
        } else {
            match ev {
                Event::Keyboard(KeyEvent {
                    code: Key::Up, ..
                })
                | Event::Keyboard(KeyEvent {
                    code: Key::Char('k'),
                    ..
                }) => {
                    if self.move_up() {
                        Some(AppMessage::SessionScrollUp)
                    } else {
                        None
                    }
                }
                Event::Keyboard(KeyEvent {
                    code: Key::Down, ..
                })
                | Event::Keyboard(KeyEvent {
                    code: Key::Char('j'),
                    ..
                }) => {
                    if self.move_down() {
                        Some(AppMessage::SessionScrollDown)
                    } else {
                        None
                    }
                }
                Event::Keyboard(KeyEvent {
                    code: Key::Char('/'),
                    ..
                }) => {
                    self.is_searching = true;
                    None
                }
                Event::Keyboard(KeyEvent {
                    code: Key::Char('o'),
                    ..
                }) => Some(AppMessage::ToggleSessionOrder),
                Event::Keyboard(KeyEvent {
                    code: Key::Char('c'),
                    ..
                }) => self
                    .get_selected_item()
                    .map(|item| AppMessage::CopyToClipboard(item.raw_json.clone())),
                Event::Keyboard(KeyEvent {
                    code: Key::Char('C'),
                    ..
                }) => {
                    // Copy all raw messages
                    Some(AppMessage::CopyToClipboard(self.raw_messages.join("\n\n")))
                }
                Event::Keyboard(KeyEvent {
                    code: Key::Char('i'),
                    ..
                })
                | Event::Keyboard(KeyEvent {
                    code: Key::Char('I'),
                    ..
                }) => self.session_id.clone().map(AppMessage::CopyToClipboard),
                Event::Keyboard(KeyEvent {
                    code: Key::Char('f'),
                    ..
                })
                | Event::Keyboard(KeyEvent {
                    code: Key::Char('F'),
                    ..
                }) => self.file_path.clone().map(AppMessage::CopyToClipboard),
                Event::Keyboard(KeyEvent {
                    code: Key::Char('m'),
                    ..
                })
                | Event::Keyboard(KeyEvent {
                    code: Key::Char('M'),
                    ..
                }) => self
                    .get_selected_item()
                    .map(|item| AppMessage::CopyToClipboard(item.content.clone())),
                Event::Keyboard(KeyEvent {
                    code: Key::Esc, ..
                }) => Some(AppMessage::ExitSessionViewer),
                _ => None,
            }
        }
    }
}