use anyhow::{Context, Result};
use bytes::BytesMut;
use chrono::DateTime;
use std::fs::Metadata;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Mutex;
use std::time::Instant;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, BufReader};
use tokio::sync::mpsc;

use super::file_discovery::{discover_claude_files, expand_tilde};
use crate::interactive_ratatui::domain::models::SearchOrder;
use crate::query::{QueryCondition, SearchOptions, SearchResult};
use crate::schemas::SessionMessage;

/// Optimized async search engine with worker pool pattern
pub struct OptimizedAsyncSearchEngineV4 {
    options: SearchOptions,
    /// Number of worker threads for file processing
    num_workers: usize,
    /// Buffer size for file reading
    buffer_size: usize,
    /// Whether to use hybrid rayon parsing
    use_hybrid_parsing: bool,
    /// Channel buffer size for job queue
    job_queue_size: usize,
    /// Channel buffer size for results
    result_channel_size: usize,
}

/// Job type for worker pool
#[derive(Clone)]
struct FileJob {
    path: PathBuf,
    query: Arc<QueryCondition>,
    options: Arc<SearchOptions>,
}

impl OptimizedAsyncSearchEngineV4 {
    pub fn new(options: SearchOptions) -> Self {
        let num_cpus = num_cpus::get();
        Self {
            options,
            // Fixed pool of workers
            num_workers: num_cpus,
            // Large buffer for efficient I/O
            buffer_size: 64 * 1024,
            // Enable hybrid parsing by default
            use_hybrid_parsing: true,
            // Job queue size
            job_queue_size: 256,
            // Result channel size (optimized for Tokio's 32-slot blocks)
            result_channel_size: 128,
        }
    }
    
    pub fn with_num_workers(mut self, num_workers: usize) -> Self {
        self.num_workers = num_workers;
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

        // Create channels
        let (job_tx, job_rx) = mpsc::channel::<FileJob>(self.job_queue_size);
        let (result_tx, mut result_rx) = mpsc::channel::<Vec<SearchResult>>(self.result_channel_size);
        let max_results = self.options.max_results.unwrap_or(50);
        
        let search_start = Instant::now();
        
        // Prepare shared data
        let query = Arc::new(query);
        let options = Arc::new(self.options.clone());
        let buffer_size = self.buffer_size;
        let use_hybrid_parsing = self.use_hybrid_parsing;
        
        // Wrap receiver in Arc<Mutex> for sharing
        let job_rx = Arc::new(Mutex::new(job_rx));
        
        // Spawn worker pool
        let mut workers = Vec::new();
        for _ in 0..self.num_workers {
            let job_rx = Arc::clone(&job_rx);
            let result_tx = result_tx.clone();
            
            let worker = tokio::spawn(async move {
                // Reusable buffer for this worker
                let mut file_buffer = BytesMut::with_capacity(buffer_size * 2);
                
                loop {
                    // Lock and receive job
                    let job = {
                        let mut rx = job_rx.lock().await;
                        rx.recv().await
                    };
                    
                    match job {
                        Some(job) => {
                            if let Ok(results) = Self::search_file_with_buffer(
                                &job.path,
                                &job.query,
                                &job.options,
                                buffer_size,
                                use_hybrid_parsing,
                                &mut file_buffer,
                            ).await {
                                if !results.is_empty() {
                                    let _ = result_tx.send(results).await;
                                }
                            }
                        }
                        None => break, // Channel closed
                    }
                }
            });
            
            workers.push(worker);
        }
        
        // Send jobs to workers
        let job_sender = tokio::spawn(async move {
            for file in files {
                let job = FileJob {
                    path: file,
                    query: Arc::clone(&query),
                    options: Arc::clone(&options),
                };
                if job_tx.send(job).await.is_err() {
                    break; // Workers have shut down
                }
            }
        });
        
        // Drop the original result sender
        drop(result_tx);
        
        // Collect results
        let mut all_results = Vec::new();
        let mut total_count = 0;
        
        while let Some(batch_results) = result_rx.recv().await {
            for result in batch_results {
                total_count += 1;
                if all_results.len() < max_results * 2 {
                    // Collect more than needed for sorting
                    all_results.push(result);
                }
            }
        }
        
        // Wait for job sender to complete
        job_sender.await?;
        
        // Shutdown workers by dropping the job sender
        // Workers will exit when their job_rx.recv() returns None
        
        // Wait for all workers to complete
        for worker in workers {
            worker.await?;
        }
        
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
            eprintln!("  Worker pool size: {}", self.num_workers);
        }

        Ok((all_results, elapsed, total_count))
    }

    async fn search_file_with_buffer(
        file_path: &Path,
        query: &QueryCondition,
        options: &SearchOptions,
        buffer_size: usize,
        use_hybrid_parsing: bool,
        reusable_buffer: &mut BytesMut,
    ) -> Result<Vec<SearchResult>> {
        // Get file metadata
        let metadata = tokio::fs::metadata(file_path).await?;
        let file_size = metadata.len();
        
        // Get file creation time
        let file_ctime = extract_file_ctime(&metadata, options.verbose, file_path);
        
        // Clear and reuse buffer
        reusable_buffer.clear();
        
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
        let mut results = Vec::new();
        let mut latest_timestamp: Option<String> = None;
        let mut first_timestamp: Option<String> = None;
        let mut line_num = 0;
        
        // Read file in chunks using the reusable buffer
        loop {
            let bytes_read = reader.read_buf(reusable_buffer).await?;
            if bytes_read == 0 {
                // Process any remaining data
                if !reusable_buffer.is_empty() {
                    let line = reusable_buffer.split();
                    if let Some(result) = process_line(
                        &line,
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
            while let Some(newline_pos) = reusable_buffer.iter().position(|&b| b == b'\n') {
                let line = reusable_buffer.split_to(newline_pos + 1);
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