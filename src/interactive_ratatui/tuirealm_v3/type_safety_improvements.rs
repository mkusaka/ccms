/// Type safety improvements for tui-realm v3
/// 
/// Problem: AttrValue only supports primitive types, leading to type unsafety
/// when passing complex data structures.

use std::marker::PhantomData;
use tuirealm::props::AttrValue;
use serde::{Serialize, Deserialize};

/// Type-safe wrapper for AttrValue that preserves type information
#[derive(Debug, Clone)]
pub struct TypedAttrValue<T> {
    value: AttrValue,
    _phantom: PhantomData<T>,
}

impl<T: Serialize + for<'de> Deserialize<'de>> TypedAttrValue<T> {
    /// Create a new typed attribute value
    pub fn new(data: &T) -> Result<Self, serde_json::Error> {
        let json = serde_json::to_string(data)?;
        Ok(Self {
            value: AttrValue::String(json),
            _phantom: PhantomData,
        })
    }
    
    /// Extract the typed value
    pub fn get(&self) -> Result<T, serde_json::Error> {
        match &self.value {
            AttrValue::String(s) => serde_json::from_str(s),
            _ => Err(serde_json::Error::custom("Invalid AttrValue type")),
        }
    }
    
    /// Get the underlying AttrValue for tui-realm
    pub fn into_attr_value(self) -> AttrValue {
        self.value
    }
}

/// Type-safe attribute keys
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TypedAttribute {
    SearchResults,
    SessionMessages,
    ErrorInfo,
}

/// Type-safe component properties
pub struct TypedProps {
    search_results: Option<Vec<crate::query::condition::SearchResult>>,
    session_messages: Option<Vec<String>>,
    error_info: Option<crate::interactive_ratatui::tuirealm_v3::error::RecoverableError>,
}

impl TypedProps {
    /// Convert to AttrValue with type safety
    pub fn to_attr_value<T: Serialize>(&self, attr: TypedAttribute, data: &T) -> Result<AttrValue, serde_json::Error> {
        let typed = TypedAttrValue::new(data)?;
        Ok(typed.into_attr_value())
    }
    
    /// Extract typed value from AttrValue
    pub fn from_attr_value<T: for<'de> Deserialize<'de>>(attr_value: &AttrValue) -> Result<T, serde_json::Error> {
        match attr_value {
            AttrValue::String(s) => serde_json::from_str(s),
            _ => Err(serde_json::Error::custom("Expected String AttrValue")),
        }
    }
}

/// Alternative: Use indices and shared state
pub struct SharedStateIndex(usize);

pub struct SharedStateManager<T> {
    items: Vec<T>,
}

impl<T> SharedStateManager<T> {
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }
    
    pub fn store(&mut self, item: T) -> SharedStateIndex {
        let index = self.items.len();
        self.items.push(item);
        SharedStateIndex(index)
    }
    
    pub fn get(&self, index: &SharedStateIndex) -> Option<&T> {
        self.items.get(index.0)
    }
    
    pub fn get_mut(&mut self, index: &SharedStateIndex) -> Option<&mut T> {
        self.items.get_mut(index.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::condition::SearchResult;
    
    #[test]
    fn test_typed_attr_value() {
        let results = vec![
            SearchResult {
                file: "test.jsonl".to_string(),
                line_number: 1,
                session_id: "test-session".to_string(),
                timestamp: "2024-01-01T00:00:00Z".to_string(),
                role: "user".to_string(),
                text: "test message".to_string(),
                raw_json: None,
            }
        ];
        
        let typed = TypedAttrValue::new(&results).unwrap();
        let attr_value = typed.clone().into_attr_value();
        
        // Verify we can round-trip the data
        let recovered: Vec<SearchResult> = TypedProps::from_attr_value(&attr_value).unwrap();
        assert_eq!(recovered.len(), 1);
        assert_eq!(recovered[0].text, "test message");
    }
    
    #[test]
    fn test_shared_state_manager() {
        let mut manager = SharedStateManager::new();
        let data = vec!["item1".to_string(), "item2".to_string()];
        
        let index = manager.store(data.clone());
        let retrieved = manager.get(&index).unwrap();
        
        assert_eq!(retrieved, &data);
    }
}