use anyhow::Result;
use chrono::DateTime;
use crossbeam::channel;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::sync::Arc;

use super::engine::SearchEngineTrait;
use super::file_discovery::{discover_claude_files, expand_tilde};
use crate::interactive_ratatui::domain::models::SearchOrder;
use crate::query::{QueryCondition, SearchOptions, SearchResult};
use crate::schemas::SessionMessage;
use crate::utils::path_encoding;

pub struct RayonEngine {
    options: SearchOptions,
}

impl RayonEngine {
    pub fn new(options: SearchOptions) -> Self {
        Self { options }
    }
}

impl SearchEngineTrait for RayonEngine {
    fn search(
        &self,
        pattern: &str,
        query: QueryCondition,
    ) -> Result<(Vec<SearchResult>, std::time::Duration, usize)> {
        self.search_with_role_filter(pattern, query, None)
    }

    fn search_with_role_filter(
        &self,
        pattern: &str,
        query: QueryCondition,
        role_filter: Option<String>,
    ) -> Result<(Vec<SearchResult>, std::time::Duration, usize)> {
        self.search_with_role_filter_and_order(pattern, query, role_filter, SearchOrder::Descending)
    }

    fn search_with_role_filter_and_order(
        &self,
        pattern: &str,
        query: QueryCondition,
        role_filter: Option<String>,
        order: SearchOrder,
    ) -> Result<(Vec<SearchResult>, std::time::Duration, usize)> {
        let start_time = std::time::Instant::now();

        // Discover files
        let file_discovery_start = std::time::Instant::now();
        let expanded_pattern = expand_tilde(pattern);
        let files = if expanded_pattern.is_file() {
            vec![expanded_pattern]
        } else {
            discover_claude_files(Some(pattern))?
        };
        let file_discovery_time = file_discovery_start.elapsed();

        if self.options.verbose {
            eprintln!(
                "File discovery took: {}ms ({} files found)",
                file_discovery_time.as_millis(),
                files.len()
            );
        }

        if files.is_empty() {
            return Ok((Vec::new(), start_time.elapsed(), 0));
        }

        // Channel for collecting results
        let (sender, receiver) = channel::unbounded();

        // Process files in parallel using Rayon
        let search_start = std::time::Instant::now();

        let query = Arc::new(query);
        let options = Arc::new(self.options.clone());

        // Process files in parallel
        rayon::scope(|s| {
            for file_path in files {
                let sender = sender.clone();
                let query = query.clone();
                let options = options.clone();

                s.spawn(move |_| {
                    if let Ok(results) = search_file(&file_path, &query, &options) {
                        for result in results {
                            let _ = sender.send(result);
                        }
                    }
                });
            }
        });

        // Drop the original sender so the receiver knows when all tasks are done
        drop(sender);

        // Collect all results
        let mut all_results = Vec::new();
        while let Ok(result) = receiver.recv() {
            all_results.push(result);
        }

        let search_time = search_start.elapsed();

        // Apply filters
        self.apply_filters(&mut all_results, role_filter)?;

        // Sort by timestamp
        match order {
            SearchOrder::Descending => {
                all_results.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
            }
            SearchOrder::Ascending => {
                all_results.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
            }
        }

        let total_count = all_results.len();

        // Only truncate if max_results is specified
        if let Some(limit) = self.options.max_results {
            all_results.truncate(limit);
        }

        let elapsed = start_time.elapsed();

        if self.options.verbose {
            eprintln!("\nPerformance breakdown:");
            eprintln!("  File discovery: {}ms", file_discovery_time.as_millis());
            eprintln!("  Search: {}ms", search_time.as_millis());
            eprintln!("  Total: {}ms", elapsed.as_millis());
        }

        Ok((all_results, elapsed, total_count))
    }
}

impl RayonEngine {
    fn apply_filters(
        &self,
        results: &mut Vec<SearchResult>,
        role_filter: Option<String>,
    ) -> Result<()> {
        // Apply message ID filter (highest priority)
        if let Some(ref message_id) = self.options.message_id {
            results.retain(|r| &r.uuid == message_id);
        }

        // Apply role filter
        if let Some(role) = role_filter {
            results.retain(|r| r.role == role);
        }

        // Apply session filter
        if let Some(ref session_id) = self.options.session_id {
            results.retain(|r| &r.session_id == session_id);
        }

        // Apply time filters
        if let Some(ref after) = self.options.after {
            if let Ok(after_dt) = DateTime::parse_from_rfc3339(after) {
                results.retain(|r| {
                    DateTime::parse_from_rfc3339(&r.timestamp)
                        .map(|dt| dt >= after_dt)
                        .unwrap_or(false)
                });
            }
        }

        if let Some(ref before) = self.options.before {
            if let Ok(before_dt) = DateTime::parse_from_rfc3339(before) {
                results.retain(|r| {
                    DateTime::parse_from_rfc3339(&r.timestamp)
                        .map(|dt| dt <= before_dt)
                        .unwrap_or(false)
                });
            }
        }

        Ok(())
    }
}

// Helper function to search a single file synchronously
pub(super) fn search_file(
    file_path: &Path,
    query: &QueryCondition,
    options: &SearchOptions,
) -> Result<Vec<SearchResult>> {
    let file = File::open(file_path)?;
    let metadata = file.metadata()?;
    // Use same buffer size as Smol for fair comparison
    let mut reader = BufReader::with_capacity(64 * 1024, file);

    // Get file creation time for fallback
    // Use platform-specific approach like main branch
    let file_ctime = Some(&metadata)
        .and_then(|m| {
            // Try to get creation time (birth time) first
            #[cfg(target_os = "macos")]
            {
                m.created().ok()
            }
            // Fall back to modified time on other systems
            #[cfg(not(target_os = "macos"))]
            {
                m.modified().ok()
            }
        })
        .map(|t| {
            let duration = t.duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
            let ctime =
                chrono::DateTime::<chrono::Utc>::from_timestamp(duration.as_secs() as i64, 0)
                    .unwrap_or_else(chrono::Utc::now)
                    .to_rfc3339();
            if options.verbose {
                eprintln!("DEBUG: file_ctime for {file_path:?} = {ctime}");
            }
            ctime
        })
        .unwrap_or_else(|| {
            let now = chrono::Utc::now().to_rfc3339();
            if options.verbose {
                eprintln!("DEBUG: Using current time as fallback: {now}");
            }
            now
        });

    let mut results = Vec::with_capacity(256); // Same capacity as Smol
    let mut latest_timestamp: Option<String> = None;
    let mut first_timestamp: Option<String> = None;
    let mut line_buffer = Vec::with_capacity(16 * 1024); // Same buffer size as Smol
    let mut is_first_line = true;
    let mut found_summary_first = false;

    loop {
        line_buffer.clear();
        let bytes_read = reader.read_until(b'\n', &mut line_buffer)?;
        if bytes_read == 0 {
            break; // EOF
        }

        // Skip empty lines
        if line_buffer.trim_ascii().is_empty() {
            continue;
        }

        // Remove newline if present
        if line_buffer.ends_with(b"\n") {
            line_buffer.pop();
            if line_buffer.ends_with(b"\r") {
                line_buffer.pop();
            }
        }

        // Parse JSON - Always use sonic-rs for optimized engine
        // Use from_slice to avoid UTF-8 string conversion
        let message: Result<SessionMessage, _> = sonic_rs::from_slice(&line_buffer);

        match message {
            Ok(message) => {
                // Check if first message is summary
                if is_first_line {
                    is_first_line = false;
                    if message.get_type() == "summary" {
                        found_summary_first = true;
                        if options.verbose {
                            eprintln!("DEBUG: Found summary at first line in {file_path:?}");
                        }
                    }
                }

                // Update timestamps
                if let Some(ts) = message.get_timestamp() {
                    latest_timestamp = Some(ts.to_string());
                    // Track first timestamp after summary for summary messages
                    if first_timestamp.is_none() && found_summary_first {
                        first_timestamp = Some(ts.to_string());
                        if options.verbose {
                            eprintln!(
                                "DEBUG: Found first timestamp '{ts}' after summary in {file_path:?}"
                            );
                        }
                    }
                }

                // Get searchable text
                let text = message.get_searchable_text();

                // Apply query condition
                if let Ok(matches) = query.evaluate(&text) {
                    if matches {
                        // Apply inline filters
                        if let Some(role) = &options.role {
                            // For summary messages, only match if explicitly filtering for "summary"
                            if message.get_type() == "summary" {
                                if role != "summary" {
                                    continue;
                                }
                            } else if message.get_type() != role {
                                continue;
                            }
                        }

                        if let Some(session_id) = &options.session_id {
                            if message.get_session_id() != Some(session_id) {
                                continue;
                            }
                        }

                        // Check project_path filter (matches against file path)
                        if let Some(project_path) = &options.project_path {
                            let file_path_str = file_path.to_string_lossy();
                            if !path_encoding::file_belongs_to_project(&file_path_str, project_path)
                            {
                                continue;
                            }
                        }

                        // Create result
                        let timestamp = if message.get_type() == "summary" {
                            // Use first non-summary timestamp or file ctime
                            first_timestamp
                                .as_ref()
                                .or(latest_timestamp.as_ref())
                                .cloned()
                                .unwrap_or_else(|| file_ctime.clone())
                        } else {
                            message
                                .get_timestamp()
                                .map(|s| s.to_string())
                                .unwrap_or_else(|| file_ctime.clone())
                        };

                        // For SessionViewer and message details, we need raw_json
                        let raw_json =
                            if options.session_id.is_some() || options.message_id.is_some() {
                                // Convert line_buffer to String for raw_json
                                Some(String::from_utf8_lossy(&line_buffer).to_string())
                            } else {
                                None
                            };
                        results.push(SearchResult {
                            timestamp,
                            role: message.get_type().to_string(),
                            text,
                            file: file_path.display().to_string(),
                            uuid: message.get_uuid().unwrap_or("").to_string(),
                            session_id: message.get_session_id().unwrap_or("").to_string(),
                            query: query.clone(),
                            cwd: message.get_cwd().unwrap_or("").to_string(),
                            message_type: message.get_type().to_string(),
                            raw_json,
                        });
                    }
                }
            }
            Err(e) => {
                if options.verbose {
                    eprintln!("Failed to parse JSON in {file_path:?}: {e}");
                }
                // Continue processing other lines
            }
        }
    }

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::parse_query;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_search_engine() -> Result<()> {
        let temp_dir = tempdir()?;
        let test_file = temp_dir.path().join("test.jsonl");

        // Create test data
        let mut file = File::create(&test_file)?;
        writeln!(
            file,
            r#"{{"type":"user","message":{{"role":"user","content":"Hello world"}},"uuid":"123","timestamp":"2024-01-01T00:00:00Z","sessionId":"session1","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}}"#
        )?;
        writeln!(
            file,
            r#"{{"type":"assistant","message":{{"id":"msg1","type":"message","role":"assistant","model":"claude","content":[{{"type":"text","text":"Hi there!"}}],"stop_reason":"end_turn","stop_sequence":null,"usage":{{"input_tokens":10,"cache_creation_input_tokens":0,"cache_read_input_tokens":0,"output_tokens":5}}}},"uuid":"124","timestamp":"2024-01-01T00:00:01Z","sessionId":"session1","parentUuid":"123","isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}}"#
        )?;

        // Search for "Hello"
        let options = SearchOptions::default();
        let engine = RayonEngine::new(options);
        let query = parse_query("Hello")?;
        let (results, _, _) = engine.search(test_file.to_str().unwrap(), query)?;

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].role, "user");
        assert!(results[0].text.contains("Hello world"));

        Ok(())
    }

    #[test]
    fn test_role_filter() -> Result<()> {
        let temp_dir = tempdir()?;
        let test_file = temp_dir.path().join("test.jsonl");

        // Create test data with different roles
        let mut file = File::create(&test_file)?;
        writeln!(
            file,
            r#"{{"type":"user","message":{{"role":"user","content":"test message"}},"uuid":"1","timestamp":"2024-01-01T00:00:00Z","sessionId":"s1","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/","version":"1"}}"#
        )?;
        writeln!(
            file,
            r#"{{"type":"system","content":"test message","uuid":"2","timestamp":"2024-01-01T00:00:01Z","sessionId":"s1","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/","version":"1","isMeta":false}}"#
        )?;

        // Search with role filter
        let options = SearchOptions {
            role: Some("user".to_string()),
            ..Default::default()
        };

        let engine = RayonEngine::new(options);
        let query = parse_query("test")?;
        let (results, _, _) = engine.search(test_file.to_str().unwrap(), query)?;

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].role, "user");

        Ok(())
    }
}
