use r3bl_tui::{
    App, GlobalData, ComponentRegistryMap, HasFocus, InputEvent, 
    KeyEvent, KeyCode, KeyModifiers, EventPropagation, CommonResult,
    RenderPipeline, FlexBoxId, surface, stylesheet, Column, BoxSize,
    Rect, async_trait,
};
use tokio::sync::mpsc;
use std::sync::Arc;

use crate::SearchOptions;
use super::{
    state::{AppState, AppSignal, ViewMode},
    components::{SearchView, ResultDetailView, SessionView, HelpView},
    search_service::SearchService,
};

pub struct SearchApp {
    pub options: SearchOptions,
    pub file_pattern: String,
    pub signal_sender: mpsc::Sender<AppSignal>,
    pub search_service: Arc<SearchService>,
}

impl SearchApp {
    pub fn new(options: SearchOptions, file_pattern: String, signal_sender: mpsc::Sender<AppSignal>) -> Self {
        let search_service = Arc::new(SearchService::new(options.clone(), file_pattern.clone()));
        Self {
            options,
            file_pattern,
            signal_sender,
            search_service,
        }
    }
}

#[async_trait]
impl App for SearchApp {
    type State = AppState;
    type Signal = AppSignal;

    async fn app_init(
        &mut self,
        component_registry_map: &mut ComponentRegistryMap<Self::State, Self::Signal>,
        has_focus: &mut HasFocus,
    ) -> CommonResult<()> {
        // Create main layout
        let main_id = FlexBoxId::from("main");
        has_focus.set_id(Some(main_id));
        
        // Register components
        component_registry_map.put(
            main_id,
            SearchView::new_boxed(self.search_service.clone(), self.signal_sender.clone()),
        )?;
        
        component_registry_map.put(
            FlexBoxId::from("result_detail"),
            ResultDetailView::new_boxed(),
        )?;
        
        component_registry_map.put(
            FlexBoxId::from("session_view"),
            SessionView::new_boxed(),
        )?;
        
        component_registry_map.put(
            FlexBoxId::from("help"),
            HelpView::new_boxed(),
        )?;
        
        // Trigger initial search if pattern is provided
        if !component_registry_map.state.read().unwrap().query.is_empty() {
            let query = component_registry_map.state.read().unwrap().query.clone();
            let search_service = self.search_service.clone();
            let signal_sender = self.signal_sender.clone();
            
            tokio::spawn(async move {
                let results = search_service.search(&query).await;
                let _ = signal_sender.send(AppSignal::SearchCompleted(results)).await;
            });
        }
        
        Ok(())
    }

    async fn app_handle_input_event(
        &mut self,
        input_event: InputEvent,
        global_data: &mut GlobalData<Self::State, Self::Signal>,
        component_registry_map: &mut ComponentRegistryMap<Self::State, Self::Signal>,
        has_focus: &mut HasFocus,
    ) -> CommonResult<EventPropagation> {
        // Global keyboard shortcuts
        if let InputEvent::Keyboard(key_event) = &input_event {
            match key_event {
                KeyEvent {
                    code: KeyCode::Char('c'),
                    modifiers,
                    ..
                } if modifiers.contains(KeyModifiers::CONTROL) => {
                    return Ok(EventPropagation::ConsumedRender);
                }
                KeyEvent {
                    code: KeyCode::Char('q'),
                    ..
                } => {
                    let _ = self.signal_sender.send(AppSignal::Quit).await;
                    return Ok(EventPropagation::ConsumedRender);
                }
                KeyEvent {
                    code: KeyCode::Char('?'),
                    ..
                } => {
                    let _ = self.signal_sender.send(AppSignal::ShowHelp).await;
                    return Ok(EventPropagation::ConsumedRender);
                }
                _ => {}
            }
        }
        
        // Route to focused component
        let state = global_data.state.read().unwrap();
        let focused_id = match state.mode {
            ViewMode::Search => FlexBoxId::from("main"),
            ViewMode::ResultDetail => FlexBoxId::from("result_detail"),
            ViewMode::SessionViewer => FlexBoxId::from("session_view"),
            ViewMode::Help => FlexBoxId::from("help"),
        };
        drop(state);
        
        has_focus.set_id(Some(focused_id));
        
        let result = component_registry_map.route_event_to_focused_component(
            &input_event,
            global_data,
            has_focus,
        )?;
        
        Ok(result)
    }

    async fn app_handle_signal(
        &mut self,
        signal: Self::Signal,
        global_data: &mut GlobalData<Self::State, Self::Signal>,
        _component_registry_map: &mut ComponentRegistryMap<Self::State, Self::Signal>,
        _has_focus: &mut HasFocus,
    ) -> CommonResult<EventPropagation> {
        let mut state = global_data.state.write().unwrap();
        
        match signal {
            AppSignal::SearchCompleted(results) => {
                state.search_results = results;
                state.selected_index = 0;
                state.is_searching = false;
            }
            AppSignal::UpdateQuery(query) => {
                state.query = query.clone();
                state.is_searching = true;
                
                // Trigger search
                let search_service = self.search_service.clone();
                let signal_sender = self.signal_sender.clone();
                
                tokio::spawn(async move {
                    let results = search_service.search(&query).await;
                    let _ = signal_sender.send(AppSignal::SearchCompleted(results)).await;
                });
            }
            AppSignal::NavigateUp => {
                if state.selected_index > 0 {
                    state.selected_index -= 1;
                }
            }
            AppSignal::NavigateDown => {
                if state.selected_index < state.search_results.len().saturating_sub(1) {
                    state.selected_index += 1;
                }
            }
            AppSignal::EnterResultDetail => {
                state.mode = ViewMode::ResultDetail;
                state.result_detail_scroll_offset = 0;
            }
            AppSignal::EnterSessionViewer => {
                if let Some(message) = state.get_selected_message() {
                    if let Some(path) = &message.session {
                        state.selected_session_path = Some(path.clone());
                        state.mode = ViewMode::SessionViewer;
                        state.session_scroll_offset = 0;
                        
                        // Load session
                        let path = path.clone();
                        let signal_sender = self.signal_sender.clone();
                        
                        tokio::spawn(async move {
                            let _ = signal_sender.send(AppSignal::LoadSession(path)).await;
                        });
                    }
                }
            }
            AppSignal::LoadSession(path) => {
                let messages = self.search_service.load_session(&path).await;
                let _ = self.signal_sender.send(AppSignal::SessionLoaded(messages)).await;
            }
            AppSignal::SessionLoaded(messages) => {
                state.session_messages = messages;
            }
            AppSignal::ExitCurrentView => {
                match state.mode {
                    ViewMode::ResultDetail | ViewMode::SessionViewer | ViewMode::Help => {
                        state.mode = ViewMode::Search;
                    }
                    ViewMode::Search => {
                        // Already at top level, could quit or do nothing
                    }
                }
            }
            AppSignal::ShowHelp => {
                state.mode = ViewMode::Help;
            }
            AppSignal::Quit => {
                global_data.set_has_shutdown(true);
            }
        }
        
        Ok(EventPropagation::ConsumedRender)
    }

    async fn app_render(
        &mut self,
        global_data: &mut GlobalData<Self::State, Self::Signal>,
        component_registry_map: &mut ComponentRegistryMap<Self::State, Self::Signal>,
        has_focus: &mut HasFocus,
    ) -> CommonResult<RenderPipeline> {
        let window_size = global_data.window_size;
        let state = global_data.state.read().unwrap();
        
        // Select component based on current mode
        let component_id = match state.mode {
            ViewMode::Search => FlexBoxId::from("main"),
            ViewMode::ResultDetail => FlexBoxId::from("result_detail"),
            ViewMode::SessionViewer => FlexBoxId::from("session_view"),
            ViewMode::Help => FlexBoxId::from("help"),
        };
        drop(state);
        
        // Create a full-screen flexbox for the current view
        let mut surface = surface!(
            stylesheet: stylesheet! {
                id: component_id
            },
            flex_direction: Column,
            box_size: BoxSize {
                width: window_size.col_count.into(),
                height: window_size.row_count.into(),
            }
        );
        
        surface.compute_layout(global_data)?;
        
        // Render the current component
        component_registry_map.render_in_surface(
            &component_id,
            &mut surface,
            global_data,
            has_focus,
        )?;
        
        Ok(surface.render_to_pipeline()?)
    }
}