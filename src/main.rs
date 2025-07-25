use anyhow::Result;
use ccms::{
    SearchEngine, SearchOptions, default_claude_pattern, format_search_result,
    interactive_ratatui::run_interactive_search, parse_query, profiling,
};
use chrono::{DateTime, Local, Utc};
use clap::{Command, CommandFactory, Parser, ValueEnum};
use clap_complete::{Generator, Shell, generate};
use parse_datetime::parse_datetime_at_date;
use std::io::{self, Write};

#[derive(Parser)]
#[command(
    name = "ccms",
    version,
    about = "High-performance CLI for searching Claude session JSONL files",
    long_about = None
)]
struct Cli {
    /// Search query (supports literal, regex, AND/OR/NOT operators)
    #[arg(required_unless_present_any = ["interactive", "generator"])]
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

    /// Filter messages since this time (Unix timestamp or relative time like "1 day ago")
    #[arg(long)]
    since: Option<String>,

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

    /// Show raw JSON of matched messages
    #[arg(long)]
    raw: bool,

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

    /// Generate shell completion script
    #[arg(long = "completion", value_enum)]
    generator: Option<Shell>,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum OutputFormat {
    Text,
    Json,
    JsonL,
}

fn print_completions<G: Generator>(generator: G, cmd: &mut Command) {
    generate(
        generator,
        cmd,
        cmd.get_name().to_string(),
        &mut io::stdout(),
    );
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Handle completion generation
    if let Some(generator) = cli.generator {
        let mut cmd = Cli::command();
        eprintln!("Generating completion file for {generator:?}...");
        print_completions(generator, &mut cmd);
        return Ok(());
    }

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
        eprintln!(
            "Warning: Profiling is not enabled. Build with --features profiling to enable profiling."
        );
    }

    // Parse --since flag if provided
    let parsed_after = if let Some(since) = &cli.since {
        match parse_since_time(since) {
            Ok(dt) => Some(dt),
            Err(e) => {
                eprintln!("Error parsing --since: {e}");
                std::process::exit(1);
            }
        }
    } else {
        cli.after.clone()
    };

    // Get pattern
    let default_pattern = default_claude_pattern();
    let pattern = cli.pattern.as_deref().unwrap_or(&default_pattern);

    // Interactive mode
    if cli.interactive {
        return run_interactive_search(
            Some(pattern.to_string()),
            parsed_after.clone(),
            cli.before.clone(),
            cli.session_id.clone(),
        );
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
        after: parsed_after,
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
            } else if cli.raw {
                // Raw mode: output raw JSON lines
                for result in &results {
                    if let Some(raw_json) = &result.raw_json {
                        println!("{raw_json}");
                    }
                }
            } else {
                println!("Found {} results:\n", results.len());
                for result in &results {
                    println!(
                        "{}",
                        format_search_result(result, !cli.no_color, cli.full_text)
                    );
                }

                // Print search statistics
                eprintln!("\n⏱️  Search completed in {}ms", duration.as_millis());
                if total_count > results.len() {
                    eprintln!(
                        "(Showing {} of {} total results)",
                        results.len(),
                        total_count
                    );
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
            eprintln!("Profiling report saved to {profile_path}.svg");
        }
    }

    Ok(())
}

fn parse_since_time(input: &str) -> Result<String> {
    use anyhow::Context;

    // First, try to parse as Unix timestamp
    if let Ok(timestamp) = input.parse::<i64>() {
        let dt = DateTime::<Utc>::from_timestamp(timestamp, 0).context("Invalid Unix timestamp")?;
        return Ok(dt.to_rfc3339());
    }

    // Try to parse as relative time using parse_datetime
    let now = Local::now();
    match parse_datetime_at_date(now, input) {
        Ok(dt) => Ok(dt.to_rfc3339()),
        Err(e) => Err(anyhow::anyhow!(
            "Failed to parse time '{}': {}. Expected Unix timestamp or relative time like '1 day ago'",
            input,
            e
        )),
    }
}

fn print_query_help() {
    println!(
        r#"Claude Search Query Syntax Help

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
  - AND has higher precedence than OR"#
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_unix_timestamp() {
        // Test Unix timestamp parsing
        let result = parse_since_time("1704067200"); // 2024-01-01 00:00:00 UTC
        assert!(result.is_ok());
        let dt = result.unwrap();
        assert!(dt.starts_with("2024-01-01"));
    }

    #[test]
    fn test_parse_relative_time() {
        // Test relative time parsing
        let result = parse_since_time("1 hour ago");
        assert!(result.is_ok());
        // Just check it parses correctly - exact time depends on when test runs
    }

    #[test]
    fn test_parse_invalid_time() {
        // Test invalid input
        let result = parse_since_time("invalid time");
        assert!(result.is_err());
    }
}
