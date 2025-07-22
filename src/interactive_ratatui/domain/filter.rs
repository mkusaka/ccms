use crate::query::condition::SearchResult;
use anyhow::Result;

pub struct SearchFilter {
    pub role_filter: Option<String>,
}

impl SearchFilter {
    pub fn new(role_filter: Option<String>) -> Self {
        Self { role_filter }
    }

    pub fn apply(&self, results: &mut Vec<SearchResult>) -> Result<()> {
        if let Some(role) = &self.role_filter {
            results.retain(|result| result.role.to_lowercase() == role.to_lowercase());
        }
        Ok(())
    }
}

pub struct SessionFilter;

impl SessionFilter {
    pub fn filter_messages(messages: &[String], query: &str) -> Vec<usize> {
        if query.is_empty() {
            (0..messages.len()).collect()
        } else {
            let query_lower = query.to_lowercase();
            messages
                .iter()
                .enumerate()
                .filter(|(_, msg)| msg.to_lowercase().contains(&query_lower))
                .map(|(idx, _)| idx)
                .collect()
        }
    }
}
