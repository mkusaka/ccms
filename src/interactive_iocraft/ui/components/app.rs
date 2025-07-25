//! Root application component

use crate::interactive_iocraft::application::{SearchService, SessionService, CacheService, SettingsService};
use crate::interactive_iocraft::domain::models::Mode;
use crate::interactive_iocraft::ui::components::{SearchView, DetailView, SessionView, HelpModal};
use crate::interactive_iocraft::ui::contexts::{Theme, Settings};
use crate::interactive_iocraft::ui::hooks::is_quit_key;
use crate::interactive_iocraft::SearchResult;
use iocraft::prelude::*;
use std::sync::{Arc, Mutex};

#[derive(Props)]
pub struct AppProps {
    pub search_service: Option<Arc<SearchService>>,
    pub session_service: Option<Arc<SessionService>>,
    pub cache_service: Option<Arc<Mutex<CacheService>>>,
    pub settings_service: Option<Arc<SettingsService>>,
    pub initial_query: Option<String>,
    pub file_patterns: Vec<String>,
    pub settings: Settings,
}

impl Default for AppProps {
    fn default() -> Self {
        Self {
            search_service: None,
            session_service: None,
            cache_service: None,
            settings_service: None,
            initial_query: None,
            file_patterns: vec![],
            settings: Settings::default(),
        }
    }
}

#[component]
pub fn App(mut hooks: Hooks, props: &AppProps) -> impl Into<AnyElement<'static>> {
    // Validate required services with helpful error messages
    let search_service = props.search_service.as_ref()
        .expect("App component requires search_service. Please ensure SearchService is initialized and passed to App props.");
    let session_service = props.session_service.as_ref()
        .expect("App component requires session_service. Please ensure SessionService is initialized and passed to App props.");
    let cache_service = props.cache_service.as_ref()
        .expect("App component requires cache_service. Please ensure CacheService is initialized and passed to App props.");
    
    // State
    let mut mode = hooks.use_state(|| Mode::Search);
    let mut mode_stack = hooks.use_state(|| Vec::<Mode>::new());
    let mut show_help = hooks.use_state(|| false);
    let mut current_result = hooks.use_state(|| None::<SearchResult>);
    let mut current_session_path = hooks.use_state(|| None::<String>);
    let mut quit_requested = hooks.use_state(|| false);
    let mut last_quit_time = hooks.use_state(|| std::time::Instant::now());
    let mut quit_message = hooks.use_state(|| None::<String>);
    
    // Check quit timeout
    if *quit_requested.read() {
        let elapsed = std::time::Instant::now().duration_since(*last_quit_time.read());
        if elapsed.as_millis() > 1000 {
            // Timeout expired, cancel quit request
            quit_requested.set(false);
            quit_message.set(None);
        }
    }
    
    // Handle global keyboard events
    hooks.use_terminal_events({
        let mut mode = mode.clone();
        let mut mode_stack = mode_stack.clone();
        let mut show_help = show_help.clone();
        let mut quit_requested = quit_requested.clone();
        let mut last_quit_time = last_quit_time.clone();
        let mut quit_message = quit_message.clone();
        
        move |event| {
            if let TerminalEvent::Key(key) = event {
                // Check for quit
                if is_quit_key(&key) {
                    let now = std::time::Instant::now();
                    if *quit_requested.read() && now.duration_since(*last_quit_time.read()).as_millis() < 1000 {
                        // Double Ctrl+C within 1 second, exit
                        std::process::exit(0);
                    } else {
                        // First Ctrl+C, request confirmation
                        quit_requested.set(true);
                        last_quit_time.set(now);
                        quit_message.set(Some("Press Ctrl+C again to exit".to_string()));
                    }
                } else {
                    // Any other key cancels quit request
                    quit_requested.set(false);
                    quit_message.set(None);
                }
                    
                    // Check for escape (go back)
                    if key.code == iocraft::KeyCode::Esc {
                        if *show_help.read() {
                            show_help.set(false);
                        } else {
                            let stack = mode_stack.read().clone();
                            if let Some(prev_mode) = stack.last() {
                                mode.set(prev_mode.clone());
                                let mut new_stack = stack;
                                new_stack.pop();
                                mode_stack.set(new_stack);
                            }
                        }
                    }
            }
        }
    });
    
    // Helper to create mode push handler
    let make_push_mode = |new_mode: Mode| {
        let mut mode = mode.clone();
        let mut mode_stack = mode_stack.clone();
        move || {
            let mut stack = mode_stack.read().clone();
            stack.push(mode.read().clone());
            mode_stack.set(stack);
            mode.set(new_mode);
        }
    };
    
    element! {
        ContextProvider(value: Context::owned(Theme::default())) {
            ContextProvider(value: Context::owned(props.settings.clone())) {
                ContextProvider(value: Context::owned(search_service.clone())) {
                    ContextProvider(value: Context::owned(session_service.clone())) {
                        ContextProvider(value: Context::owned(cache_service.clone())) {
                        Box(
                            width: 100pct,
                            height: 100pct,
                            background_color: Color::Reset,
                        ) {
            // Quit confirmation message overlay
            #(if let Some(msg) = quit_message.read().clone() {
                element! {
                    Box(
                        position: Position::Absolute,
                        bottom: 2,
                        left: 50pct,
                        margin_left: -15,
                        background_color: Color::Red,
                        padding_top: 1,
                        padding_bottom: 1,
                        padding_left: 2,
                        padding_right: 2,
                        border_style: BorderStyle::Round,
                        border_color: Color::White,
                    ) {
                        Text(
                            content: msg,
                            color: Color::White,
                            weight: Weight::Bold,
                        )
                    }
                }
            } else {
                element! { Box() }
            })
            
            // Main content based on current mode
            #(match mode.read().clone() {
                Mode::Search => {
                    let mut current_result = current_result.clone();
                    let mut show_help = show_help.clone();
                    let mut push_to_detail = make_push_mode(Mode::ResultDetail);
                    
                    element! {
                        SearchView(
                            initial_query: props.initial_query.clone(),
                            file_pattern: if props.file_patterns.is_empty() {
                                crate::interactive_iocraft::default_claude_pattern()
                            } else {
                                props.file_patterns.join(",")
                            },
                            on_select_result: move |result: SearchResult| {
                                current_result.set(Some(result.clone()));
                                push_to_detail();
                            },
                            on_show_help: move |_| {
                                show_help.set(true);
                            },
                        )
                    }.into_any()
                }
                Mode::ResultDetail => {
                    if let Some(result) = current_result.read().clone() {
                        let mut current_session_path = current_session_path.clone();
                        let mut push_to_session = make_push_mode(Mode::SessionViewer);
                        
                        element! {
                            DetailView(
                                result: result,
                                on_view_session: move |path: String| {
                                    current_session_path.set(Some(path.clone()));
                                    push_to_session();
                                },
                            )
                        }.into_any()
                    } else {
                        // Fallback to search if no result
                        element! {
                            Box(
                                align_items: AlignItems::Center,
                                justify_content: JustifyContent::Center,
                                width: 100pct,
                                height: 100pct,
                            ) {
                                Text(content: "No result selected".to_string())
                            }
                        }.into_any()
                    }
                }
                Mode::SessionViewer => {
                    if let Some(path) = current_session_path.read().clone() {
                        element! {
                            SessionView(
                                file_path: path,
                            )
                        }.into_any()
                    } else {
                        // Fallback to search if no session
                        element! {
                            Box(
                                align_items: AlignItems::Center,
                                justify_content: JustifyContent::Center,
                                width: 100pct,
                                height: 100pct,
                            ) {
                                Text(content: "No session selected".to_string())
                            }
                        }.into_any()
                    }
                }
                Mode::Help => {
                    // Help mode is not used in the main content area
                    element! {
                        Box()
                    }.into_any()
                }
            })
            
            // Help modal overlay
            #(if *show_help.read() {
                let mut show_help = show_help.clone();
                element! {
                    HelpModal(
                        on_close: move |_| show_help.set(false),
                    )
                }.into_any()
            } else {
                element! { Box() }.into_any()
            })
            
            // Quit confirmation message
            #(if *quit_requested.read() {
                element! {
                    Box(
                        position: Position::Absolute,
                        bottom: 1,
                        left: 1,
                        background_color: Color::Red,
                        padding_left: 1,
                        padding_right: 1,
                    ) {
                        Text(
                            content: "Press Ctrl+C again to exit".to_string(),
                            color: Color::White,
                            weight: Weight::Bold,
                        )
                    }
                }.into_any()
            } else {
                element! { Box() }.into_any()
            })
                            }
                        }
                    }
                }
            }
        }
    }
}