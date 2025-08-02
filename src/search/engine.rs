use crate::interactive_ratatui::domain::models::SearchOrder;
use crate::query::{QueryCondition, SearchResult};
use anyhow::Result;
use chrono::DateTime;

/// Trait defining the interface for search engines
pub trait SearchEngineTrait {
    fn search(
        &self,
        pattern: &str,
        query: QueryCondition,
    ) -> Result<(Vec<SearchResult>, std::time::Duration, usize)>;

    fn search_with_role_filter(
        &self,
        pattern: &str,
        query: QueryCondition,
        role_filter: Option<String>,
    ) -> Result<(Vec<SearchResult>, std::time::Duration, usize)>;

    fn search_with_role_filter_and_order(
        &self,
        pattern: &str,
        query: QueryCondition,
        role_filter: Option<String>,
        order: SearchOrder,
    ) -> Result<(Vec<SearchResult>, std::time::Duration, usize)>;
}

/// Format a search result for display
pub fn format_search_result(result: &SearchResult, use_color: bool, full_text: bool) -> String {
    use colored::Colorize;

    let timestamp = if let Ok(dt) = DateTime::parse_from_rfc3339(&result.timestamp) {
        dt.format("%Y-%m-%d %H:%M:%S").to_string()
    } else {
        result.timestamp.clone()
    };

    // Format text preview similar to TypeScript implementation
    let text_preview = if full_text {
        result.text.clone()
    } else {
        format_preview(&result.text, &result.query, 150)
    };

    if use_color {
        format!(
            "{} {} [{}] {}\n  {}",
            timestamp.bright_blue(),
            result.role.bright_yellow(),
            result.file.bright_green(),
            result.uuid.dimmed(),
            text_preview
        )
    } else {
        format!(
            "{} {} [{}] {}\n  {}",
            timestamp, result.role, result.file, result.uuid, text_preview
        )
    }
}

/// Format text preview with context around match
fn format_preview(text: &str, query: &QueryCondition, context_length: usize) -> String {
    // Find the first match position
    let match_info = query.find_match(text);

    let (preview_text, has_prefix, has_suffix, _match_in_preview) =
        if let Some((start, len)) = match_info {
            // Show context around the match
            let context_before = 50;
            let context_after = context_length.saturating_sub(context_before);

            let preview_start = start.saturating_sub(context_before);
            let preview_end = (start + len + context_after).min(text.len());

            // Handle UTF-8 boundaries
            let mut actual_start = preview_start;
            while actual_start > 0 && !text.is_char_boundary(actual_start) {
                actual_start -= 1;
            }

            let mut actual_end = preview_end;
            while actual_end < text.len() && !text.is_char_boundary(actual_end) {
                actual_end += 1;
            }

            let preview = &text[actual_start..actual_end];
            let match_start_in_preview = start.saturating_sub(actual_start);

            (
                preview.to_string(),
                actual_start > 0,
                actual_end < text.len(),
                Some((match_start_in_preview, len)),
            )
        } else {
            // No match found, show beginning of text
            let end = context_length.min(text.len());
            let mut actual_end = end;
            while actual_end < text.len() && !text.is_char_boundary(actual_end) {
                actual_end += 1;
            }

            (
                text[..actual_end].to_string(),
                false,
                actual_end < text.len(),
                None,
            )
        };

    // Clean up whitespace
    let cleaned = preview_text
        .replace('\n', " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");

    // Apply highlighting and ellipsis
    let mut result = cleaned;

    // No longer need to highlight matches - removed color highlighting code

    // Add ellipsis
    if has_prefix {
        result = format!("...{result}");
    }
    if has_suffix {
        result = format!("{result}...");
    }

    result
}
