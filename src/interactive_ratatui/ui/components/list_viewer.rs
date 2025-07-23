use super::list_item::ListItem;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem as TuiListItem, Paragraph},
};

pub struct ListViewer<T: ListItem> {
    pub items: Vec<T>,
    pub filtered_indices: Vec<usize>,
    pub selected_index: usize,
    pub scroll_offset: usize,
    pub truncation_enabled: bool,
    pub title: String,
    pub empty_message: String,
}

impl<T: ListItem> Default for ListViewer<T> {
    fn default() -> Self {
        Self {
            items: Vec::new(),
            filtered_indices: Vec::new(),
            selected_index: 0,
            scroll_offset: 0,
            truncation_enabled: true,
            title: String::new(),
            empty_message: String::new(),
        }
    }
}

impl<T: ListItem> ListViewer<T> {
    pub fn new(title: String, empty_message: String) -> Self {
        Self {
            items: Vec::new(),
            filtered_indices: Vec::new(),
            selected_index: 0,
            scroll_offset: 0,
            truncation_enabled: true,
            title,
            empty_message,
        }
    }
    
    pub fn set_items(&mut self, items: Vec<T>) {
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
    
    pub fn get_selected_item(&self) -> Option<&T> {
        self.filtered_indices
            .get(self.selected_index)
            .and_then(|&idx| self.items.get(idx))
    }
    
    pub fn items_count(&self) -> usize {
        self.items.len()
    }
    
    pub fn filtered_count(&self) -> usize {
        self.filtered_indices.len()
    }
    
    pub fn selected_index(&self) -> usize {
        // Return the actual item index, not the filtered index
        self.filtered_indices
            .get(self.selected_index)
            .copied()
            .unwrap_or(0)
    }
    
    pub fn move_up(&mut self) -> bool {
        if self.selected_index > 0 {
            self.selected_index -= 1;
            true
        } else {
            false
        }
    }
    
    pub fn move_down(&mut self) -> bool {
        if self.selected_index + 1 < self.filtered_indices.len() {
            self.selected_index += 1;
            true
        } else {
            false
        }
    }
    
    pub fn page_up(&mut self) -> bool {
        let new_index = self.selected_index.saturating_sub(10);
        if new_index != self.selected_index {
            self.selected_index = new_index;
            true
        } else {
            false
        }
    }
    
    pub fn page_down(&mut self) -> bool {
        let new_index = (self.selected_index + 10).min(self.filtered_indices.len().saturating_sub(1));
        if new_index != self.selected_index {
            self.selected_index = new_index;
            true
        } else {
            false
        }
    }
    
    pub fn move_to_start(&mut self) -> bool {
        if self.selected_index > 0 {
            self.selected_index = 0;
            self.scroll_offset = 0;
            true
        } else {
            false
        }
    }
    
    pub fn move_to_end(&mut self) -> bool {
        let last_index = self.filtered_indices.len().saturating_sub(1);
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
    
    pub fn adjust_scroll_offset(&mut self, available_height: u16, available_width: u16) {
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
            
            // Calculate which items are visible with current scroll_offset
            let (start, end) = self.calculate_visible_range(available_height, available_width);
            
            // If selected item is not visible, adjust scroll offset
            if self.selected_index < start {
                self.scroll_offset = self.selected_index;
            } else if self.selected_index >= end {
                // Need to scroll down - find appropriate scroll offset
                let mut test_offset = self.scroll_offset;
                while test_offset < self.filtered_indices.len() {
                    let (_, test_end) = self.calculate_visible_range(available_height, available_width);
                    if self.selected_index < test_end {
                        break;
                    }
                    test_offset += 1;
                    self.scroll_offset = test_offset;
                }
            }
        }
    }
    
    pub fn render(&mut self, f: &mut Frame, area: Rect) {
        if self.items.is_empty() || self.filtered_indices.is_empty() {
            let empty_message = Paragraph::new(self.empty_message.clone())
                .block(Block::default().title(self.title.clone()).borders(Borders::ALL))
                .style(Style::default().fg(Color::DarkGray));
            f.render_widget(empty_message, area);
            return;
        }
        
        let available_height = area.height.saturating_sub(2); // Account for borders
        self.adjust_scroll_offset(available_height, area.width);
        let (start, end) = self.calculate_visible_range(available_height, area.width);
        
        let available_text_width = area.width.saturating_sub(35) as usize;
        
        let items: Vec<TuiListItem> = (start..end)
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
            .collect();
        
        let title = format!(
            "{} ({}/{}) - Showing {}-{}",
            self.title,
            self.selected_index + 1,
            self.filtered_indices.len(),
            start + 1,
            end
        );
        
        let list = List::new(items)
            .block(Block::default().title(title).borders(Borders::ALL))
            .style(Style::default());
        
        f.render_widget(list, area);
    }
}