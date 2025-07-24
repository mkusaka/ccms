use anyhow::Result;
use ccms::query::{parse_query, SearchOptions};
use ccms::search::format_search_result;
#[cfg(feature = "duckdb")]
use ccms::search::DuckDBSearchEngine;
use clap::Parser;
use colored::Colorize;

#[derive(Parser, Debug)]
#[command(
    name = "ccms-duckdb",
    about = "DuckDB-based Claude session search",
    version
)]
struct Args {
    /// Search query (supports AND, OR, NOT, and regex patterns)
    query: String,

    /// File pattern to search (defaults to Claude session files)
    #[arg(default_value = "~/.claude/projects/*/*.jsonl")]
    file_pattern: String,

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
}

fn main() -> Result<()> {
    let args = Args::parse();

    // Parse query
    let query_condition = parse_query(&args.query)?;

    // Configure search options
    let options = SearchOptions {
        max_results: Some(args.max_results),
        role: args.role,
        session_id: args.session_id,
        before: args.before,
        after: args.after,
        verbose: args.verbose,
        project_path: args.project_path,
    };

    // Create DuckDB search engine
    let engine = DuckDBSearchEngine::new(options)?;

    // Perform search
    let (results, duration, total_count) = engine.search(&args.file_pattern, query_condition)?;

    // Display timing information
    if args.no_color {
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
    if total_count > args.max_results {
        if args.no_color {
            eprintln!("(Showing {} of {} total results)", results.len(), total_count);
        } else {
            eprintln!(
                "(Showing {} of {} total results)",
                results.len().to_string().bright_yellow(),
                total_count.to_string().bright_yellow()
            );
        }
    } else {
        if args.no_color {
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
        println!("{}", format_search_result(&result, !args.no_color, args.full));
    }

    Ok(())
}