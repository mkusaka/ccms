use super::list_item::ListItem;
use ratatui::{
    Frame,
    layout::{Constraint, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, TableState},
};

pub struct ListViewer<T: ListItem> {
    pub items: Vec<T>,
    pub filtered_indices: Vec<usize>,
    pub state: TableState,
    pub truncation_enabled: bool,
    pub title: String,
    pub empty_message: String,
    pub with_border: bool,
}

impl<T: ListItem> Default for ListViewer<T> {
    fn default() -> Self {
        let mut state = TableState::default();
        state.select(Some(0));

        Self {
            items: Vec::new(),
            filtered_indices: Vec::new(),
            state,
            truncation_enabled: true,
            title: String::new(),
            empty_message: String::new(),
            with_border: false, // Default to no border (usually used inside ViewLayout)
        }
    }
}

impl<T: ListItem> ListViewer<T> {
    pub fn new(title: String, empty_message: String) -> Self {
        let mut state = TableState::default();
        state.select(Some(0));

        Self {
            items: Vec::new(),
            filtered_indices: Vec::new(),
            state,
            truncation_enabled: true,
            title,
            empty_message,
            with_border: false, // Default to no border (usually used inside ViewLayout)
        }
    }

    pub fn set_items(&mut self, items: Vec<T>) {
        self.items = items;
        self.filtered_indices = (0..self.items.len()).collect();
        self.state.select(Some(0));
    }

    pub fn set_filtered_indices(&mut self, indices: Vec<usize>) {
        // Only update if indices have actually changed
        if self.filtered_indices == indices {
            return;
        }

        self.filtered_indices = indices;
        if !self.filtered_indices.is_empty() {
            // Reset to first item if current selection is out of bounds
            if let Some(selected) = self.state.selected() {
                if selected >= self.filtered_indices.len() {
                    self.state.select(Some(0));
                }
            } else {
                self.state.select(Some(0));
            }
        } else {
            self.state.select(None);
        }
    }

    pub fn set_selected_index(&mut self, index: usize) {
        // If the index is within the items range
        if index < self.items.len() {
            // Find the position of this index in filtered_indices
            if let Some(pos) = self.filtered_indices.iter().position(|&i| i == index) {
                self.state.select(Some(pos));
            }
        }
    }

    pub fn set_scroll_offset(&mut self, _offset: usize) {
        // Table widget handles scrolling automatically
    }

    pub fn set_truncation_enabled(&mut self, enabled: bool) {
        self.truncation_enabled = enabled;
    }

    pub fn set_with_border(&mut self, with_border: bool) {
        self.with_border = with_border;
    }

    pub fn get_selected_item(&self) -> Option<&T> {
        self.state
            .selected()
            .and_then(|idx| self.filtered_indices.get(idx))
            .and_then(|&item_idx| self.items.get(item_idx))
    }

    pub fn items_count(&self) -> usize {
        self.items.len()
    }

    pub fn filtered_count(&self) -> usize {
        self.filtered_indices.len()
    }

    pub fn selected_index(&self) -> usize {
        // Return the actual item index, not the filtered index
        self.state
            .selected()
            .and_then(|idx| self.filtered_indices.get(idx).copied())
            .unwrap_or(0)
    }

    pub fn move_up(&mut self) -> bool {
        if let Some(selected) = self.state.selected() {
            if selected > 0 {
                self.state.select(Some(selected - 1));
                return true;
            }
        }
        false
    }

    pub fn move_down(&mut self) -> bool {
        if let Some(selected) = self.state.selected() {
            if selected + 1 < self.filtered_indices.len() {
                self.state.select(Some(selected + 1));
                return true;
            }
        }
        false
    }

    pub fn page_up(&mut self) -> bool {
        if let Some(selected) = self.state.selected() {
            let new_index = selected.saturating_sub(10);
            if new_index != selected {
                self.state.select(Some(new_index));
                return true;
            }
        }
        false
    }

    pub fn page_down(&mut self) -> bool {
        if let Some(selected) = self.state.selected() {
            let new_index = (selected + 10).min(self.filtered_indices.len().saturating_sub(1));
            if new_index != selected {
                self.state.select(Some(new_index));
                return true;
            }
        }
        false
    }

    pub fn move_to_start(&mut self) -> bool {
        if self.state.selected() != Some(0) && !self.filtered_indices.is_empty() {
            self.state.select(Some(0));
            true
        } else {
            false
        }
    }

    pub fn move_to_end(&mut self) -> bool {
        let last_index = self.filtered_indices.len().saturating_sub(1);
        if self.state.selected() != Some(last_index) && !self.filtered_indices.is_empty() {
            self.state.select(Some(last_index));
            true
        } else {
            false
        }
    }

    pub fn render(&mut self, f: &mut Frame, area: Rect) {
        if self.items.is_empty() || self.filtered_indices.is_empty() {
            let empty_message = Paragraph::new(self.empty_message.clone())
                .block(
                    Block::default()
                        .title(self.title.clone())
                        .borders(Borders::ALL),
                )
                .style(Style::default().fg(Color::DarkGray));
            f.render_widget(empty_message, area);
            return;
        }

        // Create rows from filtered items
        let rows: Vec<Row> = self
            .filtered_indices
            .iter()
            .filter_map(|&idx| {
                self.items.get(idx).map(|item| {
                    Row::new(vec![
                        Cell::from(item.format_timestamp())
                            .style(Style::default().fg(Color::DarkGray)),
                        Cell::from(item.get_role())
                            .style(Style::default().fg(item.get_role_color())),
                        Cell::from(item.get_content()),
                    ])
                })
            })
            .collect();

        // Define column constraints with better distribution
        let widths = [
            Constraint::Length(11), // Timestamp (MM/DD HH:MM)
            Constraint::Length(10), // Role
            Constraint::Min(0),     // Content takes remaining space without forcing minimum
        ];

        // Update title with selection info
        let selected_display = self.state.selected().map(|s| s + 1).unwrap_or(0);

        let title = format!(
            "{} ({}/{})",
            self.title,
            selected_display,
            self.filtered_indices.len()
        );

        // Create table widget, optionally with borders
        let mut table = Table::new(rows, widths)
            .column_spacing(1) // Minimal spacing between columns
            .row_highlight_style(
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            );

        if self.with_border {
            table = table.block(Block::default().title(title).borders(Borders::ALL));
        }

        // Clear area to prevent artifacts
        f.render_widget(ratatui::widgets::Clear, area);

        // Render the table
        f.render_stateful_widget(table, area, &mut self.state);
    }
}
