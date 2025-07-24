use tuirealm::command::{Cmd, CmdResult};
use tuirealm::props::{Alignment, AttrValue, Attribute, Props};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, Borders, List as TuiList, ListItem as TuiListItem, Paragraph};
use tuirealm::{Component, Event, MockComponent, NoUserEvent, State, StateValue};

use super::list_item::ListItem;
use super::messages::AppMessage;

#[derive(Debug, Clone, PartialEq)]
pub struct ListViewerState<T: ListItem + PartialEq> {
    pub items: Vec<T>,
    pub filtered_indices: Vec<usize>,
    pub selected_index: usize,
    pub scroll_offset: usize,
    pub truncation_enabled: bool,
}

impl<T: ListItem + PartialEq> Default for ListViewerState<T> {
    fn default() -> Self {
        Self {
            items: Vec::new(),
            filtered_indices: Vec::new(),
            selected_index: 0,
            scroll_offset: 0,
            truncation_enabled: true,
        }
    }
}

pub struct ListViewer<T: ListItem + PartialEq> {
    props: Props,
    state: ListViewerState<T>,
}

impl<T: ListItem + PartialEq> Default for ListViewer<T> {
    fn default() -> Self {
        Self {
            props: Props::default(),
            state: ListViewerState::default(),
        }
    }
}

impl<T: ListItem + PartialEq> ListViewer<T> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_items(&mut self, items: Vec<T>) {
        self.state.items = items;
        self.state.filtered_indices = (0..self.state.items.len()).collect();
        self.state.selected_index = 0;
        self.state.scroll_offset = 0;
    }

    pub fn set_filtered_indices(&mut self, indices: Vec<usize>) {
        self.state.filtered_indices = indices;
        if self.state.selected_index >= self.state.filtered_indices.len() 
            && !self.state.filtered_indices.is_empty() {
            self.state.selected_index = 0;
            self.state.scroll_offset = 0;
        }
    }

    pub fn set_selected_index(&mut self, index: usize) {
        if index < self.state.items.len() {
            if let Some(pos) = self.state.filtered_indices.iter().position(|&i| i == index) {
                self.state.selected_index = pos;
            }
        }
    }

    pub fn set_truncation_enabled(&mut self, enabled: bool) {
        self.state.truncation_enabled = enabled;
    }

    fn get_selected_item(&self) -> Option<&T> {
        self.state.filtered_indices
            .get(self.state.selected_index)
            .and_then(|&idx| self.state.items.get(idx))
    }

    fn move_up(&mut self) -> bool {
        if self.state.selected_index > 0 {
            self.state.selected_index -= 1;
            true
        } else {
            false
        }
    }

    fn move_down(&mut self) -> bool {
        if self.state.selected_index + 1 < self.state.filtered_indices.len() {
            self.state.selected_index += 1;
            true
        } else {
            false
        }
    }

    fn page_up(&mut self) -> bool {
        let new_index = self.state.selected_index.saturating_sub(10);
        if new_index != self.state.selected_index {
            self.state.selected_index = new_index;
            true
        } else {
            false
        }
    }

    fn page_down(&mut self) -> bool {
        let new_index = (self.state.selected_index + 10)
            .min(self.state.filtered_indices.len().saturating_sub(1));
        if new_index != self.state.selected_index {
            self.state.selected_index = new_index;
            true
        } else {
            false
        }
    }

    fn move_to_start(&mut self) -> bool {
        if self.state.selected_index > 0 {
            self.state.selected_index = 0;
            self.state.scroll_offset = 0;
            true
        } else {
            false
        }
    }

    fn move_to_end(&mut self) -> bool {
        let last_index = self.state.filtered_indices.len().saturating_sub(1);
        if self.state.selected_index < last_index {
            self.state.selected_index = last_index;
            true
        } else {
            false
        }
    }

    fn calculate_visible_range(&self, available_height: u16, available_width: u16) -> (usize, usize) {
        if self.state.truncation_enabled {
            let visible_count = available_height as usize;
            let start = self.state.scroll_offset;
            let end = (start + visible_count).min(self.state.filtered_indices.len());
            (start, end)
        } else {
            let start = self.state.scroll_offset;
            let mut current_height = 0;
            let mut end = start;
            let available_text_width = available_width.saturating_sub(35) as usize;

            while end < self.state.filtered_indices.len() 
                && current_height < available_height as usize {
                if let Some(&item_idx) = self.state.filtered_indices.get(end) {
                    if let Some(item) = self.state.items.get(item_idx) {
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
        if self.state.truncation_enabled {
            let visible_count = available_height as usize;
            if self.state.selected_index < self.state.scroll_offset {
                self.state.scroll_offset = self.state.selected_index;
            } else if self.state.selected_index >= self.state.scroll_offset + visible_count {
                self.state.scroll_offset = self.state.selected_index - visible_count + 1;
            }
        } else {
            let (start, end) = self.calculate_visible_range(available_height, available_width);

            if self.state.selected_index < start {
                self.state.scroll_offset = self.state.selected_index;
            } else if self.state.selected_index >= end {
                let mut test_offset = self.state.scroll_offset;
                while test_offset < self.state.filtered_indices.len() {
                    let original_offset = self.state.scroll_offset;
                    self.state.scroll_offset = test_offset;
                    let (_, test_end) = self.calculate_visible_range(available_height, available_width);

                    if self.state.selected_index < test_end {
                        break;
                    }

                    self.state.scroll_offset = original_offset;
                    test_offset += 1;
                }
                self.state.scroll_offset = test_offset;
            }
        }
    }
}

impl<T: ListItem + PartialEq> MockComponent for ListViewer<T> {
    fn view(&mut self, render: &mut Frame, area: Rect) {
        let title = self
            .props
            .get_or(Attribute::Title, AttrValue::Title(("List".to_string(), Alignment::Left)))
            .unwrap_title();
        
        let empty_message = self
            .props
            .get_or(Attribute::Text, AttrValue::String("No items".to_string()))
            .unwrap_string();

        if self.state.items.is_empty() || self.state.filtered_indices.is_empty() {
            let widget = Paragraph::new(empty_message)
                .block(Block::default().title(title.0).borders(Borders::ALL))
                .style(Style::default().fg(Color::DarkGray));
            render.render_widget(widget, area);
            return;
        }

        let available_height = area.height.saturating_sub(2);
        self.adjust_scroll_offset(available_height, area.width);
        let (start, end) = self.calculate_visible_range(available_height, area.width);

        let available_text_width = area.width.saturating_sub(35) as usize;

        let items: Vec<TuiListItem> = (start..end)
            .filter_map(|i| {
                self.state.filtered_indices.get(i).and_then(|&item_idx| {
                    self.state.items.get(item_idx).map(|item| {
                        let is_selected = i == self.state.selected_index;

                        let style = if is_selected {
                            Style::default()
                                .bg(Color::DarkGray)
                                .add_modifier(Modifier::BOLD)
                        } else {
                            Style::default()
                        };

                        if self.state.truncation_enabled {
                            TuiListItem::new(item.create_truncated_line(available_text_width))
                                .style(style)
                        } else {
                            TuiListItem::new(item.create_full_lines(available_text_width))
                                .style(style)
                        }
                    })
                })
            })
            .collect();

        let display_title = format!(
            "{} ({}/{}) - Showing {}-{}",
            title.0,
            self.state.selected_index + 1,
            self.state.filtered_indices.len(),
            start + 1,
            end
        );

        let list = TuiList::new(items)
            .block(Block::default().title(display_title).borders(Borders::ALL))
            .style(Style::default());

        render.render_widget(list, area);
    }

    fn query(&self, attr: Attribute) -> Option<AttrValue> {
        match self.props.get(attr) {
            Some(value) => Some(value.clone()),
            None => None,
        }
    }

    fn attr(&mut self, attr: Attribute, value: AttrValue) {
        self.props.set(attr, value);
    }

    fn state(&self) -> State {
        State::Vec(vec![
            StateValue::Usize(self.state.selected_index),
            StateValue::Usize(self.state.filtered_indices.len()),
        ])
    }

    fn perform(&mut self, cmd: Cmd) -> CmdResult {
        match cmd {
            Cmd::Move(tuirealm::command::Direction::Up) => {
                if self.move_up() {
                    CmdResult::Changed(self.state())
                } else {
                    CmdResult::None
                }
            }
            Cmd::Move(tuirealm::command::Direction::Down) => {
                if self.move_down() {
                    CmdResult::Changed(self.state())
                } else {
                    CmdResult::None
                }
            }
            Cmd::Scroll(tuirealm::command::Direction::Up) => {
                if self.page_up() {
                    CmdResult::Changed(self.state())
                } else {
                    CmdResult::None
                }
            }
            Cmd::Scroll(tuirealm::command::Direction::Down) => {
                if self.page_down() {
                    CmdResult::Changed(self.state())
                } else {
                    CmdResult::None
                }
            }
            Cmd::GoTo(tuirealm::command::Position::Begin) => {
                if self.move_to_start() {
                    CmdResult::Changed(self.state())
                } else {
                    CmdResult::None
                }
            }
            Cmd::GoTo(tuirealm::command::Position::End) => {
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

impl<T: ListItem + PartialEq> Component<AppMessage, NoUserEvent> for ListViewer<T> {
    fn on(&mut self, ev: Event<NoUserEvent>) -> Option<AppMessage> {
        use tuirealm::event::{Key, KeyEvent};
        
        match ev {
            Event::Keyboard(KeyEvent { code: Key::Up, .. }) => {
                self.perform(Cmd::Move(tuirealm::command::Direction::Up));
                None
            }
            Event::Keyboard(KeyEvent { code: Key::Down, .. }) => {
                self.perform(Cmd::Move(tuirealm::command::Direction::Down));
                None
            }
            Event::Keyboard(KeyEvent { code: Key::PageUp, .. }) => {
                self.perform(Cmd::Scroll(tuirealm::command::Direction::Up));
                None
            }
            Event::Keyboard(KeyEvent { code: Key::PageDown, .. }) => {
                self.perform(Cmd::Scroll(tuirealm::command::Direction::Down));
                None
            }
            Event::Keyboard(KeyEvent { code: Key::Home, .. }) => {
                self.perform(Cmd::GoTo(tuirealm::command::Position::Begin));
                None
            }
            Event::Keyboard(KeyEvent { code: Key::End, .. }) => {
                self.perform(Cmd::GoTo(tuirealm::command::Position::End));
                None
            }
            Event::Keyboard(KeyEvent { code: Key::Enter, .. }) => {
                if self.get_selected_item().is_some() {
                    Some(AppMessage::SelectResult(self.state.selected_index))
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}