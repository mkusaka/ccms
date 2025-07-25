/// Type safety improvements for tui-realm v3 implementation
use serde::{Serialize, Deserialize};
use tuirealm::AttrValue;
use crate::query::condition::SearchResult;
use super::messages::AppMessage;
use super::state::AppState;
use super::error::{AppError, AppResult};

/// Type-safe index that guarantees validity
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ValidIndex {
    value: usize,
    max: usize,
}

impl ValidIndex {
    /// Create a new ValidIndex, clamping to valid range
    pub fn new(value: usize, max: usize) -> Self {
        if max == 0 {
            return Self { value: 0, max: 0 };
        }
        Self {
            value: value.min(max.saturating_sub(1)),
            max,
        }
    }
    
    /// Get the inner value
    pub fn get(&self) -> usize {
        self.value
    }
    
    /// Get the maximum valid value
    pub fn max(&self) -> usize {
        self.max
    }
    
    /// Try to increment, returning new index if valid
    pub fn increment(&self) -> ValidIndex {
        if self.value + 1 < self.max {
            Self {
                value: self.value + 1,
                max: self.max,
            }
        } else {
            *self
        }
    }
    
    /// Try to decrement, returning new index if valid
    pub fn decrement(&self) -> ValidIndex {
        if self.value > 0 {
            Self {
                value: self.value - 1,
                max: self.max,
            }
        } else {
            *self
        }
    }
    
    /// Move by offset, clamping to valid range
    pub fn offset(&self, offset: isize) -> ValidIndex {
        let new_value = if offset < 0 {
            self.value.saturating_sub(offset.abs() as usize)
        } else {
            self.value.saturating_add(offset as usize)
        };
        Self::new(new_value, self.max)
    }
    
    /// Jump to beginning
    pub fn home(&self) -> ValidIndex {
        Self {
            value: 0,
            max: self.max,
        }
    }
    
    /// Jump to end
    pub fn end(&self) -> ValidIndex {
        Self {
            value: self.max.saturating_sub(1),
            max: self.max,
        }
    }
}

/// Builder pattern for safe message creation
pub struct MessageBuilder;

impl MessageBuilder {
    /// Create EnterResultDetail message with validation
    pub fn enter_result_detail(index: usize, results_len: usize) -> Option<AppMessage> {
        if index < results_len {
            Some(AppMessage::EnterResultDetail(index))
        } else {
            None
        }
    }
    
    /// Create SearchQueryChanged message with validation
    pub fn search_query_changed(query: String) -> Result<AppMessage, String> {
        const MAX_QUERY_LENGTH: usize = 1000;
        
        if query.len() > MAX_QUERY_LENGTH {
            Err(format!("Query too long: {} > {}", query.len(), MAX_QUERY_LENGTH))
        } else {
            Ok(AppMessage::SearchQueryChanged(query))
        }
    }
    
    /// Create SessionQueryChanged message with validation
    pub fn session_query_changed(query: String) -> Result<AppMessage, String> {
        const MAX_QUERY_LENGTH: usize = 500;
        
        if query.len() > MAX_QUERY_LENGTH {
            Err(format!("Session query too long: {} > {}", query.len(), MAX_QUERY_LENGTH))
        } else {
            Ok(AppMessage::SessionQueryChanged(query))
        }
    }
}

/// Type-safe wrapper for component properties
pub struct ComponentProps<T> {
    data: T,
    serialized: String,
}

impl<T: Serialize> ComponentProps<T> {
    /// Create new component props with serialization
    pub fn new(data: T) -> Result<Self, serde_json::Error> {
        let serialized = serde_json::to_string(&data)?;
        Ok(Self { data, serialized })
    }
    
    /// Convert to AttrValue for component
    pub fn as_attr_value(&self) -> AttrValue {
        AttrValue::String(self.serialized.clone())
    }
    
    /// Get the underlying data
    pub fn get_data(&self) -> &T {
        &self.data
    }
}

/// Safe deserialization with fallback
pub fn safe_deserialize<T: for<'de> Deserialize<'de> + Default>(
    attr: &AttrValue
) -> T {
    match attr {
        AttrValue::String(s) => {
            serde_json::from_str(s).unwrap_or_else(|_| T::default())
        }
        _ => T::default(),
    }
}

/// Validated state that maintains invariants
pub struct ValidatedState {
    inner: AppState,
    selected_index: ValidIndex,
    session_index: ValidIndex,
}

impl ValidatedState {
    /// Create new validated state
    pub fn new() -> Self {
        let inner = AppState::new();
        Self {
            selected_index: ValidIndex::new(0, inner.search_results.len()),
            session_index: ValidIndex::new(0, inner.session_filtered_indices.len()),
            inner,
        }
    }
    
    /// Get inner state reference
    pub fn inner(&self) -> &AppState {
        &self.inner
    }
    
    /// Get mutable inner state reference
    pub fn inner_mut(&mut self) -> &mut AppState {
        &mut self.inner
    }
    
    /// Get selected index
    pub fn selected_index(&self) -> usize {
        self.selected_index.get()
    }
    
    /// Move selection up
    pub fn move_up(&mut self) {
        self.selected_index = self.selected_index.decrement();
        self.inner.selected_index = self.selected_index.get();
    }
    
    /// Move selection down
    pub fn move_down(&mut self) {
        self.selected_index = self.selected_index.increment();
        self.inner.selected_index = self.selected_index.get();
    }
    
    /// Move selection by page
    pub fn move_page(&mut self, up: bool) {
        const PAGE_SIZE: isize = 10;
        let offset = if up { -PAGE_SIZE } else { PAGE_SIZE };
        self.selected_index = self.selected_index.offset(offset);
        self.inner.selected_index = self.selected_index.get();
    }
    
    /// Jump to home
    pub fn move_home(&mut self) {
        self.selected_index = self.selected_index.home();
        self.inner.selected_index = self.selected_index.get();
    }
    
    /// Jump to end
    pub fn move_end(&mut self) {
        self.selected_index = self.selected_index.end();
        self.inner.selected_index = self.selected_index.get();
    }
    
    /// Get selected result safely
    pub fn get_selected_result(&self) -> Option<&SearchResult> {
        self.inner.search_results.get(self.selected_index.get())
    }
    
    /// Update search results and fix indices
    pub fn set_search_results(&mut self, results: Vec<SearchResult>) {
        let len = results.len();
        self.inner.search_results = results;
        self.selected_index = ValidIndex::new(self.selected_index.get(), len);
        self.inner.selected_index = self.selected_index.get();
    }
    
    /// Update session messages and fix indices
    pub fn set_session_messages(&mut self, messages: Vec<String>) {
        let len = messages.len();
        self.inner.session_messages = messages;
        self.inner.session_filtered_indices = (0..len).collect();
        self.session_index = ValidIndex::new(self.session_index.get(), len);
        self.inner.selected_index = self.session_index.get();
    }
}



/// Safe state operations
pub trait SafeStateOps {
    /// Safely get a result at index
    fn safe_get_result(&self, index: usize) -> AppResult<&SearchResult>;
    
    /// Safely get current selected result
    fn safe_get_selected_result(&self) -> AppResult<&SearchResult>;
    
    /// Safely set selected index
    fn safe_set_selected_index(&mut self, index: usize) -> AppResult<()>;
}

impl SafeStateOps for AppState {
    fn safe_get_result(&self, index: usize) -> AppResult<&SearchResult> {
        self.search_results.get(index)
            .ok_or_else(|| AppError::InvalidIndex { 
                index, 
                max: self.search_results.len() 
            })
    }
    
    fn safe_get_selected_result(&self) -> AppResult<&SearchResult> {
        self.safe_get_result(self.selected_index)
    }
    
    fn safe_set_selected_index(&mut self, index: usize) -> AppResult<()> {
        if index < self.search_results.len() {
            self.selected_index = index;
            Ok(())
        } else {
            Err(AppError::InvalidIndex { 
                index, 
                max: self.search_results.len() 
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_valid_index() {
        // Test creation
        let idx = ValidIndex::new(5, 10);
        assert_eq!(idx.get(), 5);
        
        // Test clamping
        let idx = ValidIndex::new(15, 10);
        assert_eq!(idx.get(), 9);
        
        // Test empty
        let idx = ValidIndex::new(0, 0);
        assert_eq!(idx.get(), 0);
        
        // Test increment
        let idx = ValidIndex::new(5, 10);
        assert_eq!(idx.increment().get(), 6);
        
        // Test increment at max
        let idx = ValidIndex::new(9, 10);
        assert_eq!(idx.increment().get(), 9);
        
        // Test decrement
        let idx = ValidIndex::new(5, 10);
        assert_eq!(idx.decrement().get(), 4);
        
        // Test decrement at zero
        let idx = ValidIndex::new(0, 10);
        assert_eq!(idx.decrement().get(), 0);
    }
    
    #[test]
    fn test_message_builder() {
        // Valid result detail
        let msg = MessageBuilder::enter_result_detail(5, 10);
        assert!(msg.is_some());
        
        // Invalid result detail
        let msg = MessageBuilder::enter_result_detail(10, 10);
        assert!(msg.is_none());
        
        // Valid query
        let msg = MessageBuilder::search_query_changed("test".to_string());
        assert!(msg.is_ok());
        
        // Too long query
        let msg = MessageBuilder::search_query_changed("x".repeat(1001));
        assert!(msg.is_err());
    }
    
    #[test]
    fn test_component_props() {
        #[derive(Serialize, Deserialize, PartialEq, Debug)]
        struct TestData {
            value: i32,
            name: String,
        }
        
        let data = TestData {
            value: 42,
            name: "test".to_string(),
        };
        
        let props = ComponentProps::new(data.clone()).unwrap();
        assert_eq!(props.get_data(), &data);
        
        // Test serialization
        if let AttrValue::String(s) = props.as_attr_value() {
            let deserialized: TestData = serde_json::from_str(&s).unwrap();
            assert_eq!(deserialized, data);
        } else {
            panic!("Expected AttrValue::String");
        }
    }
    
    #[test]
    fn test_safe_state_ops() {
        let mut state = AppState::new();
        state.search_results = vec![
            SearchResult::default(),
            SearchResult::default(),
        ];
        
        // Valid operations
        assert!(state.safe_get_result(0).is_ok());
        assert!(state.safe_get_result(1).is_ok());
        assert!(state.safe_set_selected_index(1).is_ok());
        
        // Invalid operations
        assert!(state.safe_get_result(2).is_err());
        assert!(state.safe_set_selected_index(2).is_err());
    }
}