use anyhow::Result;
use chrono::DateTime;
use smol::channel;
use std::fs::File;
use std::io::{BufRead, BufReader, Read};
use std::path::Path;
use std::sync::Arc;
// use smol::lock::Semaphore; // Disabled for testing

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
            // Use physical CPU cores instead of logical cores
            // This can reduce cache contention and improve performance
            let physical_cores = num_cpus::get_physical();
            unsafe {
                std::env::set_var("BLOCKING_MAX_THREADS", physical_cores.to_string());
            }
            eprintln!("Optimized BLOCKING_MAX_THREADS to {} (physical CPU cores)", physical_cores);
        }
    });
}

pub struct OptimizedSmolSearchEngine {
    options: SearchOptions,
}

impl OptimizedSmolSearchEngine {
    pub fn new(options: SearchOptions) -> Self {
        // Initialize blocking threads optimization on first use
        initialize_blocking_threads();
        Self { options }
    }

    pub async fn search(
        &self,
        pattern: &str,
        query: QueryCondition,
    ) -> Result<(Vec<SearchResult>, std::time::Duration, usize)> {
        self.search_with_role_filter_and_order(pattern, query, None, SearchOrder::Descending).await
    }

    pub async fn search_with_role_filter_and_order(
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
        let (sender, receiver) = channel::unbounded(); // Changed to unbounded like basic Smol
        let max_results = self.options.max_results.unwrap_or(50);

        // Create semaphore to limit concurrent file operations
        // Use CPU count for optimal concurrency
        // let semaphore = Arc::new(Semaphore::new(num_cpus::get())); // Disabled for testing

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
            // let semaphore = semaphore.clone(); // Disabled for testing
            
            let task = smol::spawn(async move {
                // Acquire semaphore permit to limit concurrent file operations
                // let _permit = semaphore.acquire().await; // Disabled for testing
                
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
            if line_buffer.ends_with(&[b'\n']) {
                line_buffer.pop();
                if line_buffer.ends_with(&[b'\r']) {
                    line_buffer.pop();
                }
            }
            
            // Parse JSON - Always use sonic-rs for optimized engine
            // Use from_slice to avoid UTF-8 string conversion
            let message: Result<SessionMessage, _> = sonic_rs::from_slice(&line_buffer);
            
            if let Ok(message) = message {
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
                            project_path: extract_project_path(&file_path_owned),
                            raw_json: None,
                        };
                        results.push(result);
                    }
                }
            }
        }
        
        Ok(results)
    }).await
}

fn extract_project_path(file_path: &Path) -> String {
    file_path
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_string()
}

