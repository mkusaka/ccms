use anyhow::Result;
use ccms::query::{parse_query, SearchOptions};
use ccms::search::{format_search_result, DuckDBPersistentEngine};
use clap::{Parser, Subcommand};
use colored::Colorize;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "ccms-duckdb-persistent",
    about = "DuckDB-based Claude session search with persistent index",
    version
)]
struct Args {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Create or update the search index
    Index {
        /// File pattern to index (defaults to Claude session files)
        #[arg(default_value = "~/.claude/projects/*/*.jsonl")]
        file_pattern: String,
        
        /// Path to the DuckDB index file
        #[arg(short, long, default_value = "~/.claude/ccms-index.duckdb")]
        index_path: String,
    },
    
    /// Search using the existing index
    Search {
        /// Search query (supports AND, OR, NOT, and regex patterns)
        query: String,
        
        /// Path to the DuckDB index file
        #[arg(short, long, default_value = "~/.claude/ccms-index.duckdb")]
        index_path: String,
        
        /// Maximum number of results to display
        #[arg(short = 'n', long, default_value = "50")]
        max_results: usize,

        /// Show full message content instead of preview
        #[arg(short, long)]
        full: bool,

        /// Filter by role (user, assistant, system, summary)
        #[arg(short, long)]
        role: Option<String>,

        /// Filter by session ID
        #[arg(short, long)]
        session_id: Option<String>,

        /// Filter by timestamp (messages before this time)
        #[arg(short, long)]
        before: Option<String>,

        /// Filter by timestamp (messages after this time)
        #[arg(short, long)]
        after: Option<String>,

        /// Filter by project path
        #[arg(short = 'p', long)]
        project_path: Option<String>,

        /// Disable colored output
        #[arg(long)]
        no_color: bool,

        /// Show verbose output
        #[arg(short, long)]
        verbose: bool,
    },
}

fn expand_home_path(path: &str) -> PathBuf {
    if path.starts_with("~") {
        if let Some(home) = dirs::home_dir() {
            return home.join(&path[2..]);
        }
    }
    PathBuf::from(path)
}

fn main() -> Result<()> {
    let args = Args::parse();
    
    match args.command {
        Commands::Index { file_pattern, index_path } => {
            let index_path = expand_home_path(&index_path);
            let start = std::time::Instant::now();
            
            DuckDBPersistentEngine::create_index(
                index_path.to_str().unwrap(), 
                &file_pattern
            )?;
            
            let duration = start.elapsed();
            eprintln!(
                "{}  Indexing completed in {}ms",
                "✅".bright_green(),
                duration.as_millis().to_string().bright_green()
            );
        }
        
        Commands::Search {
            query,
            index_path,
            max_results,
            full,
            role,
            session_id,
            before,
            after,
            project_path,
            no_color,
            verbose,
        } => {
            let index_path = expand_home_path(&index_path);
            
            // Check if index exists
            if !index_path.exists() {
                eprintln!(
                    "{}  Index file not found: {}",
                    "❌".bright_red(),
                    index_path.display()
                );
                eprintln!("Run 'ccms-duckdb-persistent index' first to create the index.");
                std::process::exit(1);
            }
            
            // Parse query
            let query_condition = parse_query(&query)?;

            // Configure search options
            let options = SearchOptions {
                max_results: Some(max_results),
                role,
                session_id,
                before,
                after,
                verbose,
                project_path,
            };

            // Open persistent index and search
            let engine = DuckDBPersistentEngine::open(
                index_path.to_str().unwrap(),
                options
            )?;
            
            let (results, duration, total_count) = engine.search(query_condition)?;

            // Display timing information
            if no_color {
                eprintln!("⏱️  Search completed in {}ms", duration.as_millis());
            } else {
                eprintln!(
                    "{}  Search completed in {}ms",
                    "⏱️".bright_blue(),
                    duration.as_millis().to_string().bright_green()
                );
            }

            if results.is_empty() {
                eprintln!("No results found");
                return Ok(());
            }

            // Show result count
            if total_count > max_results {
                if no_color {
                    eprintln!("(Showing {} of {} total results)", results.len(), total_count);
                } else {
                    eprintln!(
                        "(Showing {} of {} total results)",
                        results.len().to_string().bright_yellow(),
                        total_count.to_string().bright_yellow()
                    );
                }
            } else {
                if no_color {
                    eprintln!("Found {} results:", results.len());
                } else {
                    eprintln!(
                        "Found {} results:",
                        results.len().to_string().bright_green()
                    );
                }
            }

            eprintln!();

            // Display results
            for result in results {
                println!("{}", format_search_result(&result, !no_color, full));
            }
        }
    }
    
    Ok(())
}