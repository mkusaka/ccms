use crate::interactive_ratatui::domain::session_list_item::SessionListItem;
#[cfg(test)]
use crate::query::condition::SearchResult;
#[cfg(test)]
use anyhow::Result;
use crate::query::fast_lowercase::FastLowercase;

#[cfg(test)]
pub struct SearchFilter {
    pub role_filter: Option<String>,
}

#[cfg(test)]
impl SearchFilter {
    pub fn new(role_filter: Option<String>) -> Self {
        Self { role_filter }
    }

    pub fn apply(&self, results: &mut Vec<SearchResult>) -> Result<()> {
        if let Some(role) = &self.role_filter {
            results.retain(|result| result.role.fast_to_lowercase() == role.fast_to_lowercase());
        }
        Ok(())
    }
}

pub struct SessionFilter;

impl SessionFilter {
    pub fn filter_messages(
        items: &[SessionListItem],
        query: &str,
        role_filter: &Option<String>,
    ) -> Vec<usize> {
        let query_lower = query.fast_to_lowercase();

        items
            .iter()
            .enumerate()
            .filter(|(_, item)| {
                // Apply role filter first
                if let Some(role) = role_filter {
                    if item.role.fast_to_lowercase() != role.fast_to_lowercase() {
                        return false;
                    }
                }

                // Then apply text filter
                if query.is_empty() {
                    true
                } else {
                    let search_text = item.to_search_text();
                    search_text.fast_to_lowercase().contains(&query_lower)
                }
            })
            .map(|(idx, _)| idx)
            .collect()
    }
}
