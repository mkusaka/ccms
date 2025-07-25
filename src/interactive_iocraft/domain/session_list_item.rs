// use crate::interactive_iocraft::ui::components::list_item::ListItem;
use chrono;

#[derive(Debug, Clone)]
pub struct SessionListItem {
    #[allow(dead_code)]
    pub index: usize,
    pub raw_json: String,
    pub role: String,
    pub timestamp: String,
    pub content: String,
    pub text: String, // For compatibility with SessionViewer
}

impl SessionListItem {
    /// Formats the timestamp for display
    fn format_timestamp(&self) -> String {
        if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(&self.timestamp) {
            dt.format("%m/%d %H:%M").to_string()
        } else if self.timestamp.len() >= 16 {
            self.timestamp.chars().take(16).collect()
        } else {
            self.timestamp.clone()
        }
    }

    /// Generates searchable text that matches what the user sees in the display
    /// Format: "{formatted_timestamp} {role} {content}"
    pub fn to_search_text(&self) -> String {
        let formatted_timestamp = self.format_timestamp();
        format!("{} {} {}", formatted_timestamp, self.role, self.content)
    }

    pub fn from_json_line(index: usize, json_line: &str) -> Option<Self> {
        if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(json_line) {
            // Extract role/type
            let role = json_value
                .get("type")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string();

            // Extract timestamp
            let timestamp = json_value
                .get("timestamp")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            // Extract content based on message type
            let content = match role.as_str() {
                "summary" => json_value
                    .get("summary")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                "system" => json_value
                    .get("content")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                _ => {
                    // For user and assistant messages
                    if let Some(content) = json_value
                        .get("message")
                        .and_then(|m| m.get("content"))
                        .and_then(|c| c.as_str())
                    {
                        content.to_string()
                    } else if let Some(arr) = json_value
                        .get("message")
                        .and_then(|m| m.get("content"))
                        .and_then(|c| c.as_array())
                    {
                        let texts: Vec<String> = arr
                            .iter()
                            .filter_map(|item| {
                                item.get("text")
                                    .and_then(|t| t.as_str())
                                    .map(|s| s.to_string())
                            })
                            .collect();
                        texts.join(" ")
                    } else {
                        String::new()
                    }
                }
            };

            Some(Self {
                index,
                raw_json: json_line.to_string(),
                role: role.clone(),
                timestamp,
                content: content.clone(),
                text: content, // Use content as text
            })
        } else {
            None
        }
    }
}

// ListItem trait is not needed for iocraft implementation
