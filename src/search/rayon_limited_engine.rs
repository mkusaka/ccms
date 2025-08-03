use anyhow::Result;
use chrono::DateTime;
use crossbeam::channel;
use std::sync::Arc;

use super::engine::SearchEngineTrait;
use super::file_discovery::{discover_claude_files, expand_tilde};
use crate::interactive_ratatui::domain::models::SearchOrder;
use crate::query::{QueryCondition, SearchOptions, SearchResult};

pub struct RayonLimitedEngine {
    options: SearchOptions,
    thread_pool: rayon::ThreadPool,
}

impl RayonLimitedEngine {
    pub fn new(options: SearchOptions) -> Self {
        // Use physical CPU count for thread pool
        let num_threads = num_cpus::get_physical();
        let thread_pool = rayon::ThreadPoolBuilder::new()
            .num_threads(num_threads)
            .build()
            .expect("Failed to create thread pool");

        Self {
            options,
            thread_pool,
        }
    }
}

impl SearchEngineTrait for RayonLimitedEngine {
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

        // Process files in parallel using limited thread pool
        let search_start = std::time::Instant::now();

        let query = Arc::new(query);
        let options = Arc::new(self.options.clone());

        // Process files in parallel using the limited thread pool
        self.thread_pool.scope(|s| {
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

impl RayonLimitedEngine {
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

// Use the same search_file function from rayon_engine.rs
use super::rayon_engine::search_file;
