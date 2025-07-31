use anyhow::Result;
use chrono::DateTime;
use crossbeam::channel;
use indicatif::{ProgressBar, ProgressStyle};
use memchr::memchr_iter;
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

        // Process files in parallel with batch size optimization
        let search_start = std::time::Instant::now();
        
        // Batch files to reduce Rayon overhead
        files
            .par_chunks(std::cmp::max(1, files.len() / (rayon::current_num_threads() * 4)))
            .for_each_with(sender, |s, chunk| {
                for file_path in chunk {
                    if let Some(pb) = &progress {
                        pb.inc(1);
                    }

                    if let Ok(results) = self.search_file(file_path, &query) {
                        for result in results {
                            let _ = s.send(result);
                        }
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
        
        let mut results = Vec::new();
        let mut start = 0;
        
        // Use memchr for fast newline scanning
        for line_end in memchr_iter(b'\n', content) {
            let line = &content[start..line_end];
            
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
        use std::io::{BufRead, BufReader};
        
        let file = File::open(file_path)?;
        let reader = BufReader::with_capacity(64 * 1024, file);
        
        let mut results = Vec::new();
        // Pre-allocate buffer to reuse
        let mut line_buffer = Vec::with_capacity(8 * 1024);
        
        let mut reader = reader;
        while reader.read_until(b'\n', &mut line_buffer)? > 0 {
            // Strip newline
            if line_buffer.last() == Some(&b'\n') {
                line_buffer.pop();
                if line_buffer.last() == Some(&b'\r') {
                    line_buffer.pop();
                }
            }
            
            if let Ok(result) = self.process_line(&line_buffer, file_path, query) {
                if let Some(res) = result {
                    results.push(res);
                }
            }
            
            line_buffer.clear();
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
        
        // Use sonic-rs if available, otherwise fall back to simd_json
        #[cfg(feature = "sonic")]
        let message: SessionMessage = sonic_rs::from_slice(line)?;
        
        #[cfg(not(feature = "sonic"))]
        let message: SessionMessage = {
            let mut line_copy = line.to_vec();
            simd_json::serde::from_slice(&mut line_copy)?
        };
        
        // Apply ASCII lowercasing optimization for search
        let content_text = message.get_content_text();
        let matches = if self.is_likely_ascii(&content_text) {
            let mut content_lower = content_text.as_bytes().to_vec();
            self.ascii_in_place_lowercase(&mut content_lower);
            let content_str = unsafe { std::str::from_utf8_unchecked(&content_lower) };
            query.evaluate(content_str)?
        } else {
            query.evaluate(&content_text.to_lowercase())?
        };
        
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
    
    fn is_likely_ascii(&self, s: &str) -> bool {
        s.chars().take(100).all(|c| c.is_ascii())
    }
    
    fn ascii_in_place_lowercase(&self, buf: &mut [u8]) {
        for b in buf.iter_mut() {
            if (b'A'..=b'Z').contains(b) {
                *b |= 0x20;
            }
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