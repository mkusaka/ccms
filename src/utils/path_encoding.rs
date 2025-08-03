/// Encode a path to Claude Code's project directory format
/// Replaces path separators and special characters with hyphens
pub fn encode_project_path(path: &str) -> String {
    path.chars()
        .map(|c| match c {
            '/' => '-',
            '\\' => '-',
            ':' => '-',
            '*' => '-',
            '?' => '-',
            '"' => '-',
            '<' => '-',
            '>' => '-',
            '|' => '-',
            '.' => '-', // Claude Code also replaces dots
            '_' => '-', // Claude Code also replaces underscores
            _ => c,
        })
        .collect()
}

/// Extract project name from Claude Code file path
/// Example: /Users/me/.claude/projects/-Users-me-project/file.jsonl -> -Users-me-project
pub fn extract_project_from_file_path(file_path: &str) -> Option<String> {
    let parts: Vec<&str> = file_path.split("/.claude/projects/").collect();
    if parts.len() >= 2 {
        let project_part = parts[1];
        if let Some(slash_idx) = project_part.find('/') {
            Some(project_part[..slash_idx].to_string())
        } else {
            Some(project_part.to_string())
        }
    } else {
        None
    }
}

/// Check if a file path belongs to a specific project
pub fn file_belongs_to_project(file_path: &str, project_path: &str) -> bool {
    if let Some(extracted_project) = extract_project_from_file_path(file_path) {
        let encoded_project = encode_project_path(project_path);

        // Debug output
        if std::env::var("DEBUG_PATH_ENCODING").is_ok() {
            eprintln!("DEBUG PATH ENCODING:");
            eprintln!("  file_path: {file_path}");
            eprintln!("  project_path: {project_path}");
            eprintln!("  extracted_project: {extracted_project}");
            eprintln!("  encoded_project: {encoded_project}");
            eprintln!(
                "  starts_with result: {}",
                extracted_project.starts_with(&encoded_project)
            );
        }

        // Check if the extracted project starts with the encoded project path
        extracted_project.starts_with(&encoded_project)
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_project_path() {
        assert_eq!(
            encode_project_path("/Users/me/src/project"),
            "-Users-me-src-project"
        );
        assert_eq!(
            encode_project_path("/Users/me/src/github.com/org/repo"),
            "-Users-me-src-github-com-org-repo"
        );
        assert_eq!(
            encode_project_path("/Users/me/src/special:chars*test"),
            "-Users-me-src-special-chars-test"
        );
        assert_eq!(
            encode_project_path("/Users/me/src/test.project"),
            "-Users-me-src-test-project"
        );
        assert_eq!(
            encode_project_path("/Users/me/src/test_project"),
            "-Users-me-src-test-project"
        );
    }

    #[test]
    fn test_extract_project_from_file_path() {
        assert_eq!(
            extract_project_from_file_path(
                "/Users/me/.claude/projects/-Users-me-project/session.jsonl"
            ),
            Some("-Users-me-project".to_string())
        );
        assert_eq!(
            extract_project_from_file_path(
                "/Users/me/.claude/projects/-Users-me-src-github-com-org-repo/abc.jsonl"
            ),
            Some("-Users-me-src-github-com-org-repo".to_string())
        );
        assert_eq!(
            extract_project_from_file_path("/Users/me/other/path.jsonl"),
            None
        );
    }

    #[test]
    fn test_file_belongs_to_project() {
        assert!(file_belongs_to_project(
            "/Users/me/.claude/projects/-Users-me-src-project/session.jsonl",
            "/Users/me/src/project"
        ));
        assert!(file_belongs_to_project(
            "/Users/me/.claude/projects/-Users-me-src-project-subdir/session.jsonl",
            "/Users/me/src/project"
        ));
        assert!(!file_belongs_to_project(
            "/Users/me/.claude/projects/-Users-me-other-project/session.jsonl",
            "/Users/me/src/project"
        ));
    }
}
