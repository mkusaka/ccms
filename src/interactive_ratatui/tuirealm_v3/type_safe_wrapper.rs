/// Type-safe wrapper layer for AttrValue limitations
use tuirealm::props::AttrValue;
use serde::{Serialize, Deserialize};
use crate::query::condition::SearchResult;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

/// Trait for types that can be safely converted to/from AttrValue
pub trait TypeSafeAttr: Sized {
    fn to_attr_value(&self) -> AttrValue;
    fn from_attr_value(value: &AttrValue) -> Option<Self>;
}

/// Wrapper for passing complex types through AttrValue using an ID system
pub struct TypeSafeStore {
    storage: Arc<Mutex<HashMap<String, Vec<u8>>>>,
    counter: Arc<Mutex<u64>>,
}

impl TypeSafeStore {
    pub fn new() -> Self {
        Self {
            storage: Arc::new(Mutex::new(HashMap::new())),
            counter: Arc::new(Mutex::new(0)),
        }
    }
    
    /// Store a value and return its ID
    pub fn store<T: Serialize>(&self, value: &T) -> String {
        let json = serde_json::to_string(value).expect("Failed to serialize to JSON");
        let data = json.as_bytes().to_vec();
        
        // Handle potential mutex poisoning gracefully
        let mut counter = match self.counter.lock() {
            Ok(guard) => guard,
            Err(poisoned) => {
                eprintln!("Warning: Counter mutex was poisoned, recovering...");
                poisoned.into_inner()
            }
        };
        let id = format!("id_{}", *counter);
        *counter += 1;
        
        let mut storage = match self.storage.lock() {
            Ok(guard) => guard,
            Err(poisoned) => {
                eprintln!("Warning: Storage mutex was poisoned, recovering...");
                poisoned.into_inner()
            }
        };
        storage.insert(id.clone(), data);
        
        id
    }
    
    /// Retrieve a value by ID
    pub fn retrieve<T: for<'de> Deserialize<'de>>(&self, id: &str) -> Option<T> {
        let storage = match self.storage.lock() {
            Ok(guard) => guard,
            Err(poisoned) => {
                eprintln!("Warning: Storage mutex was poisoned during retrieve, recovering...");
                poisoned.into_inner()
            }
        };
        let data = storage.get(id)?;
        let json = String::from_utf8(data.clone()).ok()?;
        serde_json::from_str::<T>(&json).ok()
    }
    
    /// Clean up stored value
    pub fn remove(&self, id: &str) {
        let mut storage = match self.storage.lock() {
            Ok(guard) => guard,
            Err(poisoned) => {
                eprintln!("Warning: Storage mutex was poisoned during remove, recovering...");
                poisoned.into_inner()
            }
        };
        storage.remove(id);
    }
}

/// Global type-safe store instance
pub static TYPE_SAFE_STORE: OnceLock<TypeSafeStore> = OnceLock::new();

fn get_type_safe_store() -> &'static TypeSafeStore {
    TYPE_SAFE_STORE.get_or_init(|| TypeSafeStore::new())
}

/// Wrapper for SearchResult vector
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResults(pub Vec<SearchResult>);

impl TypeSafeAttr for SearchResults {
    fn to_attr_value(&self) -> AttrValue {
        let id = get_type_safe_store().store(self);
        AttrValue::String(format!("__type_safe__{}", id))
    }
    
    fn from_attr_value(value: &AttrValue) -> Option<Self> {
        match value {
            AttrValue::String(s) if s.starts_with("__type_safe__") => {
                let id = s.strip_prefix("__type_safe__")?;
                get_type_safe_store().retrieve(id)
            }
            _ => None,
        }
    }
}

/// Wrapper for session messages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMessages(pub Vec<String>);

impl TypeSafeAttr for SessionMessages {
    fn to_attr_value(&self) -> AttrValue {
        let id = get_type_safe_store().store(self);
        AttrValue::String(format!("__type_safe__{}", id))
    }
    
    fn from_attr_value(value: &AttrValue) -> Option<Self> {
        match value {
            AttrValue::String(s) if s.starts_with("__type_safe__") => {
                let id = s.strip_prefix("__type_safe__")?;
                get_type_safe_store().retrieve(id)
            }
            _ => None,
        }
    }
}

/// Helper functions for common operations
pub mod helpers {
    use super::*;
    use tuirealm::Application;
    use tuirealm::props::Attribute;
    use crate::interactive_ratatui::tuirealm_v3::messages::{ComponentId, AppMessage};
    use tuirealm::NoUserEvent;
    
    /// Set a type-safe attribute on a component
    pub fn set_type_safe_attr<T: TypeSafeAttr>(
        app: &mut Application<ComponentId, AppMessage, NoUserEvent>,
        component: &ComponentId,
        attr: Attribute,
        value: T,
    ) -> Result<(), String> {
        app.attr(component, attr, value.to_attr_value())
            .map_err(|e| e.to_string())
    }
    
    /// Get a type-safe attribute from a component
    pub fn get_type_safe_attr<T: TypeSafeAttr>(
        app: &Application<ComponentId, AppMessage, NoUserEvent>,
        component: &ComponentId,
        attr: Attribute,
    ) -> Option<T> {
        app.query(component, attr)
            .ok()
            .flatten()
            .and_then(|v| T::from_attr_value(&v))
    }
}

/// Macro for creating type-safe wrappers
#[macro_export]
macro_rules! type_safe_wrapper {
    ($name:ident, $inner:ty) => {
        #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
        pub struct $name(pub $inner);
        
        impl $crate::interactive_ratatui::tuirealm_v3::type_safe_wrapper::TypeSafeAttr for $name {
            fn to_attr_value(&self) -> tuirealm::props::AttrValue {
                let id = $crate::interactive_ratatui::tuirealm_v3::type_safe_wrapper::get_type_safe_store().store(self);
                tuirealm::props::AttrValue::String(format!("__type_safe__{}", id))
            }
            
            fn from_attr_value(value: &tuirealm::props::AttrValue) -> Option<Self> {
                match value {
                    tuirealm::props::AttrValue::String(s) if s.starts_with("__type_safe__") => {
                        let id = s.strip_prefix("__type_safe__")?;
                        $crate::interactive_ratatui::tuirealm_v3::type_safe_wrapper::get_type_safe_store().retrieve(id)
                    }
                    _ => None,
                }
            }
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_type_safe_store() {
        let store = TypeSafeStore::new();
        
        let data = vec!["test1".to_string(), "test2".to_string()];
        let id = store.store(&data);
        
        let retrieved: Option<Vec<String>> = store.retrieve(&id);
        assert_eq!(retrieved, Some(data));
        
        store.remove(&id);
        let retrieved_after: Option<Vec<String>> = store.retrieve(&id);
        assert_eq!(retrieved_after, None);
    }
    
    #[test]
    fn test_search_results_wrapper() {
        let results = SearchResults(vec![
            SearchResult {
                file: "test.json".to_string(),
                uuid: "uuid1".to_string(),
                timestamp: "2024-01-01".to_string(),
                session_id: "123".to_string(),
                role: "user".to_string(),
                text: "test message".to_string(),
                has_tools: false,
                has_thinking: false,
                message_type: "message".to_string(),
                query: crate::query::condition::QueryCondition::Literal { pattern: "test".to_string(), case_sensitive: false },
                project_path: "/test".to_string(),
                raw_json: None,
            }
        ]);
        
        let attr_value = results.to_attr_value();
        let retrieved = SearchResults::from_attr_value(&attr_value);
        
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().0.len(), 1);
    }
    
    #[test]
    fn test_payload_variant_exists() {
        // Check if AttrValue has Payload variant
        // This will fail to compile if Payload doesn't exist
        use tuirealm::props::{PropPayload, PropValue};
        
        let test_payload = AttrValue::Payload(
            PropPayload::One(PropValue::Str("test".to_string()))
        );
        
        match test_payload {
            AttrValue::Payload(_) => assert!(true),
            _ => assert!(false, "Should be Payload variant"),
        }
        
        // Also test Vec payload
        let vec_payload = AttrValue::Payload(PropPayload::Vec(vec![
            PropValue::Str("item1".to_string()),
            PropValue::Str("item2".to_string()),
        ]));
        
        match vec_payload {
            AttrValue::Payload(PropPayload::Vec(items)) => {
                assert_eq!(items.len(), 2);
            }
            _ => panic!("Should be Payload variant with Vec"),
        }
    }
}