use anyhow::Result;
use duckdb::{params, Connection};
use rayon::prelude::*;
use std::path::Path;
use std::sync::{Arc, Mutex};

use super::file_discovery::{discover_claude_files, expand_tilde};
use crate::query::{QueryCondition, SearchOptions, SearchResult};

pub struct DuckDBSearchEngine {
    options: SearchOptions,
    conn: Arc<Mutex<Connection>>,
}

pub struct DuckDBPersistentEngine {
    options: SearchOptions,
    conn: Arc<Mutex<Connection>>,
}

impl DuckDBSearchEngine {
    pub fn new(options: SearchOptions) -> Result<Self> {
        // Create in-memory DuckDB connection
        let conn = Connection::open_in_memory()?;
        
        // Create table schema
        conn.execute(
            "CREATE TABLE messages (
                file_path VARCHAR,
                line_number INTEGER,
                uuid VARCHAR,
                timestamp VARCHAR,
                session_id VARCHAR,
                role VARCHAR,
                content TEXT,
                has_tools BOOLEAN,
                has_thinking BOOLEAN,
                message_type VARCHAR,
                project_path VARCHAR,
                raw_json TEXT
            )",
            [],
        )?;

        // Install and load FTS extension
        conn.execute_batch(
            "INSTALL fts;
             LOAD fts;",
        )?;

        Ok(Self {
            options,
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    pub fn search(
        &self,
        pattern: &str,
        query: QueryCondition,
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

        // Load data into DuckDB
        let load_start = std::time::Instant::now();
        self.load_files(&files)?;
        let load_time = load_start.elapsed();

        if self.options.verbose {
            eprintln!("Data loading took: {}ms", load_time.as_millis());
        }

        // Create FTS index
        let index_start = std::time::Instant::now();
        {
            let conn = self.conn.lock().unwrap();
            conn.execute_batch(
                "PRAGMA create_fts_index(
                    'messages',
                    'line_number',
                    'content',
                    stemmer='none',
                    stopwords='none',
                    ignore='\\\\W+',
                    strip_accents=0,
                    lower=1,
                    overwrite=1
                );",
            )?;
        }
        let index_time = index_start.elapsed();

        if self.options.verbose {
            eprintln!("FTS index creation took: {}ms", index_time.as_millis());
        }

        // Execute search query
        let search_start = std::time::Instant::now();
        let results = self.execute_search(&query)?;
        let search_time = search_start.elapsed();

        let elapsed = start_time.elapsed();

        if self.options.verbose {
            eprintln!("\nPerformance breakdown:");
            eprintln!("  File discovery: {}ms", file_discovery_time.as_millis());
            eprintln!("  Data loading: {}ms", load_time.as_millis());
            eprintln!("  Index creation: {}ms", index_time.as_millis());
            eprintln!("  Search execution: {}ms", search_time.as_millis());
            eprintln!("  Total: {}ms", elapsed.as_millis());
        }

        let total_count = results.len();
        let max_results = self.options.max_results.unwrap_or(50);
        let limited_results = results.into_iter().take(max_results).collect();

        Ok((limited_results, elapsed, total_count))
    }

    fn load_files(&self, files: &[std::path::PathBuf]) -> Result<()> {
        use std::fs::File;
        use std::io::{BufRead, BufReader};

        let conn = self.conn.clone();
        
        // Process files in parallel and collect results
        let all_rows: Vec<_> = files
            .par_iter()
            .flat_map(|file_path| {
                let mut rows = Vec::new();
                
                if let Ok(file) = File::open(file_path) {
                    let reader = BufReader::with_capacity(32 * 1024, file);
                    
                    for (line_num, line) in reader.lines().enumerate() {
                        if let Ok(line) = line {
                            if line.trim().is_empty() {
                                continue;
                            }
                            
                            // Parse JSON using simd_json
                            let mut json_bytes = line.as_bytes().to_vec();
                            if let Ok(message) = simd_json::serde::from_slice::<crate::schemas::SessionMessage>(&mut json_bytes) {
                                let text = message.get_content_text();
                                
                                // Skip empty content
                                if text.is_empty() {
                                    continue;
                                }
                                
                                rows.push((
                                    file_path.to_string_lossy().to_string(),
                                    line_num as i32 + 1,
                                    message.get_uuid().unwrap_or("").to_string(),
                                    message.get_timestamp().unwrap_or("").to_string(),
                                    message.get_session_id().unwrap_or("").to_string(),
                                    message.get_type().to_string(),
                                    text,
                                    message.has_tool_use(),
                                    message.has_thinking(),
                                    message.get_type().to_string(),
                                    Self::extract_project_path(file_path),
                                    line.clone(),
                                ));
                            }
                        }
                    }
                }
                
                rows
            })
            .collect();

        // Insert all rows in a single transaction
        let conn_guard = conn.lock().unwrap();
        let tx = conn_guard.unchecked_transaction()?;
        
        {
            let mut stmt = tx.prepare(
                "INSERT INTO messages VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
            )?;
            
            for row in all_rows {
                stmt.execute(params![
                    row.0, row.1, row.2, row.3, row.4, row.5,
                    row.6, row.7, row.8, row.9, row.10, row.11
                ])?;
            }
        }
        
        tx.commit()?;
        Ok(())
    }

    fn execute_search(&self, query: &QueryCondition) -> Result<Vec<SearchResult>> {
        let conn = self.conn.lock().unwrap();
        
        // Build SQL query based on QueryCondition
        let where_clause = self.build_where_clause(query)?;
        
        let mut sql = format!(
            "SELECT DISTINCT 
                file_path, uuid, timestamp, session_id, role, 
                content, has_tools, has_thinking, message_type, 
                project_path, raw_json
             FROM messages
             WHERE {where_clause}"
        );

        // Apply filters
        if let Some(role) = &self.options.role {
            sql.push_str(&format!(" AND role = '{}'", role.replace("'", "''")));
        }

        if let Some(session_id) = &self.options.session_id {
            sql.push_str(&format!(" AND session_id = '{}'", session_id.replace("'", "''")));
        }

        if let Some(project_path) = &self.options.project_path {
            sql.push_str(&format!(" AND project_path LIKE '{}%'", project_path.replace("'", "''")));
        }

        if let Some(before) = &self.options.before {
            sql.push_str(&format!(" AND timestamp < '{}'", before.replace("'", "''")));
        }

        if let Some(after) = &self.options.after {
            sql.push_str(&format!(" AND timestamp > '{}'", after.replace("'", "''")));
        }

        sql.push_str(" ORDER BY timestamp DESC");

        // Execute query
        let mut stmt = conn.prepare(&sql)?;
        let result_iter = stmt.query_map([], |row| {
            Ok(SearchResult {
                file: row.get(0)?,
                uuid: row.get(1)?,
                timestamp: row.get(2)?,
                session_id: row.get(3)?,
                role: row.get(4)?,
                text: row.get(5)?,
                has_tools: row.get(6)?,
                has_thinking: row.get(7)?,
                message_type: row.get(8)?,
                query: query.clone(),
                project_path: row.get(9)?,
                raw_json: Some(row.get(10)?),
            })
        })?;

        let results: Result<Vec<_>, _> = result_iter.collect();
        Ok(results?)
    }

    fn build_where_clause(&self, query: &QueryCondition) -> Result<String> {
        match query {
            QueryCondition::Literal { pattern, case_sensitive } => {
                if *case_sensitive {
                    Ok(format!("content LIKE '%{}%'", pattern.replace("'", "''")))
                } else {
                    Ok(format!("LOWER(content) LIKE LOWER('%{}%')", pattern.replace("'", "''")))
                }
            }
            QueryCondition::Regex { pattern, flags } => {
                // DuckDB doesn't have native regex support in the same way
                // For now, we'll use LIKE for simple patterns
                if flags.contains('i') {
                    Ok(format!("LOWER(content) LIKE LOWER('%{}%')", pattern.replace("'", "''")))
                } else {
                    Ok(format!("content LIKE '%{}%'", pattern.replace("'", "''")))
                }
            }
            QueryCondition::Not { condition } => {
                let clause = self.build_where_clause(condition)?;
                Ok(format!("NOT ({clause})"))
            }
            QueryCondition::And { conditions } => {
                let mut clauses = Vec::new();
                
                for condition in conditions {
                    let clause = self.build_where_clause(condition)?;
                    clauses.push(format!("({clause})"));
                }
                
                Ok(clauses.join(" AND "))
            }
            QueryCondition::Or { conditions } => {
                let mut clauses = Vec::new();
                
                for condition in conditions {
                    let clause = self.build_where_clause(condition)?;
                    clauses.push(format!("({clause})"));
                }
                
                Ok(clauses.join(" OR "))
            }
        }
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
}

impl DuckDBPersistentEngine {
    pub fn create_index(index_path: &str, file_pattern: &str) -> Result<()> {
        // Create persistent DuckDB connection
        let conn = Connection::open(index_path)?;
        
        // Create table schema
        conn.execute(
            "CREATE TABLE IF NOT EXISTS messages (
                file_path VARCHAR,
                line_number INTEGER,
                uuid VARCHAR,
                timestamp VARCHAR,
                session_id VARCHAR,
                role VARCHAR,
                content TEXT,
                has_tools BOOLEAN,
                has_thinking BOOLEAN,
                message_type VARCHAR,
                project_path VARCHAR,
                raw_json TEXT
            )",
            [],
        )?;

        // Clear existing data
        conn.execute("DELETE FROM messages", [])?;

        // Install and load FTS extension
        conn.execute_batch(
            "INSTALL fts;
             LOAD fts;",
        )?;

        // Discover files
        let expanded_pattern = expand_tilde(file_pattern);
        let files = if expanded_pattern.is_file() {
            vec![expanded_pattern]
        } else {
            discover_claude_files(Some(file_pattern))?
        };

        eprintln!("Indexing {} files...", files.len());

        // Load files
        let engine = DuckDBSearchEngine {
            options: SearchOptions::default(),
            conn: Arc::new(Mutex::new(conn)),
        };
        
        engine.load_files(&files)?;

        // Create FTS index
        {
            let conn = engine.conn.lock().unwrap();
            conn.execute_batch(
                "PRAGMA create_fts_index(
                    'messages',
                    'line_number',
                    'content',
                    stemmer='none',
                    stopwords='none',
                    ignore='\\\\W+',
                    strip_accents=0,
                    lower=1,
                    overwrite=1
                );",
            )?;
        }

        eprintln!("Index created successfully at: {}", index_path);
        Ok(())
    }

    pub fn open(index_path: &str, options: SearchOptions) -> Result<Self> {
        let conn = Connection::open(index_path)?;
        
        // Load FTS extension
        conn.execute_batch("LOAD fts;")?;
        
        Ok(Self {
            options,
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    pub fn search(&self, query: QueryCondition) -> Result<(Vec<SearchResult>, std::time::Duration, usize)> {
        let start_time = std::time::Instant::now();
        
        // Execute search query on indexed data
        let search_start = std::time::Instant::now();
        let results = self.execute_search(&query)?;
        let search_time = search_start.elapsed();

        let elapsed = start_time.elapsed();

        if self.options.verbose {
            eprintln!("Search execution took: {}ms", search_time.as_millis());
        }

        let total_count = results.len();
        let max_results = self.options.max_results.unwrap_or(50);
        let limited_results = results.into_iter().take(max_results).collect();

        Ok((limited_results, elapsed, total_count))
    }

    fn execute_search(&self, query: &QueryCondition) -> Result<Vec<SearchResult>> {
        let engine = DuckDBSearchEngine {
            options: self.options.clone(),
            conn: self.conn.clone(),
        };
        engine.execute_search(query)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::parse_query;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_duckdb_search_engine() -> Result<()> {
        let temp_dir = tempdir()?;
        let test_file = temp_dir.path().join("test.jsonl");

        // Create test data
        let mut file = std::fs::File::create(&test_file)?;
        writeln!(
            file,
            r#"{{"type":"user","message":{{"role":"user","content":"Hello world"}},"uuid":"123","timestamp":"2024-01-01T00:00:00Z","sessionId":"session1","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}}"#
        )?;
        writeln!(
            file,
            r#"{{"type":"assistant","message":{{"id":"msg1","type":"message","role":"assistant","model":"claude","content":[{{"type":"text","text":"Hi there!"}}],"stop_reason":"end_turn","stop_sequence":null,"usage":{{"input_tokens":10,"cache_creation_input_tokens":0,"cache_read_input_tokens":0,"output_tokens":5}}}},"uuid":"124","timestamp":"2024-01-01T00:00:01Z","sessionId":"session1","parentUuid":"123","isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}}"#
        )?;

        // Search for "Hello"
        let options = SearchOptions::default();
        let engine = DuckDBSearchEngine::new(options)?;
        let query = parse_query("Hello")?;
        let (results, _, _) = engine.search(test_file.to_str().unwrap(), query)?;

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].role, "user");
        assert!(results[0].text.contains("Hello world"));

        Ok(())
    }
}