use anyhow::Result;
use chrono::DateTime;
use smol::channel;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::sync::Arc;

use super::file_discovery::{discover_claude_files, expand_tilde};
use crate::interactive_ratatui::domain::models::SearchOrder;
use crate::query::{QueryCondition, SearchOptions, SearchResult};
use crate::schemas::SessionMessage;

// Initialize blocking thread pool optimization
static INIT: std::sync::Once = std::sync::Once::new();

fn initialize_blocking_threads() {
    INIT.call_once(|| {
        // Only set if not already set by user
        if std::env::var("BLOCKING_MAX_THREADS").is_err() {
            let cpu_count = num_cpus::get();
            unsafe {
                std::env::set_var("BLOCKING_MAX_THREADS", cpu_count.to_string());
            }
            eprintln!("Optimized BLOCKING_MAX_THREADS to {cpu_count} (CPU count)");
        }
    });
}

pub struct SearchEngine {
    options: SearchOptions,
}

impl SearchEngine {
    pub fn new(options: SearchOptions) -> Self {
        // Initialize blocking threads optimization on first use
        initialize_blocking_threads();
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
        // Use smol's block_on to run the async search synchronously
        smol::block_on(async {
            self.search_async(pattern, query, role_filter, order).await
        })
    }

    async fn search_async(
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
        let max_results = self.options.max_results.unwrap_or(50);

        // Process files concurrently using multi-threaded executor
        let search_start = std::time::Instant::now();
        
        let query = Arc::new(query);
        let options = Arc::new(self.options.clone());
        
        // Spawn tasks for each file on the global executor
        let mut tasks = Vec::new();
        for file_path in files {
            let sender = sender.clone();
            let query = query.clone();
            let options = options.clone();
            
            let task = smol::spawn(async move {
                if let Ok(results) = search_file(&file_path, &query, &options).await {
                    for result in results {
                        let _ = sender.send(result).await;
                    }
                }
            });
            tasks.push(task);
        }
        
        // Drop the original sender so the receiver knows when all tasks are done
        drop(sender);
        
        // Run all tasks concurrently
        let search_future = async {
            for task in tasks {
                task.await;
            }
        };
        
        // Collect results while processing
        let collect_future = async {
            let mut all_results = Vec::new();
            while let Ok(result) = receiver.recv().await {
                all_results.push(result);
            }
            all_results
        };
        
        // Run search and collection concurrently
        let (_, mut all_results) = futures_lite::future::zip(search_future, collect_future).await;
        
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
        all_results.truncate(max_results);

        let elapsed = start_time.elapsed();

        if self.options.verbose {
            eprintln!("\nPerformance breakdown:");
            eprintln!("  File discovery: {}ms", file_discovery_time.as_millis());
            eprintln!("  Search: {}ms", search_time.as_millis());
            eprintln!("  Total: {}ms", elapsed.as_millis());
        }

        Ok((all_results, elapsed, total_count))
    }


    fn apply_filters(
        &self,
        results: &mut Vec<SearchResult>,
        role_filter: Option<String>,
    ) -> Result<()> {
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

// Helper function to search a single file using blocking I/O with optimized buffer
async fn search_file(
    file_path: &Path,
    query: &QueryCondition,
    options: &SearchOptions,
) -> Result<Vec<SearchResult>> {
    let file_path_owned = file_path.to_owned();
    let query_owned = query.clone();
    let options_owned = options.clone();
    
    // Use smol's blocking executor with larger buffer for better throughput
    blocking::unblock(move || {
        let file = File::open(&file_path_owned)?;
        let metadata = file.metadata()?;
        // Increase buffer size for better I/O performance
        let mut reader = BufReader::with_capacity(64 * 1024, file); // Changed to 64KB like basic Smol
        
        // Get file creation time for fallback
        let file_ctime = metadata
            .created()
            .ok()
            .and_then(|created| {
                created
                    .duration_since(std::time::UNIX_EPOCH)
                    .ok()
                    .map(|d| {
                        let timestamp = chrono::DateTime::from_timestamp(d.as_secs() as i64, 0)
                            .unwrap_or_else(chrono::Utc::now);
                        timestamp.to_rfc3339()
                    })
            })
            .unwrap_or_else(|| chrono::Utc::now().to_rfc3339());
        
        let mut results = Vec::with_capacity(256); // 4x larger initial capacity to reduce reallocations
        let mut latest_timestamp: Option<String> = None;
        let mut first_timestamp: Option<String> = None;
        let mut line_buffer = Vec::with_capacity(16 * 1024); // 2x larger reusable line buffer
        
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
                // Update timestamps
                if let Some(ts) = message.get_timestamp() {
                    latest_timestamp = Some(ts.to_string());
                    // Track first non-summary message timestamp
                    if first_timestamp.is_none() && message.get_type() != "summary" {
                        first_timestamp = Some(ts.to_string());
                    }
                }
                
                // Get searchable text
                let text = message.get_searchable_text();
                
                // Apply query condition
                if let Ok(matches) = query_owned.evaluate(&text) {
                    if matches {
                        // Apply inline filters
                        if let Some(role) = &options_owned.role {
                            if message.get_type() != role {
                                continue;
                            }
                        }
                        
                        if let Some(session_id) = &options_owned.session_id {
                            if message.get_session_id() != Some(session_id) {
                                continue;
                            }
                        }
                        
                        // Check project_path filter (matches against cwd field)
                        if let Some(project_path) = &options_owned.project_path {
                            if let Some(cwd) = message.get_cwd() {
                                if !cwd.starts_with(project_path) {
                                    continue;
                                }
                            }
                        }
                        
                        // Determine timestamp based on message type (matching Rayon logic)
                        let final_timestamp = message
                            .get_timestamp()
                            .map(|ts| ts.to_string())
                            .or_else(|| {
                                // For summary messages, prefer first_timestamp over latest_timestamp
                                if message.get_type() == "summary" {
                                    first_timestamp.clone()
                                } else {
                                    latest_timestamp.clone()
                                }
                            })
                            .unwrap_or_else(|| file_ctime.clone());
                        
                        let result = SearchResult {
                            file: file_path_owned.to_string_lossy().to_string(),
                            uuid: message.get_uuid().unwrap_or("").to_string(),
                            timestamp: final_timestamp,
                            session_id: message.get_session_id().unwrap_or("").to_string(),
                            role: message.get_type().to_string(),
                            text: message.get_content_text(),
                            has_tools: message.has_tool_use(),
                            has_thinking: message.has_thinking(),
                            message_type: message.get_type().to_string(),
                            query: query_owned.clone(),
                            cwd: message.get_cwd().unwrap_or("").to_string(),
                            raw_json: None,
                        };
                        results.push(result);
                    }
                }
                }
                Err(e) => {
                    if options_owned.verbose {
                        eprintln!(
                            "Failed to parse JSON in {:?}: {:?}",
                            file_path_owned,
                            e
                        );
                    }
                }
            }
        }
        
        Ok(results)
    }).await
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
    fn test_cwd_filter() -> Result<()> {
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
            r#"{{"type":"user","message":{{"role":"user","content":"Project 1 message"}},"uuid":"1","timestamp":"2024-01-01T00:00:00Z","sessionId":"s1","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/Users/project1","version":"1"}}"#
        )?;

        let mut f2 = File::create(&file2)?;
        writeln!(
            f2,
            r#"{{"type":"user","message":{{"role":"user","content":"Project 2 message"}},"uuid":"2","timestamp":"2024-01-01T00:00:00Z","sessionId":"s2","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/Users/project2","version":"1"}}"#
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
