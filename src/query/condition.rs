use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum QueryCondition {
    Literal {
        pattern: String,
        #[serde(rename = "caseSensitive")]
        case_sensitive: bool,
    },
    Regex {
        pattern: String,
        flags: String,
    },
    Not {
        condition: Box<QueryCondition>,
    },
    #[serde(rename = "AND")]
    And {
        conditions: Vec<QueryCondition>,
    },
    #[serde(rename = "OR")]
    Or {
        conditions: Vec<QueryCondition>,
    },
}

impl QueryCondition {
    pub fn evaluate(&self, text: &str) -> Result<bool, regex::Error> {
        match self {
            QueryCondition::Literal {
                pattern,
                case_sensitive,
            } => {
                if *case_sensitive {
                    Ok(text.contains(pattern))
                } else {
                    Ok(text.to_lowercase().contains(&pattern.to_lowercase()))
                }
            }
            QueryCondition::Regex { pattern, flags } => {
                let regex = super::regex_cache::get_or_compile_regex(pattern, flags)?;
                Ok(regex.is_match(text))
            }
            QueryCondition::Not { condition } => Ok(!condition.evaluate(text)?),
            QueryCondition::And { conditions } => {
                for condition in conditions {
                    if !condition.evaluate(text)? {
                        return Ok(false);
                    }
                }
                Ok(true)
            }
            QueryCondition::Or { conditions } => {
                for condition in conditions {
                    if condition.evaluate(text)? {
                        return Ok(true);
                    }
                }
                Ok(false)
            }
        }
    }

    pub fn find_match(&self, text: &str) -> Option<(usize, usize)> {
        match self {
            QueryCondition::Literal {
                pattern,
                case_sensitive,
            } => {
                if *case_sensitive {
                    text.find(pattern).map(|pos| (pos, pattern.len()))
                } else {
                    let lower_text = text.to_lowercase();
                    let lower_pattern = pattern.to_lowercase();
                    lower_text
                        .find(&lower_pattern)
                        .map(|pos| (pos, pattern.len()))
                }
            }
            QueryCondition::Regex { pattern, flags } => {
                if let Ok(regex) = super::regex_cache::get_or_compile_regex(pattern, flags) {
                    regex.find(text).map(|m| (m.start(), m.len()))
                } else {
                    None
                }
            }
            QueryCondition::Not { .. } => None,
            QueryCondition::And { conditions } => {
                // Return the first match from any condition
                for condition in conditions {
                    if let Some(m) = condition.find_match(text) {
                        return Some(m);
                    }
                }
                None
            }
            QueryCondition::Or { conditions } => {
                // Return the first match from any condition
                for condition in conditions {
                    if let Some(m) = condition.find_match(text) {
                        return Some(m);
                    }
                }
                None
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct SearchOptions {
    pub max_results: Option<usize>,
    pub role: Option<String>,
    pub session_id: Option<String>,
    pub before: Option<String>,
    pub after: Option<String>,
    pub verbose: bool,
    pub project_path: Option<String>,
}

impl Default for SearchOptions {
    fn default() -> Self {
        Self {
            max_results: Some(50),
            role: None,
            session_id: None,
            before: None,
            after: None,
            verbose: false,
            project_path: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub file: String,
    pub uuid: String,
    pub timestamp: String,
    pub session_id: String,
    pub role: String,
    pub text: String,
    pub has_tools: bool,
    pub has_thinking: bool,
    pub message_type: String,
    pub query: QueryCondition,
    pub project_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw_json: Option<String>,
}

use crate::interactive_ratatui::ui::components::list_item::ListItem;

impl ListItem for SearchResult {
    fn get_role(&self) -> &str {
        &self.role
    }

    fn get_timestamp(&self) -> &str {
        &self.timestamp
    }

    fn get_content(&self) -> &str {
        &self.text
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_literal_case_insensitive() {
        let condition = QueryCondition::Literal {
            pattern: "Hello".to_string(),
            case_sensitive: false,
        };

        assert!(condition.evaluate("hello world").unwrap());
        assert!(condition.evaluate("HELLO there").unwrap());
        assert!(condition.evaluate("Say Hello!").unwrap());
        assert!(!condition.evaluate("hi world").unwrap());
    }

    #[test]
    fn test_literal_case_sensitive() {
        let condition = QueryCondition::Literal {
            pattern: "Hello".to_string(),
            case_sensitive: true,
        };

        assert!(condition.evaluate("Hello world").unwrap());
        assert!(!condition.evaluate("hello world").unwrap());
        assert!(!condition.evaluate("HELLO world").unwrap());
    }

    #[test]
    fn test_regex_matching() {
        let condition = QueryCondition::Regex {
            pattern: r"error.*\d+".to_string(),
            flags: "i".to_string(),
        };

        assert!(
            condition
                .evaluate("Error: Connection failed with code 123")
                .unwrap()
        );
        assert!(condition.evaluate("ERROR in line 45").unwrap());
        assert!(!condition.evaluate("Error without number").unwrap());
    }

    #[test]
    fn test_regex_multiline() {
        let condition = QueryCondition::Regex {
            pattern: r"^Error:".to_string(),
            flags: "m".to_string(),
        };

        assert!(condition.evaluate("Error: at start").unwrap());
        assert!(condition.evaluate("Some text\nError: on new line").unwrap());
        assert!(!condition.evaluate("Some Error: in middle").unwrap());
    }

    #[test]
    fn test_not_condition() {
        let inner = QueryCondition::Literal {
            pattern: "error".to_string(),
            case_sensitive: false,
        };
        let condition = QueryCondition::Not {
            condition: Box::new(inner),
        };

        assert!(condition.evaluate("All is well").unwrap());
        assert!(!condition.evaluate("Error occurred").unwrap());
    }

    #[test]
    fn test_and_condition() {
        let conditions = vec![
            QueryCondition::Literal {
                pattern: "error".to_string(),
                case_sensitive: false,
            },
            QueryCondition::Literal {
                pattern: "connection".to_string(),
                case_sensitive: false,
            },
        ];
        let condition = QueryCondition::And { conditions };

        assert!(condition.evaluate("Error: Connection timeout").unwrap());
        assert!(!condition.evaluate("Error: File not found").unwrap());
        assert!(!condition.evaluate("Connection established").unwrap());
    }

    #[test]
    fn test_or_condition() {
        let conditions = vec![
            QueryCondition::Literal {
                pattern: "error".to_string(),
                case_sensitive: false,
            },
            QueryCondition::Literal {
                pattern: "warning".to_string(),
                case_sensitive: false,
            },
        ];
        let condition = QueryCondition::Or { conditions };

        assert!(condition.evaluate("Error occurred").unwrap());
        assert!(condition.evaluate("Warning: deprecated").unwrap());
        assert!(condition.evaluate("Error and Warning").unwrap());
        assert!(!condition.evaluate("All good").unwrap());
    }

    #[test]
    fn test_complex_nested_condition() {
        // (error OR warning) AND NOT test
        let or_condition = QueryCondition::Or {
            conditions: vec![
                QueryCondition::Literal {
                    pattern: "error".to_string(),
                    case_sensitive: false,
                },
                QueryCondition::Literal {
                    pattern: "warning".to_string(),
                    case_sensitive: false,
                },
            ],
        };

        let not_condition = QueryCondition::Not {
            condition: Box::new(QueryCondition::Literal {
                pattern: "test".to_string(),
                case_sensitive: false,
            }),
        };

        let condition = QueryCondition::And {
            conditions: vec![or_condition, not_condition],
        };

        assert!(condition.evaluate("Error in production").unwrap());
        assert!(condition.evaluate("Warning: deprecated function").unwrap());
        assert!(!condition.evaluate("Error in test suite").unwrap());
        assert!(!condition.evaluate("Info: all good").unwrap());
    }

    #[test]
    fn test_find_match_literal() {
        let condition = QueryCondition::Literal {
            pattern: "error".to_string(),
            case_sensitive: false,
        };

        let text = "Found an error here";
        let result = condition.find_match(text);
        assert!(result.is_some());

        let (start, len) = result.unwrap();
        assert_eq!(&text[start..start + len], "error");
    }

    #[test]
    fn test_find_match_regex() {
        let condition = QueryCondition::Regex {
            pattern: r"\d+".to_string(),
            flags: "".to_string(),
        };

        let text = "Error code: 404";
        let result = condition.find_match(text);
        assert!(result.is_some());

        let (start, len) = result.unwrap();
        assert_eq!(&text[start..start + len], "404");
    }

    #[test]
    fn test_invalid_regex_error() {
        let condition = QueryCondition::Regex {
            pattern: r"[invalid".to_string(),
            flags: "".to_string(),
        };

        assert!(condition.evaluate("test").is_err());
    }

    #[test]
    fn test_empty_and_condition() {
        let condition = QueryCondition::And { conditions: vec![] };

        // Empty AND should return true (all conditions are satisfied vacuously)
        assert!(condition.evaluate("anything").unwrap());
    }

    #[test]
    fn test_empty_or_condition() {
        let condition = QueryCondition::Or { conditions: vec![] };

        // Empty OR should return false (no conditions are satisfied)
        assert!(!condition.evaluate("anything").unwrap());
    }
}
