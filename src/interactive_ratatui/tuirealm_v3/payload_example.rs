/// Example implementation using native AttrValue::Payload
use tuirealm::props::{AttrValue, PropPayload, PropValue};
use crate::query::condition::{SearchResult, QueryCondition};
use std::collections::HashMap;

/// Convert SearchResult to AttrValue using Payload
pub fn search_result_to_payload(result: &SearchResult) -> AttrValue {
    let mut map = HashMap::new();
    
    map.insert("file".to_string(), PropValue::Str(result.file.clone()));
    map.insert("uuid".to_string(), PropValue::Str(result.uuid.clone()));
    map.insert("timestamp".to_string(), PropValue::Str(result.timestamp.clone()));
    map.insert("session_id".to_string(), PropValue::Str(result.session_id.clone()));
    map.insert("role".to_string(), PropValue::Str(result.role.clone()));
    map.insert("text".to_string(), PropValue::Str(result.text.clone()));
    map.insert("has_tools".to_string(), PropValue::Bool(result.has_tools));
    map.insert("has_thinking".to_string(), PropValue::Bool(result.has_thinking));
    map.insert("message_type".to_string(), PropValue::Str(result.message_type.clone()));
    map.insert("project_path".to_string(), PropValue::Str(result.project_path.clone()));
    
    // Query condition needs special handling - serialize to string
    let query_str = match &result.query {
        QueryCondition::Literal { pattern, case_sensitive } => {
            format!("literal:{}:{}", pattern, case_sensitive)
        }
        QueryCondition::And { conditions } => {
            format!("and:complex:{}", conditions.len()) // Simplified for example
        }
        QueryCondition::Or { conditions } => {
            format!("or:complex:{}", conditions.len()) // Simplified for example
        }
        QueryCondition::Not { condition } => {
            format!("not:complex") // Simplified for example
        }
        QueryCondition::Regex { pattern, flags } => {
            format!("regex:{}:{}", pattern, flags)
        }
    };
    map.insert("query".to_string(), PropValue::Str(query_str));
    
    // Optional field
    if let Some(raw_json) = &result.raw_json {
        map.insert("raw_json".to_string(), PropValue::Str(raw_json.clone()));
    }
    
    AttrValue::Payload(PropPayload::Map(map))
}

/// Convert SearchResults vector to AttrValue using Payload
pub fn search_results_to_payload(results: &[SearchResult]) -> AttrValue {
    // Since PropPayload::Vec expects Vec<PropValue>, not Vec<PropPayload>,
    // we need to use a different approach. We'll create a Map where each
    // result is stored with an index key.
    let mut outer_map = HashMap::new();
    
    // Store the count
    outer_map.insert("count".to_string(), PropValue::U32(results.len() as u32));
    
    // Store each result as a serialized string (simplified approach)
    for (i, r) in results.iter().enumerate() {
        let result_str = format!(
            "{}|{}|{}|{}|{}|{}|{}|{}|{}|{}",
            r.file, r.uuid, r.timestamp, r.session_id, r.role,
            r.text, r.has_tools, r.has_thinking, r.message_type, r.project_path
        );
        outer_map.insert(format!("result_{}", i), PropValue::Str(result_str));
    }
    
    AttrValue::Payload(PropPayload::Map(outer_map))
}

/// Extract SearchResult from a Payload Map
pub fn payload_to_search_result(payload: &PropPayload) -> Option<SearchResult> {
    if let PropPayload::Map(map) = payload {
        let file = get_string_from_map(map, "file")?;
        let uuid = get_string_from_map(map, "uuid")?;
        let timestamp = get_string_from_map(map, "timestamp")?;
        let session_id = get_string_from_map(map, "session_id")?;
        let role = get_string_from_map(map, "role")?;
        let text = get_string_from_map(map, "text")?;
        let has_tools = get_bool_from_map(map, "has_tools")?;
        let has_thinking = get_bool_from_map(map, "has_thinking")?;
        let message_type = get_string_from_map(map, "message_type")?;
        let project_path = get_string_from_map(map, "project_path")?;
        let raw_json = get_string_from_map(map, "raw_json");
        
        // For simplicity, default to a literal query
        let query = QueryCondition::Literal {
            pattern: "".to_string(),
            case_sensitive: false,
        };
        
        Some(SearchResult {
            file,
            uuid,
            timestamp,
            session_id,
            role,
            text,
            has_tools,
            has_thinking,
            message_type,
            query,
            project_path,
            raw_json,
        })
    } else {
        None
    }
}

/// Extract SearchResults from AttrValue
pub fn payload_to_search_results(attr: &AttrValue) -> Option<Vec<SearchResult>> {
    if let AttrValue::Payload(PropPayload::Map(map)) = attr {
        let count = match map.get("count")? {
            PropValue::U32(n) => *n as usize,
            _ => return None,
        };
        
        let mut results = Vec::new();
        for i in 0..count {
            if let Some(PropValue::Str(result_str)) = map.get(&format!("result_{}", i)) {
                // Parse the serialized string (simplified - in real code would need proper parsing)
                let parts: Vec<&str> = result_str.split('|').collect();
                if parts.len() >= 10 {
                    results.push(SearchResult {
                        file: parts[0].to_string(),
                        uuid: parts[1].to_string(),
                        timestamp: parts[2].to_string(),
                        session_id: parts[3].to_string(),
                        role: parts[4].to_string(),
                        text: parts[5].to_string(),
                        has_tools: parts[6].parse().ok()?,
                        has_thinking: parts[7].parse().ok()?,
                        message_type: parts[8].to_string(),
                        project_path: parts[9].to_string(),
                        query: QueryCondition::Literal {
                            pattern: "".to_string(),
                            case_sensitive: false,
                        },
                        raw_json: None,
                    });
                }
            }
        }
        
        Some(results)
    } else {
        None
    }
}

// Helper functions
fn get_string_from_map(map: &HashMap<String, PropValue>, key: &str) -> Option<String> {
    match map.get(key)? {
        PropValue::Str(s) => Some(s.clone()),
        _ => None,
    }
}

fn get_bool_from_map(map: &HashMap<String, PropValue>, key: &str) -> Option<bool> {
    match map.get(key)? {
        PropValue::Bool(b) => Some(*b),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_search_result_to_payload() {
        let result = SearchResult {
            file: "test.json".to_string(),
            uuid: "uuid1".to_string(),
            timestamp: "2024-01-01".to_string(),
            session_id: "123".to_string(),
            role: "user".to_string(),
            text: "test message".to_string(),
            has_tools: false,
            has_thinking: false,
            message_type: "message".to_string(),
            query: QueryCondition::Literal { 
                pattern: "test".to_string(), 
                case_sensitive: false 
            },
            project_path: "/test".to_string(),
            raw_json: None,
        };
        
        let attr_value = search_result_to_payload(&result);
        
        // Verify it's a Payload
        match attr_value {
            AttrValue::Payload(PropPayload::Map(map)) => {
                assert_eq!(
                    map.get("file"),
                    Some(&PropValue::Str("test.json".to_string()))
                );
                assert_eq!(
                    map.get("has_tools"),
                    Some(&PropValue::Bool(false))
                );
            }
            _ => panic!("Expected Payload with Map"),
        }
    }
    
    #[test]
    fn test_search_results_to_payload_and_back() {
        let results = vec![
            SearchResult {
                file: "test1.json".to_string(),
                uuid: "uuid1".to_string(),
                timestamp: "2024-01-01".to_string(),
                session_id: "123".to_string(),
                role: "user".to_string(),
                text: "test message 1".to_string(),
                has_tools: false,
                has_thinking: false,
                message_type: "message".to_string(),
                query: QueryCondition::Literal { 
                    pattern: "test".to_string(), 
                    case_sensitive: false 
                },
                project_path: "/test".to_string(),
                raw_json: None,
            },
            SearchResult {
                file: "test2.json".to_string(),
                uuid: "uuid2".to_string(),
                timestamp: "2024-01-02".to_string(),
                session_id: "456".to_string(),
                role: "assistant".to_string(),
                text: "test message 2".to_string(),
                has_tools: true,
                has_thinking: true,
                message_type: "message".to_string(),
                query: QueryCondition::Literal { 
                    pattern: "test".to_string(), 
                    case_sensitive: false 
                },
                project_path: "/test".to_string(),
                raw_json: Some("{}".to_string()),
            },
        ];
        
        let attr_value = search_results_to_payload(&results);
        let retrieved = payload_to_search_results(&attr_value);
        
        assert!(retrieved.is_some());
        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.len(), 2);
        assert_eq!(retrieved[0].file, "test1.json");
        assert_eq!(retrieved[1].file, "test2.json");
        assert_eq!(retrieved[0].has_tools, false);
        assert_eq!(retrieved[1].has_tools, true);
    }
}