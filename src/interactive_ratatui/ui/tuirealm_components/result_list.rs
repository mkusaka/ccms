use tuirealm::command::{Cmd, CmdResult, Direction, Position};
use tuirealm::event::{Key, KeyEvent};
use tuirealm::props::{AttrValue, Attribute, Props};
use tuirealm::{Component, Event, MockComponent, NoUserEvent, State, StateValue};

use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, Borders, List, ListItem as TuiListItem, Paragraph};
use ratatui::Frame;

use crate::interactive_ratatui::ui::tuirealm_components::messages::AppMessage;
use crate::query::condition::SearchResult;
use crate::interactive_ratatui::ui::components::list_item::ListItem;

/// Result list component for tui-realm
pub struct ResultList {
    props: Props,
    /// The search results
    items: Vec<SearchResult>,
    /// Currently selected index
    selected_index: usize,
    /// Scroll offset for visible items
    scroll_offset: usize,
    /// Whether truncation is enabled
    truncation_enabled: bool,
}

impl Default for ResultList {
    fn default() -> Self {
        Self::new()
    }
}

impl ResultList {
    pub fn new() -> Self {
        Self {
            props: Props::default(),
            items: Vec::new(),
            selected_index: 0,
            scroll_offset: 0,
            truncation_enabled: true,
        }
    }

    pub fn set_results(&mut self, results: Vec<SearchResult>) {
        self.items = results;
        self.selected_index = 0;
        self.scroll_offset = 0;
    }

    pub fn set_selected_index(&mut self, index: usize) {
        if index < self.items.len() {
            self.selected_index = index;
        }
    }

    pub fn selected_result(&self) -> Option<&SearchResult> {
        self.items.get(self.selected_index)
    }

    pub fn update_results(&mut self, results: Vec<SearchResult>, selected_index: usize) {
        self.items = results;
        self.set_selected_index(selected_index);
    }

    pub fn set_truncation_enabled(&mut self, enabled: bool) {
        self.truncation_enabled = enabled;
    }

    pub fn update_selection(&mut self, index: usize) {
        self.set_selected_index(index);
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn selected_index(&self) -> usize {
        self.selected_index
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
        if self.selected_index + 1 < self.items.len() {
            self.selected_index += 1;
            true
        } else {
            false
        }
    }

    fn page_up(&mut self) -> bool {
        let new_index = self.selected_index.saturating_sub(10);
        if new_index != self.selected_index {
            self.selected_index = new_index;
            true
        } else {
            false
        }
    }

    fn page_down(&mut self) -> bool {
        let new_index = (self.selected_index + 10).min(self.items.len().saturating_sub(1));
        if new_index != self.selected_index {
            self.selected_index = new_index;
            true
        } else {
            false
        }
    }

    fn move_to_start(&mut self) -> bool {
        if self.selected_index > 0 {
            self.selected_index = 0;
            self.scroll_offset = 0;
            true
        } else {
            false
        }
    }

    fn move_to_end(&mut self) -> bool {
        let last_index = self.items.len().saturating_sub(1);
        if self.selected_index < last_index {
            self.selected_index = last_index;
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
            let end = (start + visible_count).min(self.items.len());
            (start, end)
        } else {
            // In full text mode, calculate how many items fit
            let start = self.scroll_offset;
            let mut current_height = 0;
            let mut end = start;

            // Calculate available width for text (accounting for timestamp and role)
            let available_text_width = available_width.saturating_sub(35) as usize;

            while end < self.items.len() && current_height < available_height as usize {
                if let Some(item) = self.items.get(end) {
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
                while test_offset < self.items.len() {
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
                self.items.get(i).map(|item| {
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
            .collect()
    }
}

impl MockComponent for ResultList {
    fn view(&mut self, frame: &mut Frame, area: Rect) {
        if self.items.is_empty() {
            let empty_message = Paragraph::new("No results found")
                .block(
                    Block::default()
                        .title("Results")
                        .borders(Borders::ALL),
                )
                .style(Style::default().fg(Color::DarkGray));
            frame.render_widget(empty_message, area);
            return;
        }

        let available_height = area.height.saturating_sub(2); // Account for borders
        self.adjust_scroll_offset(available_height, area.width);
        let (start, end) = self.calculate_visible_range(available_height, area.width);

        let available_text_width = area.width.saturating_sub(35) as usize;
        let items = self.create_list_items(start, end, available_text_width);

        let title = format!(
            "Results ({}/{}) - Showing {}-{}",
            self.selected_index + 1,
            self.items.len(),
            start + 1,
            end
        );

        let list = List::new(items)
            .block(Block::default().title(title).borders(Borders::ALL))
            .style(Style::default());

        frame.render_widget(list, area);
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
            Cmd::Scroll(Direction::Up) => {
                if self.page_up() {
                    CmdResult::Changed(self.state())
                } else {
                    CmdResult::None
                }
            }
            Cmd::Scroll(Direction::Down) => {
                if self.page_down() {
                    CmdResult::Changed(self.state())
                } else {
                    CmdResult::None
                }
            }
            Cmd::GoTo(Position::Begin) => {
                if self.move_to_start() {
                    CmdResult::Changed(self.state())
                } else {
                    CmdResult::None
                }
            }
            Cmd::GoTo(Position::End) => {
                if self.move_to_end() {
                    CmdResult::Changed(self.state())
                } else {
                    CmdResult::None
                }
            }
            _ => CmdResult::None,
        }
    }
}

impl Component<AppMessage, NoUserEvent> for ResultList {
    fn on(&mut self, ev: Event<NoUserEvent>) -> Option<AppMessage> {
        match ev {
            Event::Keyboard(KeyEvent {
                code: Key::Up, ..
            }) => {
                if self.move_up() {
                    Some(AppMessage::SelectResult(self.selected_index))
                } else {
                    None
                }
            }
            Event::Keyboard(KeyEvent {
                code: Key::Down, ..
            }) => {
                if self.move_down() {
                    Some(AppMessage::SelectResult(self.selected_index))
                } else {
                    None
                }
            }
            Event::Keyboard(KeyEvent {
                code: Key::Char('k'), ..
            }) => {
                if self.move_up() {
                    Some(AppMessage::SelectResult(self.selected_index))
                } else {
                    None
                }
            }
            Event::Keyboard(KeyEvent {
                code: Key::Char('j'), ..
            }) => {
                if self.move_down() {
                    Some(AppMessage::SelectResult(self.selected_index))
                } else {
                    None
                }
            }
            Event::Keyboard(KeyEvent {
                code: Key::PageUp, ..
            }) => {
                if self.page_up() {
                    Some(AppMessage::SelectResult(self.selected_index))
                } else {
                    None
                }
            }
            Event::Keyboard(KeyEvent {
                code: Key::PageDown, ..
            }) => {
                if self.page_down() {
                    Some(AppMessage::SelectResult(self.selected_index))
                } else {
                    None
                }
            }
            Event::Keyboard(KeyEvent {
                code: Key::Home, ..
            }) => {
                if self.move_to_start() {
                    Some(AppMessage::SelectResult(self.selected_index))
                } else {
                    None
                }
            }
            Event::Keyboard(KeyEvent {
                code: Key::End, ..
            }) => {
                if self.move_to_end() {
                    Some(AppMessage::SelectResult(self.selected_index))
                } else {
                    None
                }
            }
            Event::Keyboard(KeyEvent {
                code: Key::Enter, ..
            }) => {
                if !self.items.is_empty() {
                    Some(AppMessage::EnterResultDetail)
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}