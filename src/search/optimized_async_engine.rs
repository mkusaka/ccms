use anyhow::{Context, Result};
use bytes::BytesMut;
use chrono::DateTime;
use std::fs::Metadata;
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, BufReader};
use tokio::sync::{mpsc, Semaphore};

use super::file_discovery::{discover_claude_files, expand_tilde};
use crate::interactive_ratatui::domain::models::SearchOrder;
use crate::query::{QueryCondition, SearchOptions, SearchResult};
use crate::schemas::SessionMessage;

/// Optimized async search engine using advanced tokio patterns
pub struct OptimizedAsyncSearchEngine {
    options: SearchOptions,
    /// Maximum concurrent file operations
    max_concurrent_files: usize,
    /// Buffer size for file reading
    buffer_size: usize,
    /// Whether to use hybrid rayon parsing
    use_hybrid_parsing: bool,
}

impl OptimizedAsyncSearchEngine {
    pub fn new(options: SearchOptions) -> Self {
        let num_cpus = num_cpus::get();
        Self {
            options,
            // Optimal concurrency: num_cpus * 2
            max_concurrent_files: num_cpus * 2,
            // Large buffer for efficient I/O
            buffer_size: 64 * 1024,
            // Enable hybrid parsing by default
            use_hybrid_parsing: true,
        }
    }
    
    pub fn with_concurrency(mut self, max_concurrent_files: usize) -> Self {
        self.max_concurrent_files = max_concurrent_files;
        self
    }
    
    pub fn with_buffer_size(mut self, buffer_size: usize) -> Self {
        self.buffer_size = buffer_size;
        self
    }
    
    pub fn with_hybrid_parsing(mut self, enabled: bool) -> Self {
        self.use_hybrid_parsing = enabled;
        self
    }

    pub async fn search(
        &self,
        pattern: &str,
        query: QueryCondition,
    ) -> Result<(Vec<SearchResult>, std::time::Duration, usize)> {
        self.search_with_role_filter(pattern, query, None).await
    }

    pub async fn search_with_role_filter(
        &self,
        pattern: &str,
        query: QueryCondition,
        role_filter: Option<String>,
    ) -> Result<(Vec<SearchResult>, std::time::Duration, usize)> {
        self.search_with_role_filter_and_order(pattern, query, role_filter, SearchOrder::Descending)
            .await
    }

    pub async fn search_with_role_filter_and_order(
        &self,
        pattern: &str,
        query: QueryCondition,
        role_filter: Option<String>,
        order: SearchOrder,
    ) -> Result<(Vec<SearchResult>, std::time::Duration, usize)> {
        let start_time = Instant::now();

        // Discover files
        let file_discovery_start = Instant::now();
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

        // Semaphore for controlling concurrent file operations
        let semaphore = Arc::new(Semaphore::new(self.max_concurrent_files));
        
        // Channel for streaming results
        let (tx, mut rx) = mpsc::channel::<SearchResult>(100);
        let max_results = self.options.max_results.unwrap_or(50);
        
        let search_start = Instant::now();
        
        // Spawn file processing tasks
        let query = Arc::new(query);
        let options = Arc::new(self.options.clone());
        let buffer_size = self.buffer_size;
        let use_hybrid_parsing = self.use_hybrid_parsing;
        
        // Spawn processor task
        let (files_tx, mut files_rx) = mpsc::channel(files.len());
        for file in files {
            let _ = files_tx.send(file).await;
        }
        drop(files_tx);
        
        let processor_handle = tokio::spawn(async move {
            let mut tasks = Vec::new();
            
            while let Some(file_path) = files_rx.recv().await {
                let semaphore = Arc::clone(&semaphore);
                let tx = tx.clone();
                let query = Arc::clone(&query);
                let options = Arc::clone(&options);
                
                let task = tokio::spawn(async move {
                    // Acquire permit for file operation
                    let _permit = semaphore.acquire().await?;
                    
                    if let Ok(results) = Self::search_file_optimized(
                        &file_path,
                        &query,
                        &options,
                        buffer_size,
                        use_hybrid_parsing,
                    ).await {
                        for result in results {
                            // Send results through channel
                            if tx.send(result).await.is_err() {
                                break; // Receiver dropped
                            }
                        }
                    }
                    Ok::<_, anyhow::Error>(())
                });
                
                tasks.push(task);
            }
            
            // Wait for all tasks to complete
            for task in tasks {
                if let Err(e) = task.await {
                    eprintln!("Task error: {}", e);
                }
            }
        });
        
        // Collect results from channel
        let mut all_results = Vec::new();
        let mut total_count = 0;
        
        while let Some(result) = rx.recv().await {
            total_count += 1;
            if all_results.len() < max_results * 2 {
                // Collect more than needed for sorting
                all_results.push(result);
            }
        }
        
        // Wait for processor to complete
        processor_handle.await?;
        
        let search_time = search_start.elapsed();

        // Apply filters
        self.apply_filters(&mut all_results, role_filter)?;

        // Sort by timestamp based on search order
        match order {
            SearchOrder::Descending => {
                all_results.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
            }
            SearchOrder::Ascending => {
                all_results.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
            }
        }

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

    async fn search_file_optimized(
        file_path: &Path,
        query: &QueryCondition,
        options: &SearchOptions,
        buffer_size: usize,
        use_hybrid_parsing: bool,
    ) -> Result<Vec<SearchResult>> {
        // Get file metadata
        let metadata = tokio::fs::metadata(file_path).await?;
        let file_size = metadata.len();
        
        // Get file creation time
        let file_ctime = extract_file_ctime(&metadata, options.verbose, file_path);
        
        // For small files, read entirely into memory
        if file_size < 1024 * 1024 {
            // 1MB threshold
            let content = tokio::fs::read(file_path).await
                .with_context(|| format!("Failed to read file: {file_path:?}"))?;
            
            // Parse in blocking pool
            let file_path = file_path.to_path_buf();
            let query = query.clone();
            let options = options.clone();
            let file_ctime = file_ctime.clone();
            
            return tokio::task::spawn_blocking(move || {
                parse_file_content(&content, &file_path, &query, &options, &file_ctime, use_hybrid_parsing)
            })
            .await?;
        }
        
        // For larger files, stream with buffered reader
        let file = File::open(file_path).await
            .with_context(|| format!("Failed to open file: {file_path:?}"))?;
        
        let mut reader = BufReader::with_capacity(buffer_size, file);
        let mut buffer = BytesMut::with_capacity(buffer_size * 2);
        let mut results = Vec::new();
        let mut latest_timestamp: Option<String> = None;
        let mut first_timestamp: Option<String> = None;
        let mut line_num = 0;
        
        // Read file in chunks
        loop {
            let bytes_read = reader.read_buf(&mut buffer).await?;
            if bytes_read == 0 {
                // Process any remaining data
                if !buffer.is_empty() {
                    let remaining = buffer.split();
                    if let Some(result) = process_line(
                        &remaining,
                        line_num,
                        file_path,
                        query,
                        options,
                        &file_ctime,
                        &mut latest_timestamp,
                        &mut first_timestamp,
                    ) {
                        results.push(result);
                    }
                }
                break;
            }
            
            // Process complete lines
            while let Some(newline_pos) = buffer.iter().position(|&b| b == b'\n') {
                let line = buffer.split_to(newline_pos + 1);
                line_num += 1;
                
                if let Some(result) = process_line(
                    &line[..line.len() - 1], // Remove newline
                    line_num,
                    file_path,
                    query,
                    options,
                    &file_ctime,
                    &mut latest_timestamp,
                    &mut first_timestamp,
                ) {
                    results.push(result);
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
}

fn parse_file_content(
    content: &[u8],
    file_path: &Path,
    query: &QueryCondition,
    options: &SearchOptions,
    file_ctime: &str,
    use_hybrid_parsing: bool,
) -> Result<Vec<SearchResult>> {
    if use_hybrid_parsing {
        // Use rayon for parallel line processing
        use rayon::prelude::*;
        
        let lines: Vec<&[u8]> = content
            .split(|&b| b == b'\n')
            .filter(|line| !line.is_empty())
            .collect();
        
        Ok(lines
            .par_iter()
            .enumerate()
            .filter_map(|(line_num, line)| {
                let mut latest_timestamp = None;
                let mut first_timestamp = None;
                process_line(
                    line,
                    line_num,
                    file_path,
                    query,
                    options,
                    file_ctime,
                    &mut latest_timestamp,
                    &mut first_timestamp,
                )
            })
            .collect())
    } else {
        // Sequential processing
        let mut results = Vec::new();
        let mut latest_timestamp = None;
        let mut first_timestamp = None;
        
        for (line_num, line) in content.split(|&b| b == b'\n').enumerate() {
            if line.is_empty() {
                continue;
            }
            
            if let Some(result) = process_line(
                line,
                line_num,
                file_path,
                query,
                options,
                file_ctime,
                &mut latest_timestamp,
                &mut first_timestamp,
            ) {
                results.push(result);
            }
        }
        
        Ok(results)
    }
}

fn process_line(
    line: &[u8],
    line_num: usize,
    file_path: &Path,
    query: &QueryCondition,
    options: &SearchOptions,
    file_ctime: &str,
    latest_timestamp: &mut Option<String>,
    first_timestamp: &mut Option<String>,
) -> Option<SearchResult> {
    if line.is_empty() {
        return None;
    }
    
    // Parse JSON with SIMD optimization
    #[cfg(feature = "sonic")]
    let message_result = {
        let json_str = std::str::from_utf8(line).ok()?;
        sonic_rs::from_str::<SessionMessage>(json_str)
    };
    
    #[cfg(not(feature = "sonic"))]
    let message_result = {
        let mut json_bytes = line.to_vec();
        simd_json::serde::from_slice::<SessionMessage>(&mut json_bytes)
    };
    
    match message_result {
        Ok(message) => {
            // Update timestamps
            if let Some(ts) = message.get_timestamp() {
                *latest_timestamp = Some(ts.to_string());
                if first_timestamp.is_none() && message.get_type() != "summary" {
                    *first_timestamp = Some(ts.to_string());
                }
            }

            // Extract searchable text
            let text = message.get_searchable_text();

            // Apply query condition
            if let Ok(matches) = query.evaluate(&text) {
                if matches {
                    // Check role filter
                    if let Some(role) = &options.role {
                        if message.get_type() != role {
                            return None;
                        }
                    }

                    // Check session filter
                    if let Some(session_id) = &options.session_id {
                        if message.get_session_id() != Some(session_id) {
                            return None;
                        }
                    }

                    // Check project path filter
                    if let Some(project_filter) = &options.project_path {
                        let project_path = extract_project_path(file_path);
                        if !project_path.starts_with(project_filter) {
                            return None;
                        }
                    }

                    let msg_timestamp = message.get_timestamp();
                    let final_timestamp = msg_timestamp
                        .map(|s| s.to_string())
                        .or_else(|| {
                            if message.get_type() == "summary" {
                                first_timestamp.clone()
                            } else {
                                latest_timestamp.clone()
                            }
                        })
                        .unwrap_or_else(|| file_ctime.to_string());

                    return Some(SearchResult {
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
                        project_path: extract_project_path(file_path),
                        raw_json: Some(String::from_utf8_lossy(line).to_string()),
                    });
                }
            }
        }
        Err(e) => {
            if options.verbose {
                eprintln!(
                    "Failed to parse JSON at line {} in {:?}: {}",
                    line_num + 1,
                    file_path,
                    e
                );
            }
        }
    }
    
    None
}

fn extract_file_ctime(metadata: &Metadata, verbose: bool, file_path: &Path) -> String {
    metadata
        .created()
        .ok()
        .or_else(|| metadata.modified().ok())
        .map(|t| {
            let duration = t.duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
            let ctime = chrono::DateTime::<chrono::Utc>::from_timestamp(duration.as_secs() as i64, 0)
                .unwrap_or_else(chrono::Utc::now)
                .to_rfc3339();
            if verbose {
                eprintln!("DEBUG: file_ctime for {file_path:?} = {ctime}");
            }
            ctime
        })
        .unwrap_or_else(|| {
            let now = chrono::Utc::now().to_rfc3339();
            if verbose {
                eprintln!("DEBUG: Using current time as fallback: {now}");
            }
            now
        })
}

fn extract_project_path(file_path: &Path) -> String {
    if let Some(parent) = file_path.parent() {
        if let Some(project_name) = parent.file_name() {
            if let Some(project_str) = project_name.to_str() {
                return project_str.replace('-', "/");
            }
        }
    }
    String::new()
}