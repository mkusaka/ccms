use std::collections::{HashMap, HashSet};

#[derive(Debug, Default)]
pub struct Statistics {
    pub total_messages: usize,
    pub role_counts: HashMap<String, usize>,
    pub session_count: usize,
    pub unique_sessions: HashSet<String>,
    pub file_count: usize,
    pub unique_files: HashSet<String>,
    pub timestamp_range: Option<(String, String)>, // (earliest, latest)
    pub project_count: usize,
    pub unique_projects: HashSet<String>,
    pub message_type_counts: HashMap<String, usize>,
}

impl Statistics {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_message(
        &mut self,
        role: &str,
        session_id: &str,
        file: &str,
        timestamp: &str,
        cwd: &str,
        message_type: &str,
    ) {
        self.total_messages += 1;

        // Count by role
        *self.role_counts.entry(role.to_string()).or_insert(0) += 1;

        // Count by message type
        *self
            .message_type_counts
            .entry(message_type.to_string())
            .or_insert(0) += 1;

        // Track unique sessions
        if self.unique_sessions.insert(session_id.to_string()) {
            self.session_count += 1;
        }

        // Track unique files
        if self.unique_files.insert(file.to_string()) {
            self.file_count += 1;
        }

        // Track unique projects
        if self.unique_projects.insert(cwd.to_string()) {
            self.project_count += 1;
        }

        // Update timestamp range
        match &mut self.timestamp_range {
            None => {
                self.timestamp_range = Some((timestamp.to_string(), timestamp.to_string()));
            }
            Some((earliest, latest)) => {
                if timestamp < earliest.as_str() {
                    *earliest = timestamp.to_string();
                }
                if timestamp > latest.as_str() {
                    *latest = timestamp.to_string();
                }
            }
        }
    }
}

pub fn format_statistics(stats: &Statistics, use_color: bool) -> String {
    use colored::Colorize;

    let mut output = String::new();

    if use_color {
        output.push_str(&"Statistics".bright_blue().bold().to_string());
        output.push('\n');
        output.push_str(&"═".repeat(60).bright_blue().to_string());
        output.push_str("\n\n");

        // Total messages
        output.push_str(&format!(
            "{}: {}\n",
            "Total Messages".bright_yellow(),
            stats.total_messages.to_string().bright_green()
        ));

        // Sessions
        output.push_str(&format!(
            "{}: {}\n",
            "Sessions".bright_yellow(),
            stats.session_count.to_string().bright_green()
        ));

        // Files
        output.push_str(&format!(
            "{}: {}\n",
            "Files".bright_yellow(),
            stats.file_count.to_string().bright_green()
        ));

        // Projects
        if stats.project_count > 0 {
            output.push_str(&format!(
                "{}: {}\n",
                "Projects".bright_yellow(),
                stats.project_count.to_string().bright_green()
            ));
        }

        // Message types
        if !stats.message_type_counts.is_empty() {
            output.push_str(&format!("\n{}\n", "Message Types".bright_yellow().bold()));
            output.push_str(&"─".repeat(30).bright_blue().to_string());
            output.push('\n');

            let mut types: Vec<_> = stats.message_type_counts.iter().collect();
            types.sort_by_key(|(_, count)| std::cmp::Reverse(**count));

            for (msg_type, count) in types {
                output.push_str(&format!(
                    "  {}: {}\n",
                    msg_type.bright_cyan(),
                    count.to_string().bright_white()
                ));
            }
        }

        // Roles breakdown
        if !stats.role_counts.is_empty() {
            output.push_str(&format!(
                "\n{}\n",
                "Messages by Role".bright_yellow().bold()
            ));
            output.push_str(&"─".repeat(30).bright_blue().to_string());
            output.push('\n');

            let mut roles: Vec<_> = stats.role_counts.iter().collect();
            roles.sort_by_key(|(_, count)| std::cmp::Reverse(**count));

            for (role, count) in roles {
                let percentage = (*count as f64 / stats.total_messages as f64 * 100.0) as u32;
                output.push_str(&format!(
                    "  {}: {} ({}%)\n",
                    role.bright_cyan(),
                    count.to_string().bright_white(),
                    percentage.to_string().dimmed()
                ));
            }
        }

        // Time range
        if let Some((earliest, latest)) = &stats.timestamp_range {
            output.push_str(&format!("\n{}\n", "Time Range".bright_yellow().bold()));
            output.push_str(&"─".repeat(30).bright_blue().to_string());
            output.push('\n');

            // Try to parse and format timestamps
            let earliest_formatted = format_timestamp(earliest);
            let latest_formatted = format_timestamp(latest);

            output.push_str(&format!(
                "  {}: {}\n",
                "Earliest".bright_cyan(),
                earliest_formatted.bright_white()
            ));
            output.push_str(&format!(
                "  {}: {}\n",
                "Latest".bright_cyan(),
                latest_formatted.bright_white()
            ));
        }
    } else {
        output.push_str("Statistics\n");
        output.push_str(&"=".repeat(60));
        output.push_str("\n\n");

        output.push_str(&format!("Total Messages: {}\n", stats.total_messages));
        output.push_str(&format!("Sessions: {}\n", stats.session_count));
        output.push_str(&format!("Files: {}\n", stats.file_count));

        if stats.project_count > 0 {
            output.push_str(&format!("Projects: {}\n", stats.project_count));
        }

        if !stats.message_type_counts.is_empty() {
            output.push_str("\nMessage Types\n");
            output.push_str(&"-".repeat(30));
            output.push('\n');

            let mut types: Vec<_> = stats.message_type_counts.iter().collect();
            types.sort_by_key(|(_, count)| std::cmp::Reverse(**count));

            for (msg_type, count) in types {
                output.push_str(&format!("  {msg_type}: {count}\n"));
            }
        }

        if !stats.role_counts.is_empty() {
            output.push_str("\nMessages by Role\n");
            output.push_str(&"-".repeat(30));
            output.push('\n');

            let mut roles: Vec<_> = stats.role_counts.iter().collect();
            roles.sort_by_key(|(_, count)| std::cmp::Reverse(**count));

            for (role, count) in roles {
                let percentage = (*count as f64 / stats.total_messages as f64 * 100.0) as u32;
                output.push_str(&format!("  {role}: {count} ({percentage}%)\n"));
            }
        }

        if let Some((earliest, latest)) = &stats.timestamp_range {
            output.push_str("\nTime Range\n");
            output.push_str(&"-".repeat(30));
            output.push('\n');

            let earliest_formatted = format_timestamp(earliest);
            let latest_formatted = format_timestamp(latest);

            output.push_str(&format!("  Earliest: {earliest_formatted}\n"));
            output.push_str(&format!("  Latest: {latest_formatted}\n"));
        }
    }

    output
}

fn format_timestamp(timestamp: &str) -> String {
    use chrono::{DateTime, Local, TimeZone};

    if let Ok(dt) = DateTime::parse_from_rfc3339(timestamp) {
        let local_dt = Local.from_utc_datetime(&dt.naive_utc());
        local_dt.format("%Y-%m-%d %H:%M:%S").to_string()
    } else {
        timestamp.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_statistics_new() {
        let stats = Statistics::new();
        assert_eq!(stats.total_messages, 0);
        assert_eq!(stats.session_count, 0);
        assert_eq!(stats.file_count, 0);
        assert_eq!(stats.project_count, 0);
        assert!(stats.role_counts.is_empty());
        assert!(stats.unique_sessions.is_empty());
        assert!(stats.unique_files.is_empty());
        assert!(stats.unique_projects.is_empty());
        assert!(stats.message_type_counts.is_empty());
        assert!(stats.timestamp_range.is_none());
    }

    #[test]
    fn test_add_message() {
        let mut stats = Statistics::new();

        stats.add_message(
            "user",
            "session1",
            "file1.jsonl",
            "2024-01-01T00:00:00Z",
            "/project1",
            "message",
        );

        assert_eq!(stats.total_messages, 1);
        assert_eq!(stats.session_count, 1);
        assert_eq!(stats.file_count, 1);
        assert_eq!(stats.project_count, 1);
        assert_eq!(stats.role_counts.get("user"), Some(&1));
        assert_eq!(stats.message_type_counts.get("message"), Some(&1));
        assert_eq!(
            stats.timestamp_range,
            Some((
                "2024-01-01T00:00:00Z".to_string(),
                "2024-01-01T00:00:00Z".to_string()
            ))
        );

        // Add another message from same session/file
        stats.add_message(
            "assistant",
            "session1",
            "file1.jsonl",
            "2024-01-01T01:00:00Z",
            "/project1",
            "message",
        );

        assert_eq!(stats.total_messages, 2);
        assert_eq!(stats.session_count, 1); // Still 1 unique session
        assert_eq!(stats.file_count, 1); // Still 1 unique file
        assert_eq!(stats.project_count, 1); // Still 1 unique project
        assert_eq!(stats.role_counts.get("assistant"), Some(&1));
        assert_eq!(stats.message_type_counts.get("message"), Some(&2));

        // Timestamp range should be updated
        if let Some((earliest, latest)) = &stats.timestamp_range {
            assert_eq!(earliest, "2024-01-01T00:00:00Z");
            assert_eq!(latest, "2024-01-01T01:00:00Z");
        }
    }

    #[test]
    fn test_timestamp_range_ordering() {
        let mut stats = Statistics::new();

        // Add messages in reverse chronological order
        stats.add_message(
            "user",
            "session1",
            "file1.jsonl",
            "2024-01-03T00:00:00Z",
            "/project1",
            "message",
        );
        stats.add_message(
            "user",
            "session1",
            "file1.jsonl",
            "2024-01-01T00:00:00Z",
            "/project1",
            "message",
        );
        stats.add_message(
            "user",
            "session1",
            "file1.jsonl",
            "2024-01-02T00:00:00Z",
            "/project1",
            "message",
        );

        // Should still track earliest and latest correctly
        if let Some((earliest, latest)) = &stats.timestamp_range {
            assert_eq!(earliest, "2024-01-01T00:00:00Z");
            assert_eq!(latest, "2024-01-03T00:00:00Z");
        }
    }

    #[test]
    fn test_format_statistics_without_color() {
        let mut stats = Statistics::new();
        stats.add_message(
            "user",
            "session1",
            "file1.jsonl",
            "2024-01-01T00:00:00Z",
            "/project1",
            "message",
        );
        stats.add_message(
            "assistant",
            "session1",
            "file1.jsonl",
            "2024-01-01T01:00:00Z",
            "/project1",
            "message",
        );

        let output = format_statistics(&stats, false);

        assert!(output.contains("Statistics"));
        assert!(output.contains("Total Messages: 2"));
        assert!(output.contains("Sessions: 1"));
        assert!(output.contains("Files: 1"));
        assert!(output.contains("Projects: 1"));
        assert!(output.contains("Messages by Role"));
        assert!(output.contains("user: 1 (50%)"));
        assert!(output.contains("assistant: 1 (50%)"));
        assert!(output.contains("Time Range"));
    }

    #[test]
    fn test_format_statistics_with_multiple_message_types() {
        let mut stats = Statistics::new();
        stats.add_message(
            "user",
            "session1",
            "file1.jsonl",
            "2024-01-01T00:00:00Z",
            "/project1",
            "message",
        );
        stats.add_message(
            "system",
            "session1",
            "file1.jsonl",
            "2024-01-01T00:00:00Z",
            "/project1",
            "system",
        );
        stats.add_message(
            "user",
            "session1",
            "file1.jsonl",
            "2024-01-01T00:00:00Z",
            "/project1",
            "summary",
        );

        let output = format_statistics(&stats, false);

        assert!(output.contains("Message Types"));
        assert!(output.contains("message: 1"));
        assert!(output.contains("system: 1"));
        assert!(output.contains("summary: 1"));
    }
}
