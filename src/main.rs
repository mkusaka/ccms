use clap::{Parser, ValueEnum};
use anyhow::Result;
use claude_search::{
    SearchEngine, SearchOptions, parse_query,
    default_claude_pattern, format_search_result, profiling,
    interactive::InteractiveSearch,
};
use std::io::{self, Write};

#[derive(Parser)]
#[command(
    name = "claude-search",
    version,
    about = "High-performance CLI for searching Claude session JSONL files",
    long_about = None
)]
struct Cli {
    /// Search query (supports literal, regex, AND/OR/NOT operators)
    #[arg(required_unless_present = "interactive")]
    query: Option<String>,
    
    /// File pattern to search (default: ~/.claude/projects/**/*.jsonl)
    #[arg(short, long)]
    pattern: Option<String>,
    
    /// Filter by message role (user, assistant, system, summary)
    #[arg(short, long)]
    role: Option<String>,
    
    /// Filter by session ID
    #[arg(short, long)]
    session_id: Option<String>,
    
    /// Maximum number of results to return
    #[arg(short = 'n', long, default_value = "50")]
    max_results: usize,
    
    /// Filter messages before this timestamp (RFC3339 format)
    #[arg(long)]
    before: Option<String>,
    
    /// Filter messages after this timestamp (RFC3339 format)
    #[arg(long)]
    after: Option<String>,
    
    /// Output format
    #[arg(short = 'f', long, value_enum, default_value = "text")]
    format: OutputFormat,
    
    /// Disable colored output
    #[arg(long)]
    no_color: bool,
    
    /// Enable verbose output
    #[arg(short, long)]
    verbose: bool,
    
    /// Show query syntax help
    #[arg(long)]
    help_query: bool,
    
    /// Show full message text without truncation
    #[arg(long)]
    full_text: bool,
    
    /// Interactive search mode (fzf-like)
    #[arg(short = 'i', long)]
    interactive: bool,
    
    /// Filter by project path (e.g., current directory: $(pwd))
    #[arg(long = "project")]
    project_path: Option<String>,
    
    /// Generate profiling report (requires --features profiling)
    #[cfg(feature = "profiling")]
    #[arg(long)]
    profile: Option<String>,
    
    #[cfg(not(feature = "profiling"))]
    #[arg(long, hide = true)]
    profile: Option<String>,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum OutputFormat {
    Text,
    Json,
    JsonL,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    
    // Initialize tracing
    profiling::init_tracing();
    
    if cli.help_query {
        print_query_help();
        return Ok(());
    }
    
    // Initialize profiler if requested
    #[cfg(feature = "profiling")]
    let mut profiler = if cli.profile.is_some() {
        Some(profiling::Profiler::new()?)
    } else {
        None
    };
    
    #[cfg(not(feature = "profiling"))]
    if cli.profile.is_some() {
        eprintln!("Warning: Profiling is not enabled. Build with --features profiling to enable profiling.");
    }
    
    // Get pattern
    let default_pattern = default_claude_pattern();
    let pattern = cli.pattern.as_deref()
        .unwrap_or(&default_pattern);
    
    // Interactive mode
    if cli.interactive {
        let options = SearchOptions {
            max_results: Some(cli.max_results * 20), // Load more for interactive
            role: cli.role,
            session_id: cli.session_id,
            before: cli.before,
            after: cli.after,
            verbose: cli.verbose,
            project_path: cli.project_path.clone(),
        };
        
        let mut interactive = InteractiveSearch::new(options);
        return interactive.run(pattern);
    }
    
    // Regular search mode - query is required
    let query_str = cli.query.ok_or_else(|| {
        anyhow::anyhow!("Query argument is required (use --interactive for interactive mode)")
    })?;
    
    // Parse the query
    let query = match parse_query(&query_str) {
        Ok(q) => q,
        Err(e) => {
            eprintln!("Error parsing query: {e}");
            eprintln!("Use --help-query for query syntax help");
            std::process::exit(1);
        }
    };
    
    // Create search options
    let options = SearchOptions {
        max_results: Some(cli.max_results),
        role: cli.role,
        session_id: cli.session_id,
        before: cli.before,
        after: cli.after,
        verbose: cli.verbose,
        project_path: cli.project_path,
    };
    
    if cli.verbose {
        eprintln!("Searching in: {pattern}");
        eprintln!("Query: {query:?}");
    }
    
    // Create search engine and search
    let engine = SearchEngine::new(options);
    
    // Debug: only search specific file
    let debug_file = "/Users/masatomokusaka/.claude/projects/-Users-masatomokusaka-src-github-com-mkusaka-bookmark-agent/9ca2db47-82d6-4da7-998e-3d7cd28ce5b5.jsonl";
    let pattern_to_use = if std::env::var("DEBUG_SINGLE_FILE").is_ok() {
        eprintln!("DEBUG: Searching only {debug_file}");
        debug_file
    } else {
        pattern
    };
    
    let (results, duration, total_count) = engine.search(pattern_to_use, query)?;
    
    // Output results
    let stdout = io::stdout();
    let mut handle = stdout.lock();
    
    match cli.format {
        OutputFormat::Text => {
            if results.is_empty() {
                println!("No results found.");
            } else {
                println!("Found {} results:\n", results.len());
                for result in &results {
                    println!("{}", format_search_result(result, !cli.no_color, cli.full_text));
                }
                
                // Print search statistics
                eprintln!("\n⏱️  Search completed in {}ms", duration.as_millis());
                if total_count > results.len() {
                    eprintln!("(Showing {} of {} total results)", results.len(), total_count);
                } else {
                    eprintln!("(Found {total_count} results)");
                }
            }
        }
        OutputFormat::Json => {
            let output = serde_json::json!({
                "results": results,
                "duration_ms": duration.as_millis(),
                "total_count": total_count,
                "returned_count": results.len()
            });
            serde_json::to_writer_pretty(&mut handle, &output)?;
            writeln!(&mut handle)?;
        }
        OutputFormat::JsonL => {
            for result in &results {
                serde_json::to_writer(&mut handle, result)?;
                writeln!(&mut handle)?;
            }
            // Write metadata as last line
            let metadata = serde_json::json!({
                "_metadata": {
                    "duration_ms": duration.as_millis(),
                    "total_count": total_count,
                    "returned_count": results.len()
                }
            });
            serde_json::to_writer(&mut handle, &metadata)?;
            writeln!(&mut handle)?;
        }
    }
    
    // Generate profiling report if requested
    #[cfg(feature = "profiling")]
    if let Some(ref mut profiler) = profiler {
        if let Some(profile_path) = &cli.profile {
            profiler.report(profile_path)?;
            eprintln!("Profiling report saved to {}.svg", profile_path);
        }
    }
    
    Ok(())
}

fn print_query_help() {
    println!(r#"Claude Search Query Syntax Help

BASIC QUERIES:
  hello                   Literal search (case-insensitive)
  "hello world"          Quoted literal (preserves spaces)
  'hello world'          Single-quoted literal
  /hello.*world/i        Regular expression with flags

OPERATORS:
  hello AND world        Both terms must be present
  hello OR world         Either term must be present
  NOT hello              Term must not be present
  (hello OR hi) AND bye  Parentheses for grouping

REGEX FLAGS:
  i - Case insensitive
  m - Multi-line mode
  s - Dot matches newline

EXAMPLES:
  error AND /failed.*connection/i
  "user message" AND NOT system
  (warning OR error) AND timestamp
  /^Error:.*\d+/m

ROLE FILTERS (via --role):
  user, assistant, system, summary

TIPS:
  - Unquoted literals cannot contain spaces or special characters
  - Use quotes for exact phrases with spaces
  - Regular expressions must be enclosed in forward slashes
  - AND has higher precedence than OR"#);
}
