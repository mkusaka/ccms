use crate::query::condition::QueryCondition;
use crate::schemas::session_message::SessionMessage;
use crate::search::file_discovery::discover_claude_files;
use anyhow::{Context, Result};
use futures::stream::{self, StreamExt};
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::fs::File;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::sync::Semaphore;
use tracing::{debug, info};

#[derive(Debug, Clone)]
pub struct AsyncSearchOptions {
    pub max_results: Option<usize>,
    pub role: Option<String>,
    pub session_id: Option<String>,
    pub before: Option<String>,
    pub after: Option<String>,
    pub verbose: bool,
    pub max_concurrent_files: usize,
}

impl Default for AsyncSearchOptions {
    fn default() -> Self {
        Self {
            max_results: None,
            role: None,
            session_id: None,
            before: None,
            after: None,
            verbose: false,
            max_concurrent_files: 50, // Limit concurrent file operations
        }
    }
}

#[derive(Debug, Clone)]
pub struct AsyncSearchResult {
    pub file: String,
    pub uuid: String,
    pub timestamp: String,
    pub session_id: String,
    pub role: String,
    pub text: String,
    pub has_tools: bool,
    pub has_thinking: bool,
    pub message_type: String,
    pub query: QueryCondition,
}

pub struct AsyncSearchEngine {
    options: AsyncSearchOptions,
}

impl AsyncSearchEngine {
    pub fn new(options: AsyncSearchOptions) -> Self {
        Self { options }
    }

    pub async fn search(
        &self,
        pattern: &str,
        query: QueryCondition,
    ) -> Result<(Vec<AsyncSearchResult>, Duration, usize)> {
        let start = Instant::now();
        let files = discover_claude_files(Some(pattern))?;

        if self.options.verbose {
            info!("Found {} files to search", files.len());
        }

        let semaphore = Arc::new(Semaphore::new(self.options.max_concurrent_files));
        let max_results = self.options.max_results.unwrap_or(usize::MAX);
        let results = Arc::new(tokio::sync::Mutex::new(Vec::with_capacity(max_results)));
        let total_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));

        // Process files concurrently with controlled parallelism
        let tasks = stream::iter(files.into_iter())
            .map(|file_path: std::path::PathBuf| {
                let semaphore = Arc::clone(&semaphore);
                let results = Arc::clone(&results);
                let total_count = Arc::clone(&total_count);
                let query = query.clone();
                let options = self.options.clone();

                async move {
                    let _permit = semaphore.acquire().await.unwrap();

                    if let Ok(file_results) = search_file(&file_path, &query, &options).await {
                        let mut results_guard = results.lock().await;

                        for result in file_results {
                            total_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

                            if results_guard.len() < max_results {
                                results_guard.push(result);
                            }
                        }
                    }
                }
            })
            .buffer_unordered(self.options.max_concurrent_files);

        // Consume all tasks
        tasks.collect::<Vec<_>>().await;

        let duration = start.elapsed();
        let results = Arc::try_unwrap(results).unwrap().into_inner();
        let total = total_count.load(std::sync::atomic::Ordering::Relaxed);

        Ok((results, duration, total))
    }
}

async fn search_file(
    file_path: &Path,
    query: &QueryCondition,
    options: &AsyncSearchOptions,
) -> Result<Vec<AsyncSearchResult>> {
    let file = File::open(file_path)
        .await
        .with_context(|| format!("Failed to open file: {file_path:?}"))?;

    let reader = BufReader::new(file);
    let mut lines = reader.lines();
    let mut results = Vec::new();

    let file_name = file_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();

    while let Some(line) = lines.next_line().await? {
        if line.trim().is_empty() {
            continue;
        }

        // Parse JSON line
        let mut bytes = line.as_bytes().to_vec();
        match simd_json::serde::from_slice::<SessionMessage>(&mut bytes) {
            Ok(message) => {
                // Apply filters
                if let Some(role) = &options.role {
                    if message.get_type() != role {
                        continue;
                    }
                }

                if let Some(session_id) = &options.session_id {
                    if message.get_session_id() != Some(session_id.as_str()) {
                        continue;
                    }
                }

                // Apply timestamp filters if needed
                if let Some(timestamp) = message.get_timestamp() {
                    if let Some(before) = &options.before {
                        if timestamp >= before.as_str() {
                            continue;
                        }
                    }
                    if let Some(after) = &options.after {
                        if timestamp <= after.as_str() {
                            continue;
                        }
                    }
                }

                // Check if content matches query
                let content = message.get_content_text();
                if matches_query(&content, query) {
                    results.push(AsyncSearchResult {
                        file: file_name.clone(),
                        uuid: message.get_uuid().unwrap_or("").to_string(),
                        timestamp: message.get_timestamp().unwrap_or("").to_string(),
                        session_id: message.get_session_id().unwrap_or("").to_string(),
                        role: message.get_type().to_string(),
                        text: truncate_text(&content, 200),
                        has_tools: message.has_tool_use(),
                        has_thinking: message.has_thinking(),
                        message_type: message.get_type().to_string(),
                        query: query.clone(),
                    });
                }
            }
            Err(e) => {
                debug!("Failed to parse line in {}: {}", file_name, e);
            }
        }
    }

    Ok(results)
}

fn matches_query(text: &str, condition: &QueryCondition) -> bool {
    match condition {
        QueryCondition::Literal {
            pattern,
            case_sensitive,
        } => {
            if *case_sensitive {
                text.contains(pattern)
            } else {
                text.to_lowercase().contains(&pattern.to_lowercase())
            }
        }
        QueryCondition::Regex { pattern, flags } => {
            let mut builder = regex::RegexBuilder::new(pattern);
            if flags.contains('i') {
                builder.case_insensitive(true);
            }
            if flags.contains('m') {
                builder.multi_line(true);
            }
            if flags.contains('s') {
                builder.dot_matches_new_line(true);
            }

            match builder.build() {
                Ok(re) => re.is_match(text),
                Err(_) => false,
            }
        }
        QueryCondition::Not { condition } => !matches_query(text, condition),
        QueryCondition::And { conditions } => conditions.iter().all(|c| matches_query(text, c)),
        QueryCondition::Or { conditions } => conditions.iter().any(|c| matches_query(text, c)),
    }
}

fn truncate_text(text: &str, max_len: usize) -> String {
    if text.len() <= max_len {
        text.to_string()
    } else {
        let mut end = max_len;
        while !text.is_char_boundary(end) && end > 0 {
            end -= 1;
        }
        format!("{}...", &text[..end])
    }
}
