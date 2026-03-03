use crate::schemas::SessionMessage;
use crate::search::discover_claude_files;
use anyhow::{Context, Result, bail};
use chrono::{DateTime, Datelike, Utc};
use serde_json::{Value, json};
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConvertMode {
    WriteFile,
    Stdout,
    DryRun,
}

#[derive(Debug, Clone)]
pub struct ConvertRequest {
    pub session_id: String,
    pub source_file_hint: Option<PathBuf>,
    pub codex_home: Option<PathBuf>,
    pub mode: ConvertMode,
}

impl ConvertRequest {
    pub fn new(session_id: impl Into<String>) -> Self {
        Self {
            session_id: session_id.into(),
            source_file_hint: None,
            codex_home: None,
            mode: ConvertMode::WriteFile,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ConvertResult {
    pub source_file: PathBuf,
    pub output_path: PathBuf,
    pub codex_session_id: String,
    pub converted_messages: usize,
    pub skipped_summaries: usize,
    pub skipped_invalid_lines: usize,
    pub rollout_jsonl: Option<String>,
}

#[derive(Debug, Clone)]
struct RawResponseItem {
    timestamp: String,
    role: String,
    text: String,
}

#[derive(Debug, Clone)]
struct RolloutBuild {
    rollout_jsonl: String,
    converted_messages: usize,
    skipped_summaries: usize,
    skipped_invalid_lines: usize,
    timestamp: DateTime<Utc>,
}

pub fn convert_session_to_codex(request: &ConvertRequest) -> Result<ConvertResult> {
    if request.session_id.trim().is_empty() {
        bail!("session_id is required");
    }

    let source_file =
        resolve_source_file(&request.session_id, request.source_file_hint.as_deref())?;
    let codex_session_id = normalize_or_generate_session_id(&request.session_id);

    let build = build_rollout_jsonl(&source_file, &request.session_id, &codex_session_id)?;
    let output_path = match request.mode {
        ConvertMode::WriteFile => {
            let codex_home = resolve_codex_home(request.codex_home.as_deref())?;
            build_output_path(&codex_home, build.timestamp, &codex_session_id)
        }
        ConvertMode::Stdout => PathBuf::from("<stdout>"),
        ConvertMode::DryRun => request
            .codex_home
            .as_deref()
            .map(|codex_home| build_output_path(codex_home, build.timestamp, &codex_session_id))
            .unwrap_or_else(|| PathBuf::from("<dry-run>")),
    };

    match request.mode {
        ConvertMode::WriteFile => {
            if let Some(parent) = output_path.parent() {
                fs::create_dir_all(parent).with_context(|| {
                    format!("failed to create output directory: {}", parent.display())
                })?;
            }
            fs::write(&output_path, build.rollout_jsonl.as_bytes()).with_context(|| {
                format!("failed to write rollout file: {}", output_path.display())
            })?;
        }
        ConvertMode::Stdout | ConvertMode::DryRun => {}
    }

    Ok(ConvertResult {
        source_file,
        output_path,
        codex_session_id,
        converted_messages: build.converted_messages,
        skipped_summaries: build.skipped_summaries,
        skipped_invalid_lines: build.skipped_invalid_lines,
        rollout_jsonl: matches!(request.mode, ConvertMode::Stdout).then_some(build.rollout_jsonl),
    })
}

fn resolve_source_file(session_id: &str, hint: Option<&Path>) -> Result<PathBuf> {
    if let Some(path) = hint {
        if !path.exists() {
            bail!("source file does not exist: {}", path.display());
        }
        if !file_contains_session_id(path, session_id)? {
            bail!(
                "source file does not contain session_id '{session_id}': {}",
                path.display()
            );
        }
        return Ok(path.to_path_buf());
    }

    let files = discover_claude_files(None).context("failed to discover Claude session files")?;
    let mut matches = Vec::new();

    for file in files {
        if file_contains_session_id(&file, session_id)? {
            matches.push(file);
        }
    }

    if matches.is_empty() {
        bail!("no Claude session file found for session_id '{session_id}'");
    }

    if matches.len() > 1 {
        let preview = matches
            .iter()
            .take(5)
            .map(|p| p.display().to_string())
            .collect::<Vec<_>>()
            .join(", ");
        bail!(
            "multiple Claude session files found for session_id '{session_id}': {}{}",
            preview,
            if matches.len() > 5 { ", ..." } else { "" }
        );
    }

    Ok(matches.remove(0))
}

fn file_contains_session_id(path: &Path, session_id: &str) -> Result<bool> {
    let file = fs::File::open(path).with_context(|| {
        format!(
            "failed to open file while resolving session_id: {}",
            path.display()
        )
    })?;
    let reader = BufReader::new(file);

    for line in reader.lines() {
        let line = line.with_context(|| format!("failed to read line from {}", path.display()))?;
        if line.trim().is_empty() {
            continue;
        }

        let value: Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(_) => continue,
        };

        if value.get("sessionId").and_then(Value::as_str) == Some(session_id) {
            return Ok(true);
        }
    }

    Ok(false)
}

fn normalize_or_generate_session_id(session_id: &str) -> String {
    let normalized_session_id = session_id.trim();

    if let Ok(parsed) = Uuid::parse_str(normalized_session_id) {
        return parsed.to_string();
    }

    let seed = format!("ccms:{normalized_session_id}");
    Uuid::new_v5(&Uuid::NAMESPACE_URL, seed.as_bytes()).to_string()
}

fn build_rollout_jsonl(
    source_file: &Path,
    session_id: &str,
    codex_session_id: &str,
) -> Result<RolloutBuild> {
    let file = fs::File::open(source_file)
        .with_context(|| format!("failed to open source file: {}", source_file.display()))?;
    let reader = BufReader::new(file);

    let mut responses: Vec<RawResponseItem> = Vec::new();
    let mut skipped_summaries = 0usize;
    let mut skipped_invalid_lines = 0usize;
    let mut first_message_timestamp: Option<DateTime<Utc>> = None;
    let mut cwd: Option<String> = None;

    for line in reader.lines() {
        let line =
            line.with_context(|| format!("failed to read line from {}", source_file.display()))?;
        if line.trim().is_empty() {
            continue;
        }

        let message: SessionMessage = match sonic_rs::from_str(&line) {
            Ok(message) => message,
            Err(_) => {
                skipped_invalid_lines += 1;
                continue;
            }
        };

        if message.get_type() == "summary" {
            skipped_summaries += 1;
            continue;
        }

        if message.get_session_id() != Some(session_id) {
            continue;
        }

        let role = message.get_type().to_string();
        if role != "user" && role != "assistant" && role != "system" {
            continue;
        }

        if cwd.is_none()
            && let Some(message_cwd) = message.get_cwd()
        {
            cwd = Some(message_cwd.to_string());
        }

        if first_message_timestamp.is_none() {
            first_message_timestamp = message.get_timestamp().and_then(parse_rfc3339_utc);
        }

        responses.push(RawResponseItem {
            timestamp: message.get_timestamp().unwrap_or_default().to_string(),
            role,
            text: message.get_content_text(),
        });
    }

    if responses.is_empty() {
        bail!(
            "no convertible messages found for session_id '{session_id}' in {}",
            source_file.display()
        );
    }

    let timestamp = first_message_timestamp.unwrap_or_else(|| fallback_timestamp(source_file));
    let timestamp_str = timestamp.to_rfc3339();

    let cwd = cwd
        .or_else(|| {
            std::env::current_dir()
                .ok()
                .map(|p| p.display().to_string())
        })
        .unwrap_or_else(|| "/".to_string());

    let session_meta_line = json!({
        "timestamp": timestamp_str,
        "type": "session_meta",
        "payload": {
            "id": codex_session_id,
            "timestamp": timestamp_str,
            "cwd": cwd,
            "originator": "ccms",
            "cli_version": env!("CARGO_PKG_VERSION"),
            "source": "cli"
        }
    });

    let mut lines = vec![
        serde_json::to_string(&session_meta_line)
            .context("failed to serialize session_meta rollout line")?,
    ];

    for response in responses {
        let content_type = if response.role == "assistant" {
            "output_text"
        } else {
            "input_text"
        };
        let response_role = response.role;

        let timestamp_for_line = if response.timestamp.is_empty() {
            timestamp_str.clone()
        } else {
            response.timestamp
        };

        let line = json!({
            "timestamp": &timestamp_for_line,
            "type": "response_item",
            "payload": {
                "type": "message",
                "role": &response_role,
                "content": [{
                    "type": content_type,
                    "text": response.text,
                }]
            }
        });

        lines.push(serde_json::to_string(&line).with_context(|| {
            format!(
                "failed to serialize response_item rollout line (role={response_role}, timestamp={timestamp_for_line})"
            )
        })?);
    }

    Ok(RolloutBuild {
        rollout_jsonl: lines.join("\n") + "\n",
        converted_messages: lines.len().saturating_sub(1),
        skipped_summaries,
        skipped_invalid_lines,
        timestamp,
    })
}

fn parse_rfc3339_utc(input: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(input)
        .ok()
        .map(|dt| dt.with_timezone(&Utc))
}

fn fallback_timestamp(source_file: &Path) -> DateTime<Utc> {
    fs::metadata(source_file)
        .ok()
        .and_then(|meta| meta.modified().ok())
        .map(DateTime::<Utc>::from)
        .unwrap_or_else(Utc::now)
}

fn resolve_codex_home(codex_home: Option<&Path>) -> Result<PathBuf> {
    if let Some(path) = codex_home {
        return Ok(path.to_path_buf());
    }

    if let Some(path) = std::env::var_os("CODEX_HOME")
        && !path.is_empty()
    {
        return Ok(PathBuf::from(path));
    }

    let home = dirs::home_dir().context("failed to resolve home directory for CODEX_HOME")?;
    Ok(home.join(".codex"))
}

fn build_output_path(codex_home: &Path, timestamp: DateTime<Utc>, session_id: &str) -> PathBuf {
    let sessions_dir = codex_home
        .join("sessions")
        .join(format!("{:04}", timestamp.year()))
        .join(format!("{:02}", timestamp.month()))
        .join(format!("{:02}", timestamp.day()));

    let filename_timestamp = timestamp.format("%Y-%m-%dT%H-%M-%S").to_string();
    sessions_dir.join(format!("rollout-{filename_timestamp}-{session_id}.jsonl"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn write_claude_jsonl(path: &Path, session_id: &str) {
        let lines = vec![
            json!({
                "type": "summary",
                "summary": "summary line",
                "leafUuid": "leaf-1"
            }),
            json!({
                "type": "user",
                "message": { "role": "user", "content": "hello" },
                "uuid": "u1",
                "timestamp": "2026-02-01T10:00:00Z",
                "sessionId": session_id,
                "parentUuid": Value::Null,
                "isSidechain": false,
                "userType": "external",
                "cwd": "/tmp/project",
                "version": "1.0"
            }),
            json!({
                "type": "assistant",
                "message": {
                    "id": "a1",
                    "type": "message",
                    "role": "assistant",
                    "model": "claude",
                    "content": [{"type": "text", "text": "hi"}],
                    "stop_reason": Value::Null,
                    "stop_sequence": Value::Null,
                    "usage": {
                        "input_tokens": 1,
                        "cache_creation_input_tokens": 0,
                        "cache_read_input_tokens": 0,
                        "output_tokens": 1
                    }
                },
                "uuid": "a1",
                "timestamp": "2026-02-01T10:00:01Z",
                "sessionId": session_id,
                "parentUuid": "u1",
                "isSidechain": false,
                "userType": "external",
                "cwd": "/tmp/project",
                "version": "1.0"
            }),
        ];

        let body = lines
            .into_iter()
            .map(|line| line.to_string())
            .collect::<Vec<_>>()
            .join("\n")
            + "\n";

        fs::write(path, body).unwrap();
    }

    #[test]
    fn test_convert_with_source_hint_writes_rollout() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("session.jsonl");
        write_claude_jsonl(&source, "session-abc");

        let codex_home = dir.path().join("codex_home");
        let mut request = ConvertRequest::new("session-abc");
        request.source_file_hint = Some(source.clone());
        request.codex_home = Some(codex_home.clone());
        request.mode = ConvertMode::WriteFile;

        let result = convert_session_to_codex(&request).unwrap();

        assert_eq!(result.source_file, source);
        assert_eq!(result.converted_messages, 2);
        assert_eq!(result.skipped_summaries, 1);
        assert!(result.output_path.exists());

        let content = fs::read_to_string(&result.output_path).unwrap();
        let first_line: Value = serde_json::from_str(content.lines().next().unwrap()).unwrap();
        assert_eq!(
            first_line.get("type").and_then(Value::as_str),
            Some("session_meta")
        );

        let payload = first_line.get("payload").unwrap();
        let codex_id = payload.get("id").and_then(Value::as_str).unwrap();
        assert!(Uuid::parse_str(codex_id).is_ok());
    }

    #[test]
    fn test_convert_with_invalid_source_hint_fails() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("session.jsonl");
        write_claude_jsonl(&source, "session-abc");

        let mut request = ConvertRequest::new("different-session");
        request.source_file_hint = Some(source);
        request.codex_home = Some(dir.path().join("codex_home"));

        let err = convert_session_to_codex(&request).unwrap_err();
        assert!(
            err.to_string()
                .contains("source file does not contain session_id")
        );
    }

    #[test]
    fn test_convert_generates_stable_id_across_source_files() {
        let dir = tempdir().unwrap();
        let source_a = dir.path().join("session_a.jsonl");
        let source_b = dir.path().join("session_b.jsonl");
        write_claude_jsonl(&source_a, "session-abc");
        write_claude_jsonl(&source_b, "session-abc");

        let mut request_a = ConvertRequest::new("session-abc");
        request_a.source_file_hint = Some(source_a);
        request_a.mode = ConvertMode::DryRun;

        let mut request_b = ConvertRequest::new("session-abc");
        request_b.source_file_hint = Some(source_b);
        request_b.mode = ConvertMode::DryRun;

        let result_a = convert_session_to_codex(&request_a).unwrap();
        let result_b = convert_session_to_codex(&request_b).unwrap();

        assert_eq!(result_a.codex_session_id, result_b.codex_session_id);
    }

    #[test]
    fn test_convert_stdout_does_not_require_codex_home() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("session.jsonl");
        write_claude_jsonl(&source, "session-abc");

        let mut request = ConvertRequest::new("session-abc");
        request.source_file_hint = Some(source);
        request.mode = ConvertMode::Stdout;

        let result = convert_session_to_codex(&request).unwrap();

        assert_eq!(result.output_path, PathBuf::from("<stdout>"));
        assert!(result.rollout_jsonl.is_some());
    }
}
