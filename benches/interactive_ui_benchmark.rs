use ccms::interactive_ratatui::ui::{
    app_state::AppState,
    components::{
        Component, result_list::ResultList,
        search_bar::SearchBar, session_viewer::SessionViewer,
    },
    events::Message,
    renderer::Renderer,
};
use ccms::{SessionMessage, SearchResult, SearchOptions, QueryCondition};
use ccms::schemas::{BaseMessage, UserMessageContent, UserContent};
use codspeed_criterion_compat::{Criterion, black_box, criterion_group, criterion_main, BatchSize};
use ratatui::{
    backend::TestBackend,
    layout::Rect,
    Terminal,
};

fn create_test_search_results(count: usize) -> Vec<SearchResult> {
    (0..count)
        .map(|i| {
            let content = if i % 10 == 0 {
                format!("日本語メッセージ {} 🦀 絵文字入り テスト内容", i)
            } else if i % 5 == 0 {
                format!("Very long message content that should be truncated properly when displayed in the UI. This is message number {}. Lorem ipsum dolor sit amet, consectetur adipiscing elit.", i)
            } else {
                format!("Test message {} with normal content", i)
            };
            
            SearchResult {
                file: format!("/path/to/file{}.jsonl", i % 3),
                uuid: format!("uuid-{}", i),
                timestamp: "2024-01-01T00:00:00Z".to_string(),
                session_id: format!("session-{}", i % 10),
                role: "user".to_string(),
                text: content,
                has_tools: false,
                has_thinking: false,
                message_type: "user".to_string(),
                query: QueryCondition::Literal { 
                    pattern: "test".to_string(), 
                    case_sensitive: false 
                },
                project_path: "/test".to_string(),
                raw_json: None,
            }
        })
        .collect()
}

fn benchmark_search_bar_rendering(c: &mut Criterion) {
    let mut group = c.benchmark_group("search_bar");
    let test_area = Rect::new(0, 0, 80, 3);
    
    // 基本レンダリング
    group.bench_function("render_basic", |b| {
        let mut search_bar = SearchBar::new();
        search_bar.set_query("test query".to_string());
        
        b.iter_batched(
            || TestBackend::new(80, 24),
            |backend| {
                let mut terminal = Terminal::new(backend).unwrap();
                terminal.draw(|f| {
                    search_bar.render(f, test_area);
                }).unwrap();
            },
            BatchSize::SmallInput,
        );
    });
    
    // 検索中状態のレンダリング
    group.bench_function("render_searching", |b| {
        let mut search_bar = SearchBar::new();
        search_bar.set_query("complex AND query OR test".to_string());
        search_bar.set_searching(true);
        search_bar.set_message(Some("Searching...".to_string()));
        
        b.iter_batched(
            || TestBackend::new(80, 24),
            |backend| {
                let mut terminal = Terminal::new(backend).unwrap();
                terminal.draw(|f| {
                    search_bar.render(f, test_area);
                }).unwrap();
            },
            BatchSize::SmallInput,
        );
    });
    
    // 日本語クエリのレンダリング
    group.bench_function("render_japanese", |b| {
        let mut search_bar = SearchBar::new();
        search_bar.set_query("日本語のクエリ 🦀 絵文字入り".to_string());
        
        b.iter_batched(
            || TestBackend::new(80, 24),
            |backend| {
                let mut terminal = Terminal::new(backend).unwrap();
                terminal.draw(|f| {
                    search_bar.render(f, test_area);
                }).unwrap();
            },
            BatchSize::SmallInput,
        );
    });
    
    group.finish();
}

fn benchmark_result_list_rendering(c: &mut Criterion) {
    let mut group = c.benchmark_group("result_list");
    let test_area = Rect::new(0, 0, 80, 20);
    
    // 少量の結果
    group.bench_function("render_10_results", |b| {
        let mut result_list = ResultList::new();
        result_list.set_results(create_test_search_results(10));
        result_list.set_selected_index(5);
        
        b.iter_batched(
            || TestBackend::new(80, 24),
            |backend| {
                let mut terminal = Terminal::new(backend).unwrap();
                terminal.draw(|f| {
                    result_list.render(f, test_area);
                }).unwrap();
            },
            BatchSize::SmallInput,
        );
    });
    
    // 大量の結果
    group.bench_function("render_1000_results", |b| {
        let mut result_list = ResultList::new();
        result_list.set_results(create_test_search_results(1000));
        result_list.set_selected_index(500);
        
        b.iter_batched(
            || TestBackend::new(80, 24),
            |backend| {
                let mut terminal = Terminal::new(backend).unwrap();
                terminal.draw(|f| {
                    result_list.render(f, test_area);
                }).unwrap();
            },
            BatchSize::SmallInput,
        );
    });
    
    // トランケーション有効時
    group.bench_function("render_truncated", |b| {
        let mut result_list = ResultList::new();
        result_list.set_results(create_test_search_results(100));
        result_list.set_truncation_enabled(true);
        result_list.set_selected_index(50);
        
        b.iter_batched(
            || TestBackend::new(80, 24),
            |backend| {
                let mut terminal = Terminal::new(backend).unwrap();
                terminal.draw(|f| {
                    result_list.render(f, test_area);
                }).unwrap();
            },
            BatchSize::SmallInput,
        );
    });
    
    group.finish();
}

fn benchmark_app_state_updates(c: &mut Criterion) {
    let mut group = c.benchmark_group("app_state");
    
    // クエリ変更の処理
    group.bench_function("update_query_changed", |b| {
        b.iter_batched(
            || {
                let mut state = AppState::new(SearchOptions::default(), 1000);
                state.search.results = create_test_search_results(100);
                state
            },
            |mut state| {
                let msg = Message::QueryChanged("new query".to_string());
                black_box(state.update(msg));
            },
            BatchSize::SmallInput,
        );
    });
    
    // 検索結果の処理
    group.bench_function("update_search_completed", |b| {
        let results = create_test_search_results(1000);
        
        b.iter_batched(
            || AppState::new(SearchOptions::default(), 1000),
            |mut state| {
                let msg = Message::SearchCompleted(results.clone());
                black_box(state.update(msg));
            },
            BatchSize::SmallInput,
        );
    });
    
    // 選択変更の処理
    group.bench_function("update_move_down", |b| {
        b.iter_batched(
            || {
                let mut state = AppState::new(SearchOptions::default(), 1000);
                state.search.results = create_test_search_results(1000);
                state.search.selected_index = 500;
                state
            },
            |mut state| {
                let msg = Message::ScrollDown;
                black_box(state.update(msg));
            },
            BatchSize::SmallInput,
        );
    });
    
    // モード切り替えの処理
    group.bench_function("update_enter_detail", |b| {
        b.iter_batched(
            || {
                let mut state = AppState::new(SearchOptions::default(), 1000);
                state.search.results = create_test_search_results(100);
                state.search.selected_index = 50;
                state
            },
            |mut state| {
                let msg = Message::EnterResultDetail;
                black_box(state.update(msg));
            },
            BatchSize::SmallInput,
        );
    });
    
    group.finish();
}

fn benchmark_full_frame_rendering(c: &mut Criterion) {
    let mut group = c.benchmark_group("full_frame");
    
    // 検索モードでのフルレンダリング
    group.bench_function("render_search_mode", |b| {
        let mut renderer = Renderer::new();
        let mut state = AppState::new(SearchOptions::default(), 1000);
        state.search.results = create_test_search_results(100);
        state.search.selected_index = 50;
        state.search.is_searching = false;
        state.search.query = "test query".to_string();
        
        b.iter_batched(
            || TestBackend::new(120, 40),
            |backend| {
                let mut terminal = Terminal::new(backend).unwrap();
                terminal.draw(|f| {
                    renderer.render(f, &state);
                }).unwrap();
            },
            BatchSize::SmallInput,
        );
    });
    
    // 詳細モードでのフルレンダリング
    group.bench_function("render_detail_mode", |b| {
        let mut renderer = Renderer::new();
        let mut state = AppState::new(SearchOptions::default(), 1000);
        let test_results = create_test_search_results(10);
        state.ui.selected_result = Some(test_results[0].clone());
        state.mode = ccms::interactive_ratatui::ui::app_state::Mode::ResultDetail;
        
        b.iter_batched(
            || TestBackend::new(120, 40),
            |backend| {
                let mut terminal = Terminal::new(backend).unwrap();
                terminal.draw(|f| {
                    renderer.render(f, &state);
                }).unwrap();
            },
            BatchSize::SmallInput,
        );
    });
    
    // リアルタイムタイピングシミュレーション
    group.bench_function("render_typing_simulation", |b| {
        let queries = vec![
            "t",
            "te",
            "tes",
            "test",
            "test ",
            "test q",
            "test qu",
            "test que",
            "test quer",
            "test query",
        ];
        
        b.iter_batched(
            || {
                let renderer = Renderer::new();
                let state = AppState::new(SearchOptions::default(), 1000);
                (renderer, state, TestBackend::new(120, 40))
            },
            |(mut renderer, mut state, backend)| {
                let mut terminal = Terminal::new(backend).unwrap();
                
                // タイピングをシミュレート
                for query in &queries {
                    state.update(Message::QueryChanged(query.to_string()));
                    terminal.draw(|f| {
                        renderer.render(f, &state);
                    }).unwrap();
                }
            },
            BatchSize::SmallInput,
        );
    });
    
    group.finish();
}

fn create_test_session_messages(count: usize) -> Vec<SessionMessage> {
    (0..count)
        .map(|i| {
            let content = if i % 10 == 0 {
                format!("日本語メッセージ {} 🦀 絵文字入り テスト内容", i)
            } else if i % 5 == 0 {
                format!("Very long message content that should be truncated properly when displayed in the UI. This is message number {}. Lorem ipsum dolor sit amet, consectetur adipiscing elit.", i)
            } else {
                format!("Test message {} with normal content", i)
            };
            
            SessionMessage::User {
                base: BaseMessage {
                    parent_uuid: None,
                    is_sidechain: false,
                    user_type: "external".to_string(),
                    cwd: "/test".to_string(),
                    session_id: format!("session-{}", i % 10),
                    version: "1.0".to_string(),
                    uuid: format!("uuid-{}", i),
                    timestamp: "2024-01-01T00:00:00Z".to_string(),
                },
                message: UserMessageContent {
                    role: "user".to_string(),
                    content: UserContent::String(content),
                },
                git_branch: None,
                is_meta: None,
                is_compact_summary: None,
                tool_use_result: None,
            }
        })
        .collect()
}

fn benchmark_session_viewer_rendering(c: &mut Criterion) {
    let mut group = c.benchmark_group("session_viewer");
    
    // セッションビューアのレンダリング
    group.bench_function("render_session", |b| {
        let mut session_viewer = SessionViewer::new();
        let messages = create_test_session_messages(200);
        let message_strings: Vec<String> = messages
            .iter()
            .map(|msg| serde_json::to_string(msg).unwrap_or_default())
            .collect();
        session_viewer.set_messages(message_strings);
        session_viewer.set_filtered_indices(vec![0, 10, 20, 30, 40, 50]);
        
        b.iter_batched(
            || TestBackend::new(120, 40),
            |backend| {
                let mut terminal = Terminal::new(backend).unwrap();
                terminal.draw(|f| {
                    session_viewer.render(f, f.area());
                }).unwrap();
            },
            BatchSize::SmallInput,
        );
    });
    
    group.finish();
}

fn benchmark_component_input_handling(c: &mut Criterion) {
    let mut group = c.benchmark_group("input_handling");
    
    // SearchBarのキー入力処理
    group.bench_function("search_bar_key_handling", |b| {
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
        
        let key_events = vec![
            KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE),
            KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE),
            KeyEvent::new(KeyCode::Left, KeyModifiers::NONE),
            KeyEvent::new(KeyCode::Right, KeyModifiers::NONE),
            KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE),
        ];
        
        b.iter_batched(
            || {
                let mut search_bar = SearchBar::new();
                search_bar.set_query("test query".to_string());
                search_bar
            },
            |mut search_bar| {
                for key in &key_events {
                    black_box(search_bar.handle_key(*key));
                }
            },
            BatchSize::SmallInput,
        );
    });
    
    group.finish();
}

criterion_group!(
    benches,
    benchmark_search_bar_rendering,
    benchmark_result_list_rendering,
    benchmark_app_state_updates,
    benchmark_full_frame_rendering,
    benchmark_session_viewer_rendering,
    benchmark_component_input_handling
);
criterion_main!(benches);