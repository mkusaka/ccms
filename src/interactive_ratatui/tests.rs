use super::*;
use crate::SearchOptions;
use crate::interactive_ratatui::ui::app_state::{AppState, Mode};
use crate::interactive_ratatui::ui::commands::Command;
use crate::interactive_ratatui::ui::events::Message;
use crate::query::condition::SearchResult;

#[test]
fn test_interactive_search_creation() {
    let interactive = InteractiveSearch::new(SearchOptions::default());
    assert_eq!(interactive.pattern, "");
}

#[test]
fn test_app_state_creation() {
    let state = AppState::new();
    assert_eq!(state.mode, Mode::Search);
    assert_eq!(state.search.query, "");
    assert_eq!(state.search.results.len(), 0);
}

#[test]
fn test_message_handling() {
    let mut state = AppState::new();

    // Test query change
    let command = state.update(Message::QueryChanged("test query".to_string()));
    assert_eq!(state.search.query, "test query");
    assert!(matches!(command, Command::ScheduleSearch(_)));

    // Test mode change
    state.search.results = vec![SearchResult {
        file: "test.jsonl".to_string(),
        uuid: "test-uuid".to_string(),
        timestamp: "2024-01-01T00:00:00Z".to_string(),
        session_id: "test-session".to_string(),
        role: "user".to_string(),
        text: "test text".to_string(),
        has_tools: false,
        has_thinking: false,
        message_type: "user".to_string(),
        query: crate::query::condition::QueryCondition::Literal {
            pattern: "test".to_string(),
            case_sensitive: false,
        },
        project_path: "/test".to_string(),
        raw_json: None,
    }];

    let command = state.update(Message::EnterMessageDetail);
    assert_eq!(state.mode, Mode::MessageDetail);
    assert!(matches!(command, Command::None));
}

#[test]
fn test_search_filter() {
    use crate::interactive_ratatui::domain::filter::SearchFilter;
    use crate::query::condition::SearchResult;

    let mut results = vec![
        SearchResult {
            file: "test1.jsonl".to_string(),
            uuid: "uuid1".to_string(),
            timestamp: "2024-01-01T00:00:00Z".to_string(),
            session_id: "session1".to_string(),
            role: "user".to_string(),
            text: "user message".to_string(),
            has_tools: false,
            has_thinking: false,
            message_type: "user".to_string(),
            query: crate::query::condition::QueryCondition::Literal {
                pattern: "test".to_string(),
                case_sensitive: false,
            },
            project_path: "/test".to_string(),
            raw_json: None,
        },
        SearchResult {
            file: "test2.jsonl".to_string(),
            uuid: "uuid2".to_string(),
            timestamp: "2024-01-01T00:01:00Z".to_string(),
            session_id: "session1".to_string(),
            role: "assistant".to_string(),
            text: "assistant message".to_string(),
            has_tools: false,
            has_thinking: false,
            message_type: "assistant".to_string(),
            query: crate::query::condition::QueryCondition::Literal {
                pattern: "test".to_string(),
                case_sensitive: false,
            },
            project_path: "/test".to_string(),
            raw_json: None,
        },
    ];

    let filter = SearchFilter::new(Some("user".to_string()));
    filter.apply(&mut results).unwrap();

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].role, "user");
}
