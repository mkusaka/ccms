use anyhow::{Context, Result};
use chrono::DateTime;
use crossbeam::channel;
use indicatif::{ProgressBar, ProgressStyle};
use rayon::prelude::*;
use simd_json;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

use super::file_discovery::{discover_claude_files, expand_tilde};
use crate::interactive_ratatui::domain::models::SearchOrder;
use crate::query::{QueryCondition, SearchOptions, SearchResult};
use crate::schemas::SessionMessage;

pub struct SearchEngine {
    options: SearchOptions,
}

impl SearchEngine {
    pub fn new(options: SearchOptions) -> Self {
        Self { options }
    }

    pub fn search(
        &self,
        pattern: &str,
        query: QueryCondition,
    ) -> Result<(Vec<SearchResult>, std::time::Duration, usize)> {
        self.search_with_role_filter(pattern, query, None)
    }

    pub fn search_with_role_filter(
        &self,
        pattern: &str,
        query: QueryCondition,
        role_filter: Option<String>,
    ) -> Result<(Vec<SearchResult>, std::time::Duration, usize)> {
        self.search_with_role_filter_and_order(pattern, query, role_filter, SearchOrder::Descending)
    }

    pub fn search_with_role_filter_and_order(
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

        // Progress bar - only show for operations with many files or when explicitly verbose
        let progress = if self.options.verbose && files.len() > 100 {
            let pb = ProgressBar::new(files.len() as u64);
            pb.set_style(
                ProgressStyle::default_bar()
                    .template("[{elapsed_precise}] {bar:40} {pos}/{len} files")?
                    .progress_chars("=>-"),
            );
            Some(pb)
        } else {
            None
        };

        // Channel for collecting results
        let (sender, receiver) = channel::unbounded();
        let max_results = self.options.max_results.unwrap_or(50);

        // Process files in parallel
        let search_start = std::time::Instant::now();
        files.par_iter().for_each_with(sender, |s, file_path| {
            if let Some(pb) = &progress {
                pb.inc(1);
            }

            if let Ok(results) = self.search_file(file_path, &query) {
                for result in results {
                    // Send result through channel
                    let _ = s.send(result);
                }
            }
        });
        let search_time = search_start.elapsed();

        // Collect results
        drop(progress);
        let mut all_results: Vec<SearchResult> = receiver.try_iter().collect();

        // Don't deduplicate - match TypeScript behavior which returns all matches
        // Commenting out deduplication logic
        // let mut seen_uuids = std::collections::HashSet::new();
        // all_results.retain(|result| {
        //     if result.uuid.is_empty() {
        //         // Keep all results with empty UUIDs (summary messages)
        //         true
        //     } else {
        //         // For non-empty UUIDs, only keep if not seen before
        //         seen_uuids.insert(result.uuid.clone())
        //     }
        // });

        // Apply filters
        self.apply_filters(&mut all_results, role_filter)?;

        // Sort by timestamp based on search order
        match order {
            SearchOrder::Descending => {
                // Newest first
                all_results.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
            }
            SearchOrder::Ascending => {
                // Oldest first
                all_results.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
            }
        }

        // Store total count before truncating
        let total_count = all_results.len();

        // Limit results
        all_results.truncate(max_results);

        let elapsed = start_time.elapsed();

        if self.options.verbose {
            eprintln!("\nPerformance breakdown:");
            eprintln!("  File discovery: {}ms", file_discovery_time.as_millis());
            eprintln!("  Search: {}ms", search_time.as_millis());
            eprintln!(
                "  Post-processing: {}ms",
                elapsed
                    .saturating_sub(file_discovery_time)
                    .saturating_sub(search_time)
                    .as_millis()
            );
            eprintln!("  Total: {}ms", elapsed.as_millis());
        }

        Ok((all_results, elapsed, total_count))
    }

    fn search_file(&self, file_path: &Path, query: &QueryCondition) -> Result<Vec<SearchResult>> {
        let file =
            File::open(file_path).with_context(|| format!("Failed to open file: {file_path:?}"))?;

        // Get file metadata
        let metadata = file.metadata()?;
        let _file_size = metadata.len();

        // Get file creation time for summary messages
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
                if self.options.verbose {
                    eprintln!("DEBUG: file_ctime for {file_path:?} = {ctime}");
                }
                ctime
            })
            .unwrap_or_else(|| {
                let now = chrono::Utc::now().to_rfc3339();
                if self.options.verbose {
                    eprintln!("DEBUG: Using current time as fallback: {now}");
                }
                now
            });

        // Use optimized buffer size for JSONL files
        let reader = BufReader::with_capacity(32 * 1024, file);
        let lines: Vec<String> = reader.lines().collect::<Result<Vec<_>, _>>()?;

        // Second pass: find first timestamp if first message is summary
        let mut first_timestamp: Option<String> = None;
        if !lines.is_empty() {
            let mut first_line_bytes = lines[0].as_bytes().to_vec();
            if let Ok(first_msg) =
                simd_json::serde::from_slice::<SessionMessage>(&mut first_line_bytes)
            {
                if first_msg.get_type() == "summary" {
                    if self.options.verbose {
                        eprintln!("DEBUG: Found summary at first line in {file_path:?}");
                    }
                    // Look for first timestamp in subsequent messages
                    for (idx, line) in lines.iter().skip(1).enumerate() {
                        let mut line_bytes = line.as_bytes().to_vec();
                        if let Ok(msg) =
                            simd_json::serde::from_slice::<SessionMessage>(&mut line_bytes)
                        {
                            if let Some(ts) = msg.get_timestamp() {
                                first_timestamp = Some(ts.to_string());
                                if self.options.verbose {
                                    eprintln!(
                                        "DEBUG: Found timestamp '{}' at line {} in {:?}",
                                        ts,
                                        idx + 2,
                                        file_path
                                    );
                                }
                                break;
                            }
                        }
                    }
                    if first_timestamp.is_none() && self.options.verbose {
                        eprintln!("DEBUG: No timestamp found after summary in {file_path:?}");
                    }
                }
            }
        }

        let mut results = Vec::new();
        let mut latest_timestamp: Option<String> = None;

        for (line_num, line) in lines.iter().enumerate() {
            if line.trim().is_empty() {
                continue;
            }

            // Parse JSON with SIMD optimization
            let mut json_bytes = line.as_bytes().to_vec();
            match simd_json::serde::from_slice::<SessionMessage>(&mut json_bytes) {
                Ok(message) => {
                    // Update latest timestamp if this message has one
                    if let Some(ts) = message.get_timestamp() {
                        latest_timestamp = Some(ts.to_string());
                    }

                    // Extract searchable text (content + metadata)
                    let text = message.get_searchable_text();

                    // Apply query condition
                    if let Ok(matches) = query.evaluate(&text) {
                        if matches {
                            // Check role filter
                            if let Some(role) = &self.options.role {
                                if message.get_type() != role {
                                    continue;
                                }
                            }

                            // Check session filter
                            if let Some(session_id) = &self.options.session_id {
                                if message.get_session_id() != Some(session_id) {
                                    continue;
                                }
                            }

                            // Check project path filter
                            if let Some(project_filter) = &self.options.project_path {
                                let project_path = Self::extract_project_path(file_path);
                                if !project_path.starts_with(project_filter) {
                                    continue;
                                }
                            }

                            let msg_timestamp = message.get_timestamp();
                            if self.options.verbose && message.get_type() == "summary" {
                                eprintln!(
                                    "DEBUG: Processing summary message, uuid={}, original_timestamp={:?}",
                                    message.get_uuid().unwrap_or("NO_UUID"),
                                    msg_timestamp
                                );
                            }

                            let final_timestamp = msg_timestamp
                                    .map(|s| {
                                        if self.options.verbose && message.get_type() == "summary" {
                                            eprintln!("DEBUG: Summary has its own timestamp: {s}");
                                        }
                                        s.to_string()
                                    })
                                    .or_else(|| {
                                        // For summary messages, prefer first_timestamp over latest_timestamp
                                        if message.get_type() == "summary" {
                                            if self.options.verbose {
                                                eprintln!("DEBUG: Summary message without timestamp, first_timestamp={first_timestamp:?}, latest_timestamp={latest_timestamp:?}");
                                            }
                                            first_timestamp.clone()
                                        } else {
                                            latest_timestamp.clone()
                                        }
                                    })
                                    .unwrap_or_else(|| {
                                        if self.options.verbose && message.get_type() == "summary" {
                                            eprintln!("DEBUG: Using file_ctime '{file_ctime}' for summary as fallback");
                                        }
                                        file_ctime.clone()
                                    });

                            if self.options.verbose && message.get_type() == "summary" {
                                eprintln!("DEBUG: Final timestamp for summary: {final_timestamp}");
                            }

                            results.push(SearchResult {
                                file: file_path.to_string_lossy().to_string(),
                                uuid: message.get_uuid().unwrap_or("").to_string(),
                                timestamp: final_timestamp,
                                session_id: message.get_session_id().unwrap_or("").to_string(),
                                role: message.get_type().to_string(),
                                text: message.get_content_text(),
                                has_tools: message.has_tool_use(),
                                has_thinking: message.has_thinking(),
                                message_type: message.get_type().to_string(),
                                query: query.clone(),
                                project_path: Self::extract_project_path(file_path),
                                raw_json: Some(line.clone()),
                            });
                        }
                    }
                }
                Err(e) => {
                    if self.options.verbose {
                        eprintln!(
                            "Failed to parse JSON at line {} in {:?}: {}",
                            line_num + 1,
                            file_path,
                            e
                        );
                    }
                }
            }
        }
        Ok(results)
    }

    fn apply_filters(
        &self,
        results: &mut Vec<SearchResult>,
        role_filter: Option<String>,
    ) -> Result<()> {
        // Apply role filter from interactive UI (if provided)
        if let Some(role) = role_filter {
            results.retain(|r| r.role == role);
        }

        // Apply role filter from command line options
        if let Some(role) = &self.options.role {
            results.retain(|r| r.role == *role);
        }
        // Apply timestamp filters
        if let Some(before) = &self.options.before {
            let before_time =
                DateTime::parse_from_rfc3339(before).context("Invalid 'before' timestamp")?;
            results.retain(|r| {
                if let Ok(time) = DateTime::parse_from_rfc3339(&r.timestamp) {
                    time < before_time
                } else {
                    false
                }
            });
        }

        if let Some(after) = &self.options.after {
            let after_time =
                DateTime::parse_from_rfc3339(after).context("Invalid 'after' timestamp")?;
            results.retain(|r| {
                if let Ok(time) = DateTime::parse_from_rfc3339(&r.timestamp) {
                    time > after_time
                } else {
                    false
                }
            });
        }

        Ok(())
    }

    fn extract_project_path(file_path: &Path) -> String {
        // Extract project path from file path
        // Format: ~/.claude/projects/{encoded-project-path}/{session-id}.jsonl
        if let Some(parent) = file_path.parent() {
            if let Some(project_name) = parent.file_name() {
                if let Some(project_str) = project_name.to_str() {
                    // Decode the project path (replace hyphens with slashes)
                    return project_str.replace('-', "/");
                }
            }
        }
        String::new()
    }
}

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
        let engine = SearchEngine::new(options);
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

        let engine = SearchEngine::new(options);
        let query = parse_query("test")?;
        let (results, _, _) = engine.search(test_file.to_str().unwrap(), query)?;

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].role, "user");

        Ok(())
    }

    #[test]
    fn test_complex_query() -> Result<()> {
        let temp_dir = tempdir()?;
        let test_file = temp_dir.path().join("test.jsonl");

        // Create test data
        let mut file = File::create(&test_file)?;
        writeln!(
            file,
            r#"{{"type":"user","message":{{"role":"user","content":"foo bar"}},"uuid":"1","timestamp":"2024-01-01T00:00:00Z","sessionId":"s1","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/","version":"1"}}"#
        )?;
        writeln!(
            file,
            r#"{{"type":"user","message":{{"role":"user","content":"foo baz"}},"uuid":"2","timestamp":"2024-01-01T00:00:01Z","sessionId":"s1","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/","version":"1"}}"#
        )?;
        writeln!(
            file,
            r#"{{"type":"user","message":{{"role":"user","content":"only bar"}},"uuid":"3","timestamp":"2024-01-01T00:00:02Z","sessionId":"s1","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/","version":"1"}}"#
        )?;

        // Search for "foo AND bar"
        let options = SearchOptions::default();
        let engine = SearchEngine::new(options);
        let query = parse_query("foo AND bar")?;
        let (results, _, _) = engine.search(test_file.to_str().unwrap(), query)?;

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].text, "foo bar");

        // Search for "foo OR bar"
        let query = parse_query("foo OR bar")?;
        let (results, _, _) = engine.search(test_file.to_str().unwrap(), query)?;

        assert_eq!(results.len(), 3);

        Ok(())
    }

    #[test]
    fn test_summary_message_handling() -> Result<()> {
        let temp_dir = tempdir()?;
        let test_file = temp_dir.path().join("test.jsonl");

        // Create test data with summary message
        let mut file = File::create(&test_file)?;
        writeln!(
            file,
            r#"{{"type":"summary","summary":"Starting new session","leafUuid":"leaf-123"}}"#
        )?;
        writeln!(
            file,
            r#"{{"type":"user","message":{{"role":"user","content":"Hello"}},"uuid":"1","timestamp":"2024-01-01T00:00:01Z","sessionId":"s1","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/","version":"1"}}"#
        )?;

        // Search for summary content
        let options = SearchOptions::default();
        let engine = SearchEngine::new(options);
        let query = parse_query("Starting")?;
        let (results, _, _) = engine.search(test_file.to_str().unwrap(), query)?;

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].role, "summary");
        assert!(results[0].text.contains("Starting new session"));

        Ok(())
    }

    #[test]
    fn test_timestamp_filtering() -> Result<()> {
        let temp_dir = tempdir()?;
        let test_file = temp_dir.path().join("test.jsonl");

        // Create test data with different timestamps
        let mut file = File::create(&test_file)?;
        writeln!(
            file,
            r#"{{"type":"user","message":{{"role":"user","content":"Early message"}},"uuid":"1","timestamp":"2024-01-01T00:00:00Z","sessionId":"s1","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/","version":"1"}}"#
        )?;
        writeln!(
            file,
            r#"{{"type":"user","message":{{"role":"user","content":"Middle message"}},"uuid":"2","timestamp":"2024-01-02T00:00:00Z","sessionId":"s1","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/","version":"1"}}"#
        )?;
        writeln!(
            file,
            r#"{{"type":"user","message":{{"role":"user","content":"Late message"}},"uuid":"3","timestamp":"2024-01-03T00:00:00Z","sessionId":"s1","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/","version":"1"}}"#
        )?;

        // Search with timestamp filter
        let options = SearchOptions {
            after: Some("2024-01-01T12:00:00Z".to_string()),
            before: Some("2024-01-02T12:00:00Z".to_string()),
            ..Default::default()
        };

        let engine = SearchEngine::new(options);
        let query = parse_query("message")?;
        let (results, _, _) = engine.search(test_file.to_str().unwrap(), query)?;

        assert_eq!(results.len(), 1);
        assert!(results[0].text.contains("Middle message"));

        Ok(())
    }

    #[test]
    fn test_timestamp_filtering_with_since() -> Result<()> {
        let temp_dir = tempdir()?;
        let test_file = temp_dir.path().join("test.jsonl");

        // Create test data with different timestamps
        let mut file = File::create(&test_file)?;

        // Use current time and relative times for more realistic test
        let now = chrono::Utc::now();
        let two_days_ago = now - chrono::Duration::days(2);
        let one_day_ago = now - chrono::Duration::days(1);
        let one_hour_ago = now - chrono::Duration::hours(1);

        writeln!(
            file,
            r#"{{"type":"user","message":{{"role":"user","content":"Old message"}},"uuid":"1","timestamp":"{}","sessionId":"s1","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/","version":"1"}}"#,
            two_days_ago.to_rfc3339()
        )?;
        writeln!(
            file,
            r#"{{"type":"user","message":{{"role":"user","content":"Yesterday message"}},"uuid":"2","timestamp":"{}","sessionId":"s1","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/","version":"1"}}"#,
            one_day_ago.to_rfc3339()
        )?;
        writeln!(
            file,
            r#"{{"type":"user","message":{{"role":"user","content":"Recent message"}},"uuid":"3","timestamp":"{}","sessionId":"s1","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/","version":"1"}}"#,
            one_hour_ago.to_rfc3339()
        )?;

        // Test filtering with a timestamp that's 1.5 days ago
        // Should only find the messages from yesterday and recent
        let since_time = (now - chrono::Duration::hours(36)).to_rfc3339();
        let options = SearchOptions {
            after: Some(since_time),
            ..Default::default()
        };

        let engine = SearchEngine::new(options);
        let query = parse_query("message")?;
        let (results, _, _) = engine.search(test_file.to_str().unwrap(), query)?;

        assert_eq!(results.len(), 2);

        // Results should be sorted by timestamp (newest first)
        assert!(results[0].text.contains("Recent message"));
        assert!(results[1].text.contains("Yesterday message"));

        Ok(())
    }

    #[test]
    fn test_session_id_filter() -> Result<()> {
        let temp_dir = tempdir()?;
        let test_file = temp_dir.path().join("test.jsonl");

        // Create test data with different session IDs
        let mut file = File::create(&test_file)?;
        writeln!(
            file,
            r#"{{"type":"user","message":{{"role":"user","content":"Session 1"}},"uuid":"1","timestamp":"2024-01-01T00:00:00Z","sessionId":"session1","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/","version":"1"}}"#
        )?;
        writeln!(
            file,
            r#"{{"type":"user","message":{{"role":"user","content":"Session 2"}},"uuid":"2","timestamp":"2024-01-01T00:00:01Z","sessionId":"session2","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/","version":"1"}}"#
        )?;

        // Search with session filter
        let options = SearchOptions {
            session_id: Some("session1".to_string()),
            ..Default::default()
        };

        let engine = SearchEngine::new(options);
        let query = parse_query("Session")?;
        let (results, _, _) = engine.search(test_file.to_str().unwrap(), query)?;

        assert_eq!(results.len(), 1);
        assert!(results[0].text.contains("Session 1"));

        Ok(())
    }

    #[test]
    fn test_max_results_limit() -> Result<()> {
        let temp_dir = tempdir()?;
        let test_file = temp_dir.path().join("test.jsonl");

        // Create test data with multiple matches
        let mut file = File::create(&test_file)?;
        for i in 0..10 {
            writeln!(
                file,
                r#"{{"type":"user","message":{{"role":"user","content":"Message {i}"}},"uuid":"{i}","timestamp":"2024-01-01T00:00:0{i}Z","sessionId":"s1","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/","version":"1"}}"#
            )?;
        }

        // Search with max results limit
        let options = SearchOptions {
            max_results: Some(3),
            ..Default::default()
        };

        let engine = SearchEngine::new(options);
        let query = parse_query("Message")?;
        let (results, _, total_count) = engine.search(test_file.to_str().unwrap(), query)?;

        assert_eq!(results.len(), 3);
        assert!(total_count >= 10);

        Ok(())
    }

    #[test]
    fn test_project_path_extraction() -> Result<()> {
        let project_path = SearchEngine::extract_project_path(Path::new(
            "/home/user/.claude/projects/-Users-project-name/session.jsonl",
        ));
        assert_eq!(project_path, "/Users/project/name");

        let project_path =
            SearchEngine::extract_project_path(Path::new("/invalid/path/file.jsonl"));
        assert_eq!(project_path, "path");

        Ok(())
    }

    #[test]
    fn test_project_path_filter() -> Result<()> {
        let temp_dir = tempdir()?;
        let projects_dir = temp_dir.path().join(".claude").join("projects");
        std::fs::create_dir_all(&projects_dir)?;

        // Create project directories
        let project1_dir = projects_dir.join("-Users-project1");
        let project2_dir = projects_dir.join("-Users-project2");
        std::fs::create_dir_all(&project1_dir)?;
        std::fs::create_dir_all(&project2_dir)?;

        // Create test files in different projects
        let file1 = project1_dir.join("test.jsonl");
        let file2 = project2_dir.join("test.jsonl");

        let mut f1 = File::create(&file1)?;
        writeln!(
            f1,
            r#"{{"type":"user","message":{{"role":"user","content":"Project 1 message"}},"uuid":"1","timestamp":"2024-01-01T00:00:00Z","sessionId":"s1","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/","version":"1"}}"#
        )?;

        let mut f2 = File::create(&file2)?;
        writeln!(
            f2,
            r#"{{"type":"user","message":{{"role":"user","content":"Project 2 message"}},"uuid":"2","timestamp":"2024-01-01T00:00:00Z","sessionId":"s2","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/","version":"1"}}"#
        )?;

        // Search with project filter
        let options = SearchOptions {
            project_path: Some("/Users/project1".to_string()),
            ..Default::default()
        };

        let engine = SearchEngine::new(options);
        let query = parse_query("message")?;

        // Search in both files pattern
        let pattern = projects_dir.join("*/*.jsonl");
        let (results, _, _) = engine.search(pattern.to_str().unwrap(), query)?;

        // Should only find results from project1
        assert_eq!(results.len(), 1);
        assert!(results[0].text.contains("Project 1"));

        Ok(())
    }

    #[test]
    fn test_regex_query() -> Result<()> {
        let temp_dir = tempdir()?;
        let test_file = temp_dir.path().join("test.jsonl");

        // Create test data
        let mut file = File::create(&test_file)?;
        writeln!(
            file,
            r#"{{"type":"user","message":{{"role":"user","content":"Error code: 404"}},"uuid":"1","timestamp":"2024-01-01T00:00:00Z","sessionId":"s1","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/","version":"1"}}"#
        )?;
        writeln!(
            file,
            r#"{{"type":"user","message":{{"role":"user","content":"Success code: 200"}},"uuid":"2","timestamp":"2024-01-01T00:00:01Z","sessionId":"s1","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/","version":"1"}}"#
        )?;

        // Search with regex
        let options = SearchOptions::default();
        let engine = SearchEngine::new(options);
        let query = parse_query(r"/Error.*\d+/")?;
        let (results, _, _) = engine.search(test_file.to_str().unwrap(), query)?;

        assert_eq!(results.len(), 1);
        assert!(results[0].text.contains("Error code: 404"));

        Ok(())
    }

    #[test]
    fn test_unicode_handling() -> Result<()> {
        let temp_dir = tempdir()?;
        let test_file = temp_dir.path().join("test.jsonl");

        // Create test data with unicode
        let mut file = File::create(&test_file)?;
        writeln!(
            file,
            r#"{{"type":"user","message":{{"role":"user","content":"こんにちは世界"}},"uuid":"1","timestamp":"2024-01-01T00:00:00Z","sessionId":"s1","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/","version":"1"}}"#
        )?;
        writeln!(
            file,
            r#"{{"type":"user","message":{{"role":"user","content":"Hello 世界"}},"uuid":"2","timestamp":"2024-01-01T00:00:01Z","sessionId":"s1","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/","version":"1"}}"#
        )?;

        // Search for Japanese text
        let options = SearchOptions::default();
        let engine = SearchEngine::new(options);
        let query = parse_query("世界")?;
        let (results, _, _) = engine.search(test_file.to_str().unwrap(), query)?;

        assert_eq!(results.len(), 2);

        Ok(())
    }

    #[test]
    fn test_very_long_lines() -> Result<()> {
        let temp_dir = tempdir()?;
        let test_file = temp_dir.path().join("test.jsonl");

        // Create a very long message
        let long_text = "a".repeat(10000);
        let mut file = File::create(&test_file)?;
        writeln!(
            file,
            r#"{{"type":"user","message":{{"role":"user","content":"{long_text}"}},"uuid":"1","timestamp":"2024-01-01T00:00:00Z","sessionId":"s1","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/","version":"1"}}"#
        )?;

        // Search should handle long lines
        let options = SearchOptions::default();
        let engine = SearchEngine::new(options);
        let query = parse_query("aaa")?;
        let (results, _, _) = engine.search(test_file.to_str().unwrap(), query)?;

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].text.len(), 10000);

        Ok(())
    }

    #[test]
    fn test_empty_content_messages() -> Result<()> {
        let temp_dir = tempdir()?;
        let test_file = temp_dir.path().join("test.jsonl");

        // Create messages with empty content
        let mut file = File::create(&test_file)?;
        writeln!(
            file,
            r#"{{"type":"user","message":{{"role":"user","content":""}},"uuid":"1","timestamp":"2024-01-01T00:00:00Z","sessionId":"s1","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/","version":"1"}}"#
        )?;
        writeln!(
            file,
            r#"{{"type":"assistant","message":{{"id":"msg1","type":"message","role":"assistant","model":"claude","content":[],"stop_reason":"end_turn","stop_sequence":null,"usage":{{"input_tokens":10,"cache_creation_input_tokens":0,"cache_read_input_tokens":0,"output_tokens":5}}}},"uuid":"2","timestamp":"2024-01-01T00:00:01Z","sessionId":"s1","parentUuid":"1","isSidechain":false,"userType":"external","cwd":"/","version":"1"}}"#
        )?;

        // Search should handle empty content
        let options = SearchOptions::default();
        let engine = SearchEngine::new(options);
        let query = parse_query(".*")?; // Match any
        let (results, _, _) = engine.search(test_file.to_str().unwrap(), query)?;

        // Empty content should not match
        assert_eq!(results.len(), 0);

        Ok(())
    }

    #[test]
    fn test_malformed_json_recovery() -> Result<()> {
        let temp_dir = tempdir()?;
        let test_file = temp_dir.path().join("test.jsonl");

        // Create file with mix of valid and invalid JSON
        let mut file = File::create(&test_file)?;
        writeln!(file, "{{invalid json")?;
        writeln!(
            file,
            r#"{{"type":"user","message":{{"role":"user","content":"Valid message 1"}},"uuid":"1","timestamp":"2024-01-01T00:00:00Z","sessionId":"s1","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/","version":"1"}}"#
        )?;
        writeln!(file, "null")?;
        writeln!(file, "{{}}")?;
        writeln!(
            file,
            r#"{{"type":"user","message":{{"role":"user","content":"Valid message 2"}},"uuid":"2","timestamp":"2024-01-01T00:00:01Z","sessionId":"s1","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/","version":"1"}}"#
        )?;

        // Should skip invalid lines and process valid ones
        let options = SearchOptions::default();
        let engine = SearchEngine::new(options);
        let query = parse_query("Valid")?;
        let (results, _, _) = engine.search(test_file.to_str().unwrap(), query)?;

        assert_eq!(results.len(), 2);

        Ok(())
    }

    #[test]
    fn test_special_characters_in_content() -> Result<()> {
        let temp_dir = tempdir()?;
        let test_file = temp_dir.path().join("test.jsonl");

        // Create messages with special characters
        let mut file = File::create(&test_file)?;
        writeln!(
            file,
            r#"{{"type":"user","message":{{"role":"user","content":"Special chars: \n\t\r\\ \" ' < > & ="}},"uuid":"1","timestamp":"2024-01-01T00:00:00Z","sessionId":"s1","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/","version":"1"}}"#
        )?;

        // Search for escaped characters
        let options = SearchOptions::default();
        let engine = SearchEngine::new(options);
        let query = parse_query(r#""\n\t\r\\""#)?;
        let (results, _, _) = engine.search(test_file.to_str().unwrap(), query)?;

        assert_eq!(results.len(), 1);
        // JSON parsing may decode escape sequences
        // Check if the content contains the special characters (decoded)
        let text = &results[0].text;
        assert!(
            text.contains('\n')
                || text.contains('\t')
                || text.contains('\r')
                || text.contains('\\')
        );

        Ok(())
    }

    #[test]
    fn test_search_by_session_id() -> Result<()> {
        let temp_dir = tempdir()?;
        let test_file = temp_dir.path().join("test.jsonl");

        // Create test data with different session IDs
        let mut file = File::create(&test_file)?;
        writeln!(
            file,
            r#"{{"type":"user","message":{{"role":"user","content":"Message in session 1"}},"uuid":"1","timestamp":"2024-01-01T00:00:00Z","sessionId":"session-abc-123","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/","version":"1"}}"#
        )?;
        writeln!(
            file,
            r#"{{"type":"user","message":{{"role":"user","content":"Message in session 2"}},"uuid":"2","timestamp":"2024-01-01T00:00:01Z","sessionId":"session-def-456","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/","version":"1"}}"#
        )?;
        writeln!(
            file,
            r#"{{"type":"user","message":{{"role":"user","content":"Another message in session 1"}},"uuid":"3","timestamp":"2024-01-01T00:00:02Z","sessionId":"session-abc-123","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/","version":"1"}}"#
        )?;

        // Search by session ID
        let options = SearchOptions::default();
        let engine = SearchEngine::new(options);
        let query = parse_query("session-abc-123")?;
        let (results, _, _) = engine.search(test_file.to_str().unwrap(), query)?;

        assert_eq!(results.len(), 2);
        assert!(results.iter().all(|r| r.session_id == "session-abc-123"));

        Ok(())
    }

    #[test]
    fn test_search_by_message_uuid() -> Result<()> {
        let temp_dir = tempdir()?;
        let test_file = temp_dir.path().join("test.jsonl");

        // Create test data with unique UUIDs
        let mut file = File::create(&test_file)?;
        writeln!(
            file,
            r#"{{"type":"user","message":{{"role":"user","content":"First message"}},"uuid":"unique-uuid-123","timestamp":"2024-01-01T00:00:00Z","sessionId":"session1","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/","version":"1"}}"#
        )?;
        writeln!(
            file,
            r#"{{"type":"user","message":{{"role":"user","content":"Second message"}},"uuid":"unique-uuid-456","timestamp":"2024-01-01T00:00:01Z","sessionId":"session1","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/","version":"1"}}"#
        )?;

        // Search by UUID
        let options = SearchOptions::default();
        let engine = SearchEngine::new(options);
        let query = parse_query("unique-uuid-123")?;
        let (results, _, _) = engine.search(test_file.to_str().unwrap(), query)?;

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].uuid, "unique-uuid-123");
        assert!(results[0].text.contains("First message"));

        Ok(())
    }

    #[test]
    fn test_combined_content_and_session_search() -> Result<()> {
        let temp_dir = tempdir()?;
        let test_file = temp_dir.path().join("test.jsonl");

        // Create test data
        let mut file = File::create(&test_file)?;
        writeln!(
            file,
            r#"{{"type":"user","message":{{"role":"user","content":"Error in production"}},"uuid":"1","timestamp":"2024-01-01T00:00:00Z","sessionId":"prod-session-123","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/","version":"1"}}"#
        )?;
        writeln!(
            file,
            r#"{{"type":"user","message":{{"role":"user","content":"Error in development"}},"uuid":"2","timestamp":"2024-01-01T00:00:01Z","sessionId":"dev-session-456","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/","version":"1"}}"#
        )?;
        writeln!(
            file,
            r#"{{"type":"user","message":{{"role":"user","content":"Success in production"}},"uuid":"3","timestamp":"2024-01-01T00:00:02Z","sessionId":"prod-session-123","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/","version":"1"}}"#
        )?;

        // Search for "error" AND session ID
        let options = SearchOptions::default();
        let engine = SearchEngine::new(options);
        let query = parse_query("error AND prod-session-123")?;
        let (results, _, _) = engine.search(test_file.to_str().unwrap(), query)?;

        assert_eq!(results.len(), 1);
        assert!(results[0].text.contains("Error in production"));
        assert_eq!(results[0].session_id, "prod-session-123");

        Ok(())
    }

    #[test]
    fn test_multiple_file_patterns() -> Result<()> {
        let temp_dir = tempdir()?;
        let projects_dir = temp_dir.path().join("projects");
        std::fs::create_dir_all(&projects_dir)?;

        // Create multiple files
        for i in 0..3 {
            let file_path = projects_dir.join(format!("session{i}.jsonl"));
            let mut file = File::create(&file_path)?;
            writeln!(
                file,
                r#"{{"type":"user","message":{{"role":"user","content":"Message {i}"}},"uuid":"{i}","timestamp":"2024-01-01T00:00:0{i}Z","sessionId":"s{i}","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/","version":"1"}}"#
            )?;
        }

        // Search with wildcard pattern
        let pattern = projects_dir.join("*.jsonl");
        let options = SearchOptions::default();
        let engine = SearchEngine::new(options);
        let query = parse_query("Message")?;
        let (results, _, _) = engine.search(pattern.to_str().unwrap(), query)?;

        assert_eq!(results.len(), 3);

        Ok(())
    }

    #[test]
    fn test_session_with_thinking_and_tools() -> Result<()> {
        let temp_dir = tempdir()?;
        let test_file = temp_dir.path().join("test.jsonl");

        // Create messages with thinking and tool use
        let mut file = File::create(&test_file)?;
        writeln!(
            file,
            r#"{{"type":"assistant","message":{{"id":"msg1","type":"message","role":"assistant","model":"claude","content":[{{"type":"thinking","thinking":"Let me analyze this..."}},{{"type":"text","text":"Here's my response"}}],"stop_reason":"end_turn","stop_sequence":null,"usage":{{"input_tokens":10,"cache_creation_input_tokens":0,"cache_read_input_tokens":0,"output_tokens":5}}}},"uuid":"1","timestamp":"2024-01-01T00:00:00Z","sessionId":"s1","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/","version":"1"}}"#
        )?;
        writeln!(
            file,
            r#"{{"type":"assistant","message":{{"id":"msg2","type":"message","role":"assistant","model":"claude","content":[{{"type":"tool_use","id":"tool1","name":"calculator","input":{{"a":1,"b":2}}}},{{"type":"text","text":"The result is 3"}}],"stop_reason":"end_turn","stop_sequence":null,"usage":{{"input_tokens":10,"cache_creation_input_tokens":0,"cache_read_input_tokens":0,"output_tokens":5}}}},"uuid":"2","timestamp":"2024-01-01T00:00:01Z","sessionId":"s1","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/","version":"1"}}"#
        )?;

        let options = SearchOptions::default();
        let engine = SearchEngine::new(options);
        let query = parse_query("response OR result")?;
        let (results, _, _) = engine.search(test_file.to_str().unwrap(), query)?;

        // The actual number of results depends on whether thinking content is searched
        assert!(!results.is_empty());
        // At least one message should have thinking or tools
        assert!(results.iter().any(|r| r.has_thinking || r.has_tools));

        Ok(())
    }
}
