use anyhow::Result;
use chrono::DateTime;
use smol::channel;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::sync::Arc;
use smol::lock::Semaphore;

use super::file_discovery::{discover_claude_files, expand_tilde};
use crate::interactive_ratatui::domain::models::SearchOrder;
use crate::query::{QueryCondition, SearchOptions, SearchResult};
use crate::schemas::SessionMessage;

// Global executor for multi-threaded execution
static EXECUTOR: smol::Executor<'static> = smol::Executor::new();

pub struct OptimizedSmolSearchEngine {
    options: SearchOptions,
}

impl OptimizedSmolSearchEngine {
    pub fn new(options: SearchOptions) -> Self {
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
        let (sender, receiver) = channel::bounded(1024); // Bounded channel for backpressure
        let max_results = self.options.max_results.unwrap_or(50);

        // Create semaphore to limit concurrent file operations
        // Use CPU count for optimal concurrency
        let semaphore = Arc::new(Semaphore::new(num_cpus::get()));

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
            let semaphore = semaphore.clone();
            
            let task = EXECUTOR.spawn(async move {
                // Acquire semaphore permit to limit concurrent file operations
                let _permit = semaphore.acquire().await;
                
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
        // Increase buffer size for better I/O performance
        let reader = BufReader::with_capacity(128 * 1024, file);
        
        let mut results = Vec::with_capacity(32); // Pre-allocate for typical result size
        let mut latest_timestamp: Option<String> = None;
        
        for line in reader.lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            
            // Parse JSON
            #[cfg(feature = "sonic")]
            let message: Result<SessionMessage, _> = sonic_rs::from_str(&line);
            
            #[cfg(not(feature = "sonic"))]
            let message: Result<SessionMessage, _> = {
                let mut line_bytes = line.as_bytes().to_vec();
                simd_json::serde::from_slice(&mut line_bytes)
                    .map_err(|e| anyhow::anyhow!("JSON parse error: {}", e))
            };
            
            if let Ok(message) = message {
                // Update latest timestamp
                if let Some(ts) = message.get_timestamp() {
                    latest_timestamp = Some(ts.to_string());
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
                        
                        let final_timestamp = message
                            .get_timestamp()
                            .map(|ts| ts.to_string())
                            .or_else(|| latest_timestamp.clone())
                            .unwrap_or_else(|| chrono::Utc::now().to_rfc3339());
                        
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

// Initialize the multi-threaded executor
pub fn init_executor() {
    let num_threads = num_cpus::get().max(1);
    for _ in 0..num_threads {
        std::thread::spawn(|| {
            loop {
                smol::block_on(EXECUTOR.run(smol::future::pending::<()>()));
            }
        });
    }
}