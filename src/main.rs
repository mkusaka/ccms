#[cfg(feature = "mimalloc")]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

use anyhow::Result;
#[cfg(feature = "profiling")]
use ccms::profiling_enhanced;
use ccms::{
    RayonEngine, SearchEngineTrait, SearchOptions, SearchResult, SmolEngine,
    default_claude_pattern, format_search_result, interactive_ratatui::InteractiveSearch,
    parse_query, profiling,
};
use chrono::{DateTime, Local, Utc};
use clap::{Command, CommandFactory, Parser, ValueEnum};
use clap_complete::{Generator, Shell, generate};
use parse_datetime::parse_datetime_at_date;
use std::collections::HashMap;
use std::io::{self, Write};

#[derive(Parser)]
#[command(
    name = "ccms",
    version,
    about = "High-performance CLI for searching Claude session JSONL files",
    long_about = None
)]
struct Cli {
    /// Search query (supports literal, regex, AND/OR/NOT operators). If not provided, enters interactive mode.
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

    /// Search for a specific message by UUID
    #[arg(long)]
    message_id: Option<String>,

    /// Maximum number of results to return
    #[arg(short = 'n', long, default_value = "200")]
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

    /// Filter by working directory (cwd) path
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

    /// Search engine to use
    #[arg(long, value_enum, default_value = "smol")]
    engine: EngineType,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum OutputFormat {
    Text,
    Json,
    JsonL,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum EngineType {
    Smol,
    Rayon,
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
        Some(profiling_enhanced::EnhancedProfiler::new("main")?)
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

    // Set default project_path to current directory if not specified
    let project_path = cli.project_path.clone().or_else(|| {
        std::env::current_dir()
            .ok()
            .and_then(|path| path.to_str().map(|s| s.to_string()))
    });

    // Get pattern
    let default_pattern = default_claude_pattern();
    let pattern = cli.pattern.as_deref().unwrap_or(&default_pattern);

    // Handle --message-id search
    if let Some(message_id) = &cli.message_id {
        // Create a special query to search for the UUID
        let query = parse_query(message_id)?;

        // Create search options
        let options = SearchOptions {
            max_results: Some(1), // We only need one result
            role: None,
            session_id: None,
            message_id: Some(message_id.clone()),
            before: None,
            after: None,
            verbose: cli.verbose,
            project_path: None,
        };

        if cli.verbose {
            eprintln!("Searching for message ID: {message_id}");
        }

        // Execute search
        let engine = SmolEngine::new(options);
        let (results, duration, _) = engine.search(pattern, query)?;

        if results.is_empty() {
            eprintln!("Message with ID '{message_id}' not found.");
            std::process::exit(1);
        }

        // Pretty print the message
        let result = &results[0];
        print_message_details(result, !cli.no_color);

        if cli.verbose {
            eprintln!("\n⏱️  Search completed in {}ms", duration.as_millis());
        }

        return Ok(());
    }

    // Interactive mode when no query provided or query is empty
    if cli.query.is_none() || cli.query.as_ref().map(|s| s.is_empty()).unwrap_or(false) {
        let options = SearchOptions {
            max_results: Some(cli.max_results), // Use the CLI value directly
            role: cli.role,
            session_id: cli.session_id,
            message_id: None,
            before: cli.before,
            after: parsed_after.clone(),
            verbose: cli.verbose,
            project_path: project_path.clone(),
        };

        let mut interactive = InteractiveSearch::new(options);
        return interactive.run(pattern);
    }

    // Regular search mode - query is provided
    let query_str = cli.query.unwrap();

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
        message_id: None,
        before: cli.before,
        after: parsed_after,
        verbose: cli.verbose,
        project_path,
    };

    if cli.verbose {
        eprintln!("Searching in: {pattern}");
        eprintln!("Query: {query:?}");
    }

    // Debug: only search specific file
    let debug_file = "/Users/masatomokusaka/.claude/projects/-Users-masatomokusaka-src-github-com-mkusaka-bookmark-agent/9ca2db47-82d6-4da7-998e-3d7cd28ce5b5.jsonl";
    let pattern_to_use = if std::env::var("DEBUG_SINGLE_FILE").is_ok() {
        eprintln!("DEBUG: Searching only {debug_file}");
        debug_file
    } else {
        pattern
    };

    // Execute search
    if cli.verbose {
        eprintln!(
            "Using {} engine",
            match cli.engine {
                EngineType::Smol => "Smol",
                EngineType::Rayon => "Rayon",
            }
        );
    }

    // Create appropriate engine based on CLI flag
    let (results, duration, total_count) = match cli.engine {
        EngineType::Smol => {
            let engine = SmolEngine::new(options);
            engine.search(pattern_to_use, query)?
        }
        EngineType::Rayon => {
            let engine = RayonEngine::new(options);
            engine.search(pattern_to_use, query)?
        }
    };

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
            // Collect statistics
            let mut session_counts: HashMap<String, usize> = HashMap::new();
            let mut file_counts: HashMap<String, usize> = HashMap::new();

            for result in &results {
                *session_counts.entry(result.session_id.clone()).or_insert(0) += 1;
                *file_counts.entry(result.file.clone()).or_insert(0) += 1;
            }

            // Create detailed file information
            let files_detail: Vec<_> = file_counts
                .iter()
                .map(|(file, count)| {
                    serde_json::json!({
                        "path": file,
                        "message_count": count,
                        "session_id": results.iter()
                            .find(|r| &r.file == file)
                            .map(|r| &r.session_id)
                            .unwrap_or(&String::new())
                    })
                })
                .collect();

            // Create detailed session information
            let sessions_detail: Vec<_> = session_counts
                .iter()
                .map(|(session_id, count)| {
                    serde_json::json!({
                        "session_id": session_id,
                        "message_count": count
                    })
                })
                .collect();

            let output = serde_json::json!({
                "results": results,
                "summary": {
                    "duration_ms": duration.as_millis(),
                    "total_count": total_count,
                    "returned_count": results.len(),
                    "unique_sessions": session_counts.len(),
                    "unique_files": file_counts.len()
                },
                "files": files_detail,
                "sessions": sessions_detail
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
            let report = profiler.generate_comprehensive_report(profile_path)?;
            eprintln!("\n{report}");
            eprintln!(
                "\nDetailed profiling reports saved to {profile_path}_{{comprehensive.txt,svg}}"
            );
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

fn print_message_details(result: &SearchResult, use_color: bool) {
    use chrono::{DateTime, Local, TimeZone};
    use colored::Colorize;

    // Parse timestamp to local time
    let timestamp = if let Ok(dt) = DateTime::parse_from_rfc3339(&result.timestamp) {
        let local_dt = Local.from_utc_datetime(&dt.naive_utc());
        local_dt.format("%Y-%m-%d %H:%M:%S").to_string()
    } else {
        result.timestamp.clone()
    };

    // Pretty print the message details
    if use_color {
        println!("{}", "Message Details".bright_blue().bold());
        println!("{}", "═".repeat(80).bright_blue());
        println!("{}: {}", "Message ID".bright_yellow(), result.uuid);
        println!("{}: {}", "Type".bright_yellow(), result.message_type);
        println!("{}: {}", "Role".bright_yellow(), result.role.bright_green());
        println!(
            "{}: {}",
            "Timestamp".bright_yellow(),
            timestamp.bright_cyan()
        );
        println!(
            "{}: {}",
            "Session ID".bright_yellow(),
            result.session_id.dimmed()
        );
        println!(
            "{}: {}",
            "File".bright_yellow(),
            result.file.bright_magenta()
        );
        println!("{}: {}", "Working Dir".bright_yellow(), result.cwd.dimmed());
        println!("\n{}", "Content".bright_yellow().bold());
        println!("{}", "─".repeat(80).bright_blue());

        // Print the full content with nice formatting
        let content_lines: Vec<&str> = result.text.lines().collect();
        for line in content_lines {
            println!("{line}");
        }

        if let Some(raw_json) = &result.raw_json {
            println!("\n{}", "Raw JSON".bright_yellow().bold());
            println!("{}", "─".repeat(80).bright_blue());
            // Pretty print JSON if possible
            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(raw_json) {
                if let Ok(pretty) = serde_json::to_string_pretty(&parsed) {
                    println!("{}", pretty.dimmed());
                } else {
                    println!("{}", raw_json.dimmed());
                }
            } else {
                println!("{}", raw_json.dimmed());
            }
        }
    } else {
        println!("Message Details");
        println!("{}", "=".repeat(80));
        println!("Message ID: {}", result.uuid);
        println!("Type: {}", result.message_type);
        println!("Role: {}", result.role);
        println!("Timestamp: {timestamp}");
        println!("Session ID: {}", result.session_id);
        println!("File: {}", result.file);
        println!("Working Dir: {}", result.cwd);
        println!("\nContent");
        println!("{}", "-".repeat(80));

        // Print the full content
        println!("{}", result.text);

        if let Some(raw_json) = &result.raw_json {
            println!("\nRaw JSON");
            println!("{}", "-".repeat(80));
            // Pretty print JSON if possible
            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(raw_json) {
                if let Ok(pretty) = serde_json::to_string_pretty(&parsed) {
                    println!("{pretty}");
                } else {
                    println!("{raw_json}");
                }
            } else {
                println!("{raw_json}");
            }
        }
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

    #[test]
    fn test_empty_query_detection() {
        // Test that empty string queries are detected correctly
        let empty_string = Some(String::from(""));
        let non_empty_string = Some(String::from("test"));
        let none_query: Option<String> = None;

        // Empty string should be detected as empty
        assert!(empty_string.as_ref().map(|s| s.is_empty()).unwrap_or(false));

        // Non-empty string should not be detected as empty
        assert!(
            !non_empty_string
                .as_ref()
                .map(|s| s.is_empty())
                .unwrap_or(false)
        );

        // None should trigger the first condition (is_none())
        assert!(none_query.is_none());

        // The combined condition used in main should be true for both None and empty string
        assert!(none_query.is_none() || none_query.as_ref().map(|s| s.is_empty()).unwrap_or(false));
        assert!(
            empty_string.is_none() || empty_string.as_ref().map(|s| s.is_empty()).unwrap_or(false)
        );
        assert!(
            !(non_empty_string.is_none()
                || non_empty_string
                    .as_ref()
                    .map(|s| s.is_empty())
                    .unwrap_or(false))
        );
    }
}
