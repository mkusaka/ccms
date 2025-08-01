use anyhow::Result;
use chrono::DateTime;
use crossbeam::channel;
use indicatif::{ProgressBar, ProgressStyle};
use rayon::prelude::*;
use std::fs::File;
use std::path::Path;

use super::file_discovery::{discover_claude_files, expand_tilde};
use crate::interactive_ratatui::domain::models::SearchOrder;
use crate::query::{QueryCondition, SearchOptions, SearchResult};
use crate::schemas::SessionMessage;

#[cfg(feature = "mmap")]
use memmap2::Mmap;

pub struct OptimizedRayonEngine {
    options: SearchOptions,
}

impl OptimizedRayonEngine {
    pub fn new(options: SearchOptions) -> Self {
        // Configure Rayon to use physical CPU cores
        // This can reduce cache contention and improve performance
        let physical_cores = num_cpus::get_physical();
        if let Err(e) = rayon::ThreadPoolBuilder::new()
            .num_threads(physical_cores)
            .build_global() 
        {
            if options.verbose {
                eprintln!("Warning: Could not set Rayon thread pool size: {}", e);
            }
        } else if options.verbose {
            eprintln!("Configured Rayon to use {} physical CPU cores", physical_cores);
        }
        
        Self { options }
    }

    pub fn search(
        &self,
        pattern: &str,
        query: QueryCondition,
    ) -> Result<(Vec<SearchResult>, std::time::Duration, usize)> {
        self.search_with_role_filter_and_order(pattern, query, None, SearchOrder::Descending)
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

        // Progress bar
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

        // Process files in parallel with chunk-based batching to reduce Rayon overhead
        let search_start = std::time::Instant::now();
        
        // Use standard per-file parallelization without chunking
        // The chunking approach reduced parallelism too much
        files.par_iter()
            .for_each_with(sender, |s, file_path| {
                if let Some(pb) = &progress {
                    pb.inc(1);
                }

                if let Ok(results) = self.search_file(file_path, &query) {
                    for result in results {
                        let _ = s.send(result);
                    }
                }
            });
            
        let search_time = search_start.elapsed();

        // Collect results
        drop(progress);
        let mut all_results: Vec<SearchResult> = receiver.try_iter().collect();

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

    #[cfg(feature = "mmap")]
    fn search_file(&self, file_path: &Path, query: &QueryCondition) -> Result<Vec<SearchResult>> {
        let file = File::open(file_path)?;
        let _metadata = file.metadata()?;
        
        // Memory-map the file
        let mmap = unsafe { Mmap::map(&file)? };
        let content = &mmap[..];
        
        let mut results = Vec::with_capacity(256); // 4x larger initial capacity to reduce reallocations
        let mut start = 0;
        
        // Use memchr for fast newline scanning
        for line_end in memchr::memchr_iter(b'\n', content) {
            let line = &content[start..line_end];
            
            // For mmap version, we don't have the summary processing
            // This keeps the simple high-performance path for mmap
            if let Ok(result) = self.process_line(line, file_path, query) {
                if let Some(res) = result {
                    results.push(res);
                }
            }
            
            start = line_end + 1;
        }
        
        // Handle last line if no trailing newline
        if start < content.len() {
            let line = &content[start..];
            if let Ok(result) = self.process_line(line, file_path, query) {
                if let Some(res) = result {
                    results.push(res);
                }
            }
        }
        
        Ok(results)
    }
    
    #[cfg(not(feature = "mmap"))]
    fn search_file(&self, file_path: &Path, query: &QueryCondition) -> Result<Vec<SearchResult>> {
        use std::io::{BufRead, BufReader, Read};
        
        let file = File::open(file_path)?;
        
        // Get file metadata for fallback timestamp
        let metadata = file.metadata()?;
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
        
        let mut reader = BufReader::with_capacity(64 * 1024, file);
        
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
                if let Ok(matches) = query.evaluate(&text) {
                    if matches {
                        // Apply inline filters
                        if let Some(role) = &self.options.role {
                            if message.get_type() != role {
                                continue;
                            }
                        }
                        
                        if let Some(session_id) = &self.options.session_id {
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
                            raw_json: None,
                        };
                        results.push(result);
                    }
                }
            }
        }
        
        Ok(results)
    }
    
    fn process_line(
        &self,
        line: &[u8],
        file_path: &Path,
        query: &QueryCondition,
    ) -> Result<Option<SearchResult>> {
        // Skip empty lines
        if line.is_empty() {
            return Ok(None);
        }
        
        // Always use sonic-rs for better performance
        let message: SessionMessage = sonic_rs::from_slice(line)?;
        
        // Simple search without ASCII optimization
        let content_text = message.get_content_text();
        let matches = query.evaluate(&content_text)?;
        
        if matches {
            let timestamp = message
                .get_timestamp()
                .map(|ts| ts.to_string())
                .unwrap_or_else(|| chrono::Utc::now().to_rfc3339());
                
            let result = SearchResult {
                file: file_path.to_string_lossy().to_string(),
                uuid: message.get_uuid().unwrap_or("").to_string(),
                timestamp,
                session_id: message.get_session_id().unwrap_or("").to_string(),
                role: message.get_type().to_string(),
                text: content_text,
                has_tools: message.has_tool_use(),
                has_thinking: message.has_thinking(),
                message_type: message.get_type().to_string(),
                query: query.clone(),
                project_path: file_path.parent()
                    .and_then(|p| p.file_name())
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default(),
                raw_json: None,
            };
            Ok(Some(result))
        } else {
            Ok(None)
        }
    }
    
    #[allow(dead_code)]
    fn process_line_with_timestamps(
        &self,
        line: &[u8],
        file_path: &Path,
        query: &QueryCondition,
        file_ctime: &str,
        first_timestamp: &Option<String>,
        latest_timestamp: &mut Option<String>,
    ) -> Result<Option<SearchResult>> {
        // Skip empty lines
        if line.is_empty() {
            return Ok(None);
        }
        
        // Always use sonic-rs for better performance
        let message: SessionMessage = sonic_rs::from_slice(line)?;
        
        // Update latest timestamp if this message has one
        if let Some(ts) = message.get_timestamp() {
            *latest_timestamp = Some(ts.to_string());
        }
        
        // Get searchable text
        let text = message.get_searchable_text();
        
        // Apply query condition
        if query.evaluate(&text)? {
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
                .unwrap_or_else(|| file_ctime.to_string());
                
            let result = SearchResult {
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
                project_path: file_path.parent()
                    .and_then(|p| p.file_name())
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default(),
                raw_json: None,
            };
            Ok(Some(result))
        } else {
            Ok(None)
        }
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

fn extract_project_path(file_path: &Path) -> String {
    file_path
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_string()
}