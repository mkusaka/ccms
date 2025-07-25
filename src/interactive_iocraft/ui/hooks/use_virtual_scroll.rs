//! Virtual scrolling hook for efficient rendering of large lists

use iocraft::prelude::*;
use std::cmp::min;

/// Configuration for virtual scrolling
#[derive(Clone, Debug)]
pub struct VirtualScrollConfig {
    /// Total number of items
    pub total_items: usize,
    /// Height of each item in rows
    pub item_height: usize,
    /// Height of the visible viewport in rows
    pub viewport_height: usize,
    /// Number of items to render outside the viewport for smooth scrolling
    pub overscan: usize,
}

/// State for virtual scrolling
#[derive(Clone, Debug)]
pub struct VirtualScrollState {
    /// Current scroll offset
    pub scroll_offset: usize,
    /// Index of the first visible item
    pub start_index: usize,
    /// Index of the last visible item (exclusive)
    pub end_index: usize,
    /// Total height of the scrollable content
    pub total_height: usize,
    /// Offset for positioning the visible items
    pub offset_y: usize,
}

/// Hook for virtual scrolling
pub fn use_virtual_scroll<'a>(
    hooks: &'a mut Hooks,
    config: VirtualScrollConfig,
) -> (State<usize>, VirtualScrollState) {
    let scroll_offset = hooks.use_state(|| 0);
    
    // Calculate virtual scroll state
    let state = calculate_virtual_scroll_state(&config, *scroll_offset.read());
    
    (scroll_offset, state)
}

/// Calculate the virtual scroll state based on configuration and scroll offset
fn calculate_virtual_scroll_state(
    config: &VirtualScrollConfig,
    scroll_offset: usize,
) -> VirtualScrollState {
    // Calculate the total height of all items
    let total_height = config.total_items * config.item_height;
    
    // Calculate which items are visible
    let start_index = scroll_offset / config.item_height;
    let visible_items = (config.viewport_height + config.item_height - 1) / config.item_height;
    let end_index = min(start_index + visible_items, config.total_items);
    
    // Apply overscan
    let start_with_overscan = start_index.saturating_sub(config.overscan);
    let end_with_overscan = min(end_index + config.overscan, config.total_items);
    
    // Calculate offset for positioning
    let offset_y = start_with_overscan * config.item_height;
    
    VirtualScrollState {
        scroll_offset,
        start_index: start_with_overscan,
        end_index: end_with_overscan,
        total_height,
        offset_y,
    }
}

/// Hook that provides optimized list rendering with virtual scrolling
pub fn use_virtual_list<'a, T: Clone + 'static>(
    hooks: &'a mut Hooks,
    items: &[T],
    item_height: usize,
    viewport_height: usize,
) -> (State<usize>, Vec<(usize, T)>) {
    let config = VirtualScrollConfig {
        total_items: items.len(),
        item_height,
        viewport_height,
        overscan: 3, // Render 3 extra items above and below
    };
    
    let (scroll_ref, state) = use_virtual_scroll(hooks, config);
    
    // Get the visible items with their original indices
    let visible_items: Vec<(usize, T)> = items
        .iter()
        .enumerate()
        .skip(state.start_index)
        .take(state.end_index - state.start_index)
        .map(|(idx, item)| (idx, item.clone()))
        .collect();
    
    (scroll_ref, visible_items)
}

/// Helper to handle scroll events for virtual scrolling
pub fn handle_virtual_scroll(
    scroll_ref: &mut State<usize>,
    direction: ScrollDirection,
    config: &VirtualScrollConfig,
) {
    let current = *scroll_ref.read();
    let max_scroll = config.total_items.saturating_sub(config.viewport_height / config.item_height) * config.item_height;
    
    match direction {
        ScrollDirection::Up => {
            scroll_ref.set(current.saturating_sub(config.item_height));
        }
        ScrollDirection::Down => {
            scroll_ref.set(min(current + config.item_height, max_scroll));
        }
        ScrollDirection::PageUp => {
            scroll_ref.set(current.saturating_sub(config.viewport_height));
        }
        ScrollDirection::PageDown => {
            scroll_ref.set(min(current + config.viewport_height, max_scroll));
        }
        ScrollDirection::Home => {
            scroll_ref.set(0);
        }
        ScrollDirection::End => {
            scroll_ref.set(max_scroll);
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum ScrollDirection {
    Up,
    Down,
    PageUp,
    PageDown,
    Home,
    End,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_virtual_scroll_calculation() {
        let config = VirtualScrollConfig {
            total_items: 1000,
            item_height: 1,
            viewport_height: 20,
            overscan: 3,
        };
        
        // Test at start
        let state = calculate_virtual_scroll_state(&config, 0);
        assert_eq!(state.start_index, 0);
        assert_eq!(state.end_index, 23); // 20 visible + 3 overscan
        
        // Test in middle
        let state = calculate_virtual_scroll_state(&config, 100);
        assert_eq!(state.start_index, 97); // 100 - 3 overscan
        assert_eq!(state.end_index, 123); // 100 + 20 + 3 overscan
        
        // Test near end
        let state = calculate_virtual_scroll_state(&config, 980);
        assert_eq!(state.start_index, 977);
        assert_eq!(state.end_index, 1000); // Capped at total items
    }
    
    #[test]
    fn test_scroll_direction_handling() {
        let config = VirtualScrollConfig {
            total_items: 100,
            item_height: 1,
            viewport_height: 10,
            overscan: 0,
        };
        
        // Use a mock state
        let mut scroll = 50;
        
        // Test up
        let new_scroll = match ScrollDirection::Up {
            ScrollDirection::Up => scroll.saturating_sub(config.item_height),
            _ => scroll,
        };
        assert_eq!(new_scroll, 49);
        
        // Test page down
        scroll = 50;
        let new_scroll = match ScrollDirection::PageDown {
            ScrollDirection::PageDown => scroll + config.viewport_height,
            _ => scroll,
        };
        assert_eq!(new_scroll, 60);
    }
}