use anyhow::Result;
use rayon::prelude::*;
use std::path::Path;

use super::fast_json_scanner::FastJsonScanner;
use crate::query::{QueryCondition, SearchOptions, SearchResult};
use crate::schemas::SessionMessage;

/// Optimized search with two-phase filtering
pub struct OptimizedSearchEngine {
    options: SearchOptions,
}

impl OptimizedSearchEngine {
    pub fn new(options: SearchOptions) -> Self {
        Self { options }
    }

    pub fn search_file_optimized(
        &self, 
        file_path: &Path, 
        query: &QueryCondition,
        query_hint: Option<&str>
    ) -> Result<Vec<SearchResult>> {
        let file_name = file_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        // Use memory-mapped I/O
        let mmap_reader = match super::mmap_reader::MmapReader::new(file_path) {
            Ok(reader) => reader,
            Err(_) => return Ok(Vec::new()),
        };
        
        let lines: Vec<&str> = mmap_reader.lines().collect();

        // Phase 1: Quick filtering with fast scanner
        let candidates: Vec<(usize, &str)> = if let Some(hint) = query_hint {
            lines
                .par_iter()
                .enumerate()
                .filter(|(_, line)| FastJsonScanner::might_contain(line, hint))
                .map(|(i, line)| (i, *line))
                .collect()
        } else {
            // No hint, must check all lines
            lines
                .iter()
                .enumerate()
                .map(|(i, line)| (i, *line))
                .collect()
        };

        // Phase 2: Full parsing and evaluation for candidates only
        let mut results = Vec::new();
        for (_line_num, line) in candidates {
            if line.trim().is_empty() {
                continue;
            }

            // Try fast extraction first for simple queries
            if let Some(text) = FastJsonScanner::extract_text_content(line) {
                if let Ok(matches) = query.evaluate(&text) {
                    if matches {
                        // Now do full parsing to get complete result
                        let mut json_bytes = line.as_bytes().to_vec();
                        if let Ok(message) = simd_json::serde::from_slice::<SessionMessage>(&mut json_bytes) {
                            // Check filters
                            if let Some(role) = &self.options.role {
                                if message.get_type() != role {
                                    continue;
                                }
                            }

                            results.push(SearchResult {
                                file: file_name.clone(),
                                uuid: message.get_uuid().unwrap_or("").to_string(),
                                timestamp: message.get_timestamp().unwrap_or("").to_string(),
                                session_id: message.get_session_id().unwrap_or("").to_string(),
                                role: message.get_type().to_string(),
                                text: message.get_content_text(),
                                has_tools: message.has_tool_use(),
                                has_thinking: message.has_thinking(),
                                message_type: message.get_type().to_string(),
                                query: query.clone(),
                                project_path: String::new(),
                                raw_json: Some(line.to_string()),
                            });
                        }
                    }
                }
            }
        }

        Ok(results)
    }
}

/// Extract query hint for optimization
pub fn extract_query_hint(query: &QueryCondition) -> Option<String> {
    match query {
        QueryCondition::Literal { pattern, .. } => Some(pattern.clone()),
        QueryCondition::And { conditions } => {
            // Use the first literal as hint
            conditions.iter()
                .find_map(|c| extract_query_hint(c))
        }
        QueryCondition::Or { conditions } => {
            // For OR, we can't use a single hint effectively
            if conditions.len() == 1 {
                extract_query_hint(&conditions[0])
            } else {
                None
            }
        }
        _ => None,
    }
}