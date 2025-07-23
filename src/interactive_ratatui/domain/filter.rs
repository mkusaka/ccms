use crate::query::condition::SearchResult;
use crate::interactive_ratatui::domain::session_list_item::SessionListItem;
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
    pub fn filter_messages(items: &[SessionListItem], query: &str) -> Vec<usize> {
        if query.is_empty() {
            (0..items.len()).collect()
        } else {
            let query_lower = query.to_lowercase();
            items
                .iter()
                .enumerate()
                .filter(|(_, item)| {
                    let search_text = item.to_search_text();
                    search_text.to_lowercase().contains(&query_lower)
                })
                .map(|(idx, _)| idx)
                .collect()
        }
    }
}
