//! Keyboard navigation hook for lists

use iocraft::prelude::*;
use futures::StreamExt;

pub struct UseKeyboardNavigationResult {
    pub selected: usize,
    pub scroll_offset: usize,
}

/// Hook for keyboard navigation in lists
pub fn use_keyboard_navigation(
    hooks: &mut Hooks,
    items_count: usize,
    visible_items: usize,
) -> UseKeyboardNavigationResult {
    let selected = hooks.use_state(|| 0);
    let scroll_offset = hooks.use_state(|| 0);
    let mut events = super::use_terminal_events(hooks);
    
    // Handle keyboard events
    hooks.use_future({
        let selected = selected.clone();
        let scroll_offset = scroll_offset.clone();
        
        async move {
            while let Some(event) = events.next().await {
                if let TerminalEvent::Key(key) = event {
                        handle_navigation_key(
                        &key,
                        selected.clone(),
                        scroll_offset.clone(),
                        items_count,
                        visible_items,
                    );
                }
            }
        }
    });
    
    UseKeyboardNavigationResult {
        selected: *selected.read(),
        scroll_offset: *scroll_offset.read(),
    }
}

fn handle_navigation_key(
    key: &iocraft::KeyEvent,
    mut selected: State<usize>,
    mut scroll_offset: State<usize>,
    items_count: usize,
    visible_items: usize,
) {
    if items_count == 0 {
        return;
    }
    
    let current = *selected.read();
    let offset = *scroll_offset.read();
    
    match key.code {
        KeyCode::Up | KeyCode::Char('k') => {
            if current > 0 {
                let new_selected = current - 1;
                selected.set(new_selected);
                
                // Adjust scroll if needed
                if new_selected < offset {
                    scroll_offset.set(new_selected);
                }
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if current < items_count.saturating_sub(1) {
                let new_selected = current + 1;
                selected.set(new_selected);
                
                // Adjust scroll if needed
                if new_selected >= offset + visible_items {
                    scroll_offset.set(new_selected.saturating_sub(visible_items - 1));
                }
            }
        }
        KeyCode::PageUp => {
            let page_size = visible_items.saturating_sub(1).max(1);
            let new_selected = current.saturating_sub(page_size);
            selected.set(new_selected);
            
            // Adjust scroll
            if new_selected < offset {
                scroll_offset.set(new_selected);
            }
        }
        KeyCode::PageDown => {
            let page_size = visible_items.saturating_sub(1).max(1);
            let new_selected = (current + page_size).min(items_count.saturating_sub(1));
            selected.set(new_selected);
            
            // Adjust scroll
            if new_selected >= offset + visible_items {
                scroll_offset.set(new_selected.saturating_sub(visible_items - 1));
            }
        }
        KeyCode::Home => {
            selected.set(0);
            scroll_offset.set(0);
        }
        KeyCode::End => {
            let last = items_count.saturating_sub(1);
            selected.set(last);
            
            // Scroll to show the last item
            if last >= visible_items {
                scroll_offset.set(last.saturating_sub(visible_items - 1));
            }
        }
        _ => {}
    }
}