use crate::interactive_ratatui::domain::models::{Mode, SearchOrder, SearchTab, SessionOrder};
use crate::query::condition::SearchResult;

/// Represents a complete navigation state that can be restored
#[derive(Clone, Debug)]
pub struct NavigationState {
    pub mode: Mode,
    pub search_state: SearchStateSnapshot,
    pub session_state: SessionStateSnapshot,
    pub ui_state: UiStateSnapshot,
}

/// Snapshot of search state
#[derive(Clone, Debug)]
pub struct SearchStateSnapshot {
    pub query: String,
    pub results: Vec<SearchResult>,
    pub selected_index: usize,
    pub scroll_offset: usize,
    pub role_filter: Option<String>,
    pub order: SearchOrder,
    pub preview_enabled: bool,
    pub current_tab: SearchTab,
}

/// Snapshot of session state
#[derive(Clone, Debug)]
pub struct SessionStateSnapshot {
    pub messages: Vec<String>,
    pub search_results: Vec<SearchResult>,
    pub query: String,
    pub filtered_indices: Vec<usize>,
    pub selected_index: usize,
    pub scroll_offset: usize,
    pub order: SessionOrder,
    pub file_path: Option<String>,
    pub session_id: Option<String>,
    pub role_filter: Option<String>,
}

/// Snapshot of UI state
#[derive(Clone, Debug)]
pub struct UiStateSnapshot {
    pub message: Option<String>,
    pub detail_scroll_offset: usize,
    pub selected_result: Option<SearchResult>,
    pub truncation_enabled: bool,
}

/// Manages navigation history with back/forward capabilities
pub struct NavigationHistory {
    history: Vec<NavigationState>,
    current_index: Option<usize>, // None means we're at the initial state before any navigation
    max_history: usize,
}

impl NavigationHistory {
    pub fn new(max_history: usize) -> Self {
        Self {
            history: Vec::new(),
            current_index: None,
            max_history,
        }
    }

    /// Push a new state to history
    /// This removes any forward history from current position
    pub fn push(&mut self, state: NavigationState) {
        // Remove forward history when navigating to a new state
        match self.current_index {
            None => {
                // First push
                self.history.clear();
            }
            Some(idx) => {
                // Truncate history after current position
                self.history.truncate(idx + 1);
            }
        }

        // Add new state
        self.history.push(state);

        // Maintain max history size
        if self.history.len() > self.max_history {
            self.history.remove(0);
            // Don't update current_index here, it's updated below
        }

        // Update current index to point to the new state
        self.current_index = Some(self.history.len() - 1);
    }

    /// Navigate back in history
    pub fn go_back(&mut self) -> Option<NavigationState> {
        match self.current_index {
            Some(idx) if idx > 0 => {
                // Move to previous index
                self.current_index = Some(idx - 1);
                // Return the state at the new position
                self.history.get(idx - 1).cloned()
            }
            _ => None, // Can't go back from index 0 or None
        }
    }

    /// Navigate forward in history
    pub fn go_forward(&mut self) -> Option<NavigationState> {
        match self.current_index {
            Some(idx) if idx + 1 < self.history.len() => {
                self.current_index = Some(idx + 1);
                self.history.get(idx + 1).cloned()
            }
            None if !self.history.is_empty() => {
                // Can go forward from None to index 0
                self.current_index = Some(0);
                self.history.first().cloned()
            }
            _ => None,
        }
    }

    /// Check if can navigate back
    pub fn can_go_back(&self) -> bool {
        // Can go back if current index > 0
        match self.current_index {
            Some(idx) => idx > 0,
            None => false,
        }
    }

    /// Check if can navigate forward
    pub fn can_go_forward(&self) -> bool {
        match self.current_index {
            Some(idx) => idx + 1 < self.history.len(),
            None => !self.history.is_empty(), // Can go forward from None to index 0
        }
    }

    /// Get current state
    pub fn current(&self) -> Option<&NavigationState> {
        match self.current_index {
            Some(idx) => self.history.get(idx),
            None => None,
        }
    }

    /// Clear history
    pub fn clear(&mut self) {
        self.history.clear();
        self.current_index = None;
    }

    /// Get history length
    pub fn len(&self) -> usize {
        self.history.len()
    }

    /// Check if history is empty
    pub fn is_empty(&self) -> bool {
        self.history.is_empty()
    }

    /// Get current position in history (0-based)
    pub fn position(&self) -> usize {
        self.current_index.unwrap_or(0)
    }

    /// Get current position as Option
    pub fn current_position(&self) -> Option<usize> {
        self.current_index
    }

    /// Update the current state without changing position
    pub fn update_current(&mut self, state: NavigationState) {
        if let Some(idx) = self.current_index
            && let Some(current) = self.history.get_mut(idx) {
                *current = state;
            }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_state(mode: Mode) -> NavigationState {
        NavigationState {
            mode,
            search_state: SearchStateSnapshot {
                query: String::new(),
                results: Vec::new(),
                selected_index: 0,
                scroll_offset: 0,
                role_filter: None,
                order: SearchOrder::Descending,
                preview_enabled: false,
                current_tab: SearchTab::Search,
            },
            session_state: SessionStateSnapshot {
                messages: Vec::new(),
                search_results: Vec::new(),
                query: String::new(),
                filtered_indices: Vec::new(),
                selected_index: 0,
                scroll_offset: 0,
                order: SessionOrder::Ascending,
                file_path: None,
                session_id: None,
                role_filter: None,
            },
            ui_state: UiStateSnapshot {
                message: None,
                detail_scroll_offset: 0,
                selected_result: None,
                truncation_enabled: true,
            },
        }
    }

    #[test]
    fn test_navigation_history_basic() {
        let mut history = NavigationHistory::new(10);
        assert!(history.is_empty());
        assert!(!history.can_go_back());
        assert!(!history.can_go_forward());

        // Push first state
        let state1 = create_test_state(Mode::Search);
        history.push(state1.clone());
        assert_eq!(history.len(), 1);
        assert!(!history.can_go_back()); // Can't go back from position 0
        assert!(!history.can_go_forward());

        // Push second state
        let state2 = create_test_state(Mode::MessageDetail);
        history.push(state2.clone());
        assert_eq!(history.len(), 2);
        assert!(history.can_go_back()); // Can go back
        assert!(!history.can_go_forward());
    }

    #[test]
    fn test_navigation_back_forward() {
        let mut history = NavigationHistory::new(10);

        let state1 = create_test_state(Mode::Search);
        let state2 = create_test_state(Mode::MessageDetail);
        let state3 = create_test_state(Mode::SessionViewer);

        history.push(state1.clone());
        history.push(state2.clone());
        history.push(state3.clone());

        // Current index is 2 (SessionViewer)
        // Go back - should return ResultDetail and move to index 1
        assert!(history.can_go_back());
        let back_state = history.go_back().unwrap();
        assert_eq!(back_state.mode, Mode::MessageDetail);
        assert!(history.can_go_back());
        assert!(history.can_go_forward());

        // Go back again - should return Search and move to index 0
        let back_state2 = history.go_back().unwrap();
        assert_eq!(back_state2.mode, Mode::Search);
        assert!(!history.can_go_back()); // Can't go back from index 0
        assert!(history.can_go_forward());

        // Try to go back from index 0 - should return None
        assert!(history.go_back().is_none());

        // Go forward - should return ResultDetail and move to index 1
        let forward_state = history.go_forward().unwrap();
        assert_eq!(forward_state.mode, Mode::MessageDetail);
        assert!(history.can_go_back());
        assert!(history.can_go_forward());

        // Go forward again - should return SessionViewer and move to index 2
        let forward_state2 = history.go_forward().unwrap();
        assert_eq!(forward_state2.mode, Mode::SessionViewer);
        assert!(history.can_go_back());
        assert!(!history.can_go_forward());
    }

    #[test]
    fn test_navigation_history_truncation() {
        let mut history = NavigationHistory::new(10);

        let state1 = create_test_state(Mode::Search);
        let state2 = create_test_state(Mode::MessageDetail);
        let state3 = create_test_state(Mode::SessionViewer);
        let state4 = create_test_state(Mode::Help);

        history.push(state1.clone());
        history.push(state2.clone());
        history.push(state3.clone());

        // Go back twice (from index 2 to 1, then to 0)
        history.go_back();
        history.go_back();

        // Current index is now 0
        // Push new state - should truncate forward history after index 0
        history.push(state4.clone());
        assert_eq!(history.len(), 2); // Should have states at index 0 and 1
        assert!(!history.can_go_forward());
    }

    #[test]
    fn test_max_history_limit() {
        let mut history = NavigationHistory::new(3);

        for _i in 0..5 {
            let state = create_test_state(Mode::Search);
            history.push(state);
        }

        // Should only keep last 3 states
        assert_eq!(history.len(), 3);
    }
}
