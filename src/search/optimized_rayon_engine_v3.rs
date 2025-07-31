use anyhow::Result;
use chrono::DateTime;
use crossbeam::channel;
use indicatif::{ProgressBar, ProgressStyle};
use rayon::prelude::*;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

use super::file_discovery::{discover_claude_files, expand_tilde};
use crate::interactive_ratatui::domain::models::SearchOrder;
use crate::query::{QueryCondition, SearchOptions, SearchResult};
use crate::schemas::SessionMessage;

pub struct OptimizedRayonEngineV3 {
    options: SearchOptions,
}

impl OptimizedRayonEngineV3 {
    pub fn new(options: SearchOptions) -> Self {
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

        // Process files in parallel
        let search_start = std::time::Instant::now();
        
        files
            .par_iter()
            .for_each_with(sender, |s, file_path| {
                if let Some(pb) = &progress {
                    pb.inc(1);
                }

                if let Ok(results) = self.search_file_single_pass(file_path, &query) {
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

    fn search_file_single_pass(&self, file_path: &Path, query: &QueryCondition) -> Result<Vec<SearchResult>> {
        let file = File::open(file_path)?;
        
        // Get file metadata for fallback timestamp
        let metadata = file.metadata()?;
        let file_ctime = metadata
            .created()
            .or_else(|_| metadata.modified())
            .ok()
            .map(|t| {
                let duration = t.duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
                chrono::DateTime::<chrono::Utc>::from_timestamp(duration.as_secs() as i64, 0)
                    .unwrap_or_else(chrono::Utc::now)
                    .to_rfc3339()
            })
            .unwrap_or_else(|| chrono::Utc::now().to_rfc3339());
        
        // Use larger buffer for better I/O performance
        let reader = BufReader::with_capacity(64 * 1024, file);
        
        let mut results = Vec::new();
        let mut latest_timestamp: Option<String> = None;
        let mut first_timestamp: Option<String> = None;
        let mut is_first_line = true;
        let mut first_is_summary = false;
        
        // Single pass processing
        for line in reader.lines() {
            let line = line?;
            
            // Skip empty lines
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
                // Track timestamps
                if let Some(ts) = message.get_timestamp() {
                    latest_timestamp = Some(ts.to_string());
                    if first_timestamp.is_none() && !first_is_summary {
                        first_timestamp = Some(ts.to_string());
                    }
                }
                
                // Check if first message is summary
                if is_first_line {
                    is_first_line = false;
                    if message.get_type() == "summary" {
                        first_is_summary = true;
                    }
                }
                
                // Get searchable text
                let text = message.get_searchable_text();
                
                // Apply query condition
                if let Ok(matches) = query.evaluate(&text) {
                    if matches {
                        // Apply inline filters to reduce memory usage
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
                        
                        // Determine final timestamp
                        let final_timestamp = if message.get_type() == "summary" {
                            if let Some(ref ts) = first_timestamp {
                                ts.clone()
                            } else if let Some(ref ts) = latest_timestamp {
                                ts.clone()
                            } else {
                                file_ctime.clone()
                            }
                        } else {
                            message.get_timestamp()
                                .map(|ts| ts.to_string())
                                .or_else(|| latest_timestamp.clone())
                                .unwrap_or_else(|| file_ctime.clone())
                        };
                        
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
                            project_path: Self::extract_project_path(file_path),
                            raw_json: None,
                        };
                        results.push(result);
                    }
                }
            }
        }
        
        Ok(results)
    }
    
    fn extract_project_path(file_path: &Path) -> String {
        // Extract project path from file path pattern
        // ~/.claude/projects/{project-path}/{session-id}.jsonl
        file_path
            .parent()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string()
    }
    
    fn apply_filters(
        &self,
        results: &mut Vec<SearchResult>,
        role_filter: Option<String>,
    ) -> Result<()> {
        // Apply role filter (if not already applied inline)
        if let Some(role) = role_filter {
            results.retain(|r| r.role == role);
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
        
        // Apply project path filter
        if let Some(ref project_filter) = self.options.project_path {
            results.retain(|r| r.project_path.starts_with(project_filter));
        }

        Ok(())
    }
}