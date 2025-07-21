#[cfg(test)]
mod tests;

use anyhow::Result;
use colored::Colorize;
use console::{Term, Key, style};
use std::io::{self, Write};

use crate::{SearchEngine, SearchOptions, SearchResult, parse_query};

pub struct InteractiveSearch {
    base_options: SearchOptions,
    max_results: usize,
}

impl InteractiveSearch {
    pub fn new(options: SearchOptions) -> Self {
        let max_results = options.max_results.unwrap_or(50);
        Self {
            base_options: options,
            max_results,
        }
    }
    
    pub fn run(&mut self, pattern: &str) -> Result<()> {
        let mut stdout = io::stdout();
        let term = Term::stdout();
        
        // Print headers
        println!("{}", "Interactive Claude Search".cyan());
        println!("{}", "Type to search, ↑/↓ to navigate, Enter to select, Tab for role filter, Esc/Ctrl+C to exit".dimmed());
        println!();
        
        let mut query = String::new();
        let mut selected_index = 0;
        let mut results: Vec<SearchResult> = Vec::new();
        let mut role_filter: Option<String> = None;
        
        // Remember where we start displaying results
        let result_start_row = 4; // After header + empty line + search prompt
        
        loop {
            // Clear and redraw search prompt
            term.move_cursor_to(0, 3)?;
            term.clear_line()?;
            if let Some(ref role) = role_filter {
                print!("Search [{}]: {}", role.yellow(), query);
            } else {
                print!("Search: {query}");
            }
            stdout.flush()?;
            
            // Clear result area
            term.move_cursor_to(0, result_start_row)?;
            term.clear_to_end_of_screen()?;
            
            // Display results
            if !query.is_empty() {
                if !results.is_empty() {
                    // Check if we hit the limit
                    if results.len() >= self.max_results {
                        println!("Found {} results (limit reached)", self.max_results);
                    } else {
                        println!("Found {} results", results.len());
                    }
                    println!();
                    
                    let display_count = results.len().min(10);
                    for (idx, result) in results.iter().take(display_count).enumerate() {
                        let line = self.format_result_line(result, idx);
                        if idx == selected_index {
                            println!("{} {}", ">".cyan(), style(&line).bold());
                        } else {
                            println!("  {line}");
                        }
                    }
                    
                    if results.len() > 10 {
                        println!();
                        if results.len() >= self.max_results {
                            println!("{}", format!("... {} more results shown (and more...)", results.len() - 10).dimmed());
                        } else {
                            println!("{}", format!("... and {} more results", results.len() - 10).dimmed());
                        }
                    }
                } else {
                    println!("{}", "No results".yellow());
                }
            }
            
            // Move cursor back to end of search prompt
            let prompt_len = if let Some(ref role) = role_filter {
                8 + role.len() + 3 // "Search [role]: "
            } else {
                8 // "Search: "
            };
            term.move_cursor_to(prompt_len + query.len(), 3)?;
            stdout.flush()?;
            
            // Read key
            match term.read_key()? {
                Key::Char(c) => {
                    query.push(c);
                    selected_index = 0;
                    
                    // Execute search
                    if !query.is_empty() {
                        if let Ok(parsed_query) = parse_query(&query) {
                            // Create engine with current role filter
                            let mut options = self.base_options.clone();
                            options.role = role_filter.clone();
                            let engine = SearchEngine::new(options);
                            if let Ok((search_results, _, _)) = engine.search(pattern, parsed_query) {
                                results = search_results;
                            } else {
                                results.clear();
                            }
                        } else {
                            results.clear();
                        }
                    } else {
                        results.clear();
                    }
                }
                Key::Backspace => {
                    query.pop();
                    selected_index = 0;
                    
                    // Re-execute search
                    if !query.is_empty() {
                        if let Ok(parsed_query) = parse_query(&query) {
                            // Create engine with current role filter
                            let mut options = self.base_options.clone();
                            options.role = role_filter.clone();
                            let engine = SearchEngine::new(options);
                            if let Ok((search_results, _, _)) = engine.search(pattern, parsed_query) {
                                results = search_results;
                            } else {
                                results.clear();
                            }
                        } else {
                            results.clear();
                        }
                    } else {
                        results.clear();
                    }
                }
                Key::ArrowUp => {
                    selected_index = selected_index.saturating_sub(1);
                }
                Key::ArrowDown => {
                    if selected_index < results.len().saturating_sub(1).min(9) {
                        selected_index += 1;
                    }
                }
                Key::Enter => {
                    if !results.is_empty() && selected_index < results.len() {
                        // Clear screen for full display
                        term.clear_screen()?;
                        
                        // Display full result
                        self.display_full_result(&results[selected_index])?;
                        
                        // Handle action selection
                        match term.read_key()? {
                            Key::Char('s') | Key::Char('S') => {
                                self.view_session(&results[selected_index], &term)?;
                            }
                            Key::Char('f') | Key::Char('F') => {
                                self.copy_to_clipboard(&results[selected_index].file)?;
                                println!("\n{}", "File path copied to clipboard!".green());
                                term.read_key()?;
                            }
                            Key::Char('i') | Key::Char('I') => {
                                self.copy_to_clipboard(&results[selected_index].session_id)?;
                                println!("\n{}", "Session ID copied to clipboard!".green());
                                term.read_key()?;
                            }
                            Key::Char('p') | Key::Char('P') => {
                                self.copy_to_clipboard(&results[selected_index].project_path)?;
                                println!("\n{}", "Project path copied to clipboard!".green());
                                term.read_key()?;
                            }
                            _ => {}
                        }
                        
                        // Restore screen
                        term.clear_screen()?;
                        println!("{}", "Interactive Claude Search".cyan());
                        println!("{}", "Type to search, ↑/↓ to navigate, Enter to select, Esc/Ctrl+C to exit".dimmed());
                        println!();
                    }
                }
                Key::Tab => {
                    // Cycle through role filters
                    role_filter = match role_filter {
                        None => Some("user".to_string()),
                        Some(ref r) if r == "user" => Some("assistant".to_string()),
                        Some(ref r) if r == "assistant" => Some("system".to_string()),
                        Some(ref r) if r == "system" => Some("summary".to_string()),
                        Some(ref r) if r == "summary" => None,
                        _ => None,
                    };
                    selected_index = 0;
                    
                    // Re-execute search with new filter
                    if !query.is_empty() {
                        if let Ok(parsed_query) = parse_query(&query) {
                            let mut options = self.base_options.clone();
                            options.role = role_filter.clone();
                            let engine = SearchEngine::new(options);
                            if let Ok((search_results, _, _)) = engine.search(pattern, parsed_query) {
                                results = search_results;
                            } else {
                                results.clear();
                            }
                        } else {
                            results.clear();
                        }
                    }
                }
                Key::Escape | Key::CtrlC => {
                    break;
                }
                _ => {}
            }
        }
        
        // Clear and exit
        term.move_cursor_to(0, 3)?;
        term.clear_to_end_of_screen()?;
        println!("\n{}", "Goodbye!".yellow());
        
        Ok(())
    }
    
    fn format_result_line(&self, result: &SearchResult, index: usize) -> String {
        use chrono::DateTime;
        
        let timestamp = if let Ok(dt) = DateTime::parse_from_rfc3339(&result.timestamp) {
            dt.format("%m/%d %H:%M").to_string()
        } else {
            result.timestamp.chars().take(16).collect()
        };
        
        let role = format!("[{}]", result.role.to_uppercase());
        let preview = result.text
            .replace('\n', " ")
            .chars()
            .take(40)
            .collect::<String>();
        
        format!(
            "{:2}. {:9} {} {}...",
            index + 1,
            style(&role).yellow(),
            timestamp.dimmed(),
            preview.dimmed()
        )
    }
    
    fn display_full_result(&self, result: &SearchResult) -> Result<()> {
        use chrono::DateTime;
        
        let separator = "─".repeat(80);
        let timestamp = if let Ok(dt) = DateTime::parse_from_rfc3339(&result.timestamp) {
            dt.format("%Y-%m-%d %H:%M:%S").to_string()
        } else {
            result.timestamp.clone()
        };
        
        println!("{}", separator.cyan());
        println!("{} {}", "Role:".yellow(), result.role);
        println!("{} {}", "Time:".yellow(), timestamp);
        println!("{} {}", "File:".yellow(), result.file);
        println!("{} {}", "Project:".yellow(), result.project_path);
        println!("{} {}", "UUID:".yellow(), result.uuid);
        println!("{} {}", "Session:".yellow(), result.session_id);
        println!("{}", separator.cyan());
        println!("{}", result.text);
        println!("{}", separator.cyan());
        
        // Show options
        println!();
        println!("{}:", "Actions".cyan());
        println!("  {} - View full session", "[S]".yellow());
        println!("  {} - Copy file path", "[F]".yellow());
        println!("  {} - Copy session ID", "[I]".yellow());
        println!("  {} - Copy project path", "[P]".yellow());
        println!("  {} - Continue", "[Any other key]".yellow());
        
        Ok(())
    }
    
    fn view_session(&self, result: &SearchResult, term: &Term) -> Result<()> {
        use std::fs::File;
        use std::io::{BufRead, BufReader};
        
        term.clear_screen()?;
        println!("{}", "Session Viewer".cyan());
        println!("{} {}", "Session:".yellow(), result.session_id);
        println!("{} {}", "File:".yellow(), result.file);
        println!();
        println!("{}", "[A]scending / [D]escending / [Q]uit".dimmed());
        println!();
        
        // Read order preference
        let ascending = match term.read_key()? {
            Key::Char('a') | Key::Char('A') => true,
            Key::Char('d') | Key::Char('D') => false,
            _ => return Ok(()),
        };
        
        // Read all messages from the file
        let file = File::open(&result.file)?;
        let reader = BufReader::new(file);
        let mut messages = Vec::new();
        
        #[allow(clippy::manual_flatten)]
        for line in reader.lines() {
            if let Ok(line) = line {
                if !line.trim().is_empty() {
                    messages.push(line);
                }
            }
        }
        
        // Sort messages
        if !ascending {
            messages.reverse();
        }
        
        // Display messages
        let separator = "─".repeat(80);
        for (idx, msg_line) in messages.iter().enumerate() {
            // Try to parse and display nicely
            if let Ok(msg) = serde_json::from_str::<serde_json::Value>(msg_line) {
                println!("{}", separator.dimmed());
                println!("{} {}/{}", "Message".cyan(), idx + 1, messages.len());
                
                if let Some(role) = msg.get("type").and_then(|v| v.as_str()) {
                    println!("{} {}", "Role:".yellow(), role);
                }
                if let Some(ts) = msg.get("timestamp").and_then(|v| v.as_str()) {
                    println!("{} {}", "Time:".yellow(), ts);
                }
                if let Some(content) = msg.get("content") {
                    if let Some(text) = content.as_str() {
                        println!("\n{text}");
                    } else if let Some(parts) = content.as_array() {
                        for part in parts {
                            if let Some(text) = part.get("text").and_then(|v| v.as_str()) {
                                println!("\n{text}");
                            }
                        }
                    }
                }
                
                // Pause every 3 messages
                if (idx + 1) % 3 == 0 && idx < messages.len() - 1 {
                    println!("\n{}", "Press any key to continue, Q to quit...".dimmed());
                    if let Key::Char('q') | Key::Char('Q') = term.read_key()? {
                        break;
                    }
                }
            }
        }
        
        println!("\n{}", "Press any key to return...".dimmed());
        term.read_key()?;
        term.clear_screen()?;
        
        Ok(())
    }
    
    fn copy_to_clipboard(&self, text: &str) -> Result<()> {
        use std::process::Command;
        
        #[cfg(target_os = "macos")]
        {
            let mut child = Command::new("pbcopy")
                .stdin(std::process::Stdio::piped())
                .spawn()?;
            
            if let Some(stdin) = child.stdin.as_mut() {
                use std::io::Write;
                stdin.write_all(text.as_bytes())?;
            }
            
            child.wait()?;
        }
        
        #[cfg(target_os = "linux")]
        {
            // Try xclip first, then xsel
            let result = Command::new("xclip")
                .arg("-selection")
                .arg("clipboard")
                .stdin(std::process::Stdio::piped())
                .spawn();
                
            match result {
                Ok(mut child) => {
                    if let Some(stdin) = child.stdin.as_mut() {
                        use std::io::Write;
                        stdin.write_all(text.as_bytes())?;
                    }
                    child.wait()?;
                }
                Err(_) => {
                    // Fallback to xsel
                    let mut child = Command::new("xsel")
                        .arg("--clipboard")
                        .arg("--input")
                        .stdin(std::process::Stdio::piped())
                        .spawn()?;
                    
                    if let Some(stdin) = child.stdin.as_mut() {
                        use std::io::Write;
                        stdin.write_all(text.as_bytes())?;
                    }
                    
                    child.wait()?;
                }
            }
        }
        
        #[cfg(target_os = "windows")]
        {
            let mut child = Command::new("clip")
                .stdin(std::process::Stdio::piped())
                .spawn()?;
            
            if let Some(stdin) = child.stdin.as_mut() {
                use std::io::Write;
                stdin.write_all(text.as_bytes())?;
            }
            
            child.wait()?;
        }
        
        Ok(())
    }
}