use r3bl_tui::{
    Component, GlobalData, ComponentRegistryMap, HasFocus, InputEvent,
    KeyEvent, KeyCode, KeyModifiers, EventPropagation, CommonResult,
    RenderPipeline, FlexBoxId, Surface, render_ops, RenderOp, Position,
    ZOrder, BoxedSafeComponent, async_trait, Rect,
    DEFAULT_STYLE, DEFAULT_STYLE_BOLD, DEFAULT_STYLE_DIM, DEFAULT_STYLE_REVERSED,
};
use tokio::sync::mpsc;
use std::sync::Arc;

use crate::interactive_r3bl::{
    state::{AppState, AppSignal},
    search_service::SearchService,
};

pub struct SearchView {
    search_service: Arc<SearchService>,
    signal_sender: mpsc::Sender<AppSignal>,
    editor_id: FlexBoxId,
}

impl SearchView {
    pub fn new(
        search_service: Arc<SearchService>,
        signal_sender: mpsc::Sender<AppSignal>,
    ) -> Self {
        Self {
            search_service,
            signal_sender,
            editor_id: FlexBoxId::from("search_editor"),
        }
    }
    
    pub fn new_boxed(
        search_service: Arc<SearchService>,
        signal_sender: mpsc::Sender<AppSignal>,
    ) -> BoxedSafeComponent<AppState, AppSignal> {
        Box::new(Self::new(search_service, signal_sender))
    }
}

#[async_trait]
impl Component<AppState, AppSignal> for SearchView {
    fn reset(&mut self) {
        // Reset any component state if needed
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    async fn handle_event(
        &mut self,
        input_event: &InputEvent,
        global_data: &mut GlobalData<AppState, AppSignal>,
        _component_registry_map: &mut ComponentRegistryMap<AppState, AppSignal>,
        _has_focus: &mut HasFocus,
    ) -> CommonResult<EventPropagation> {
        if let InputEvent::Keyboard(key) = input_event {
            let state = global_data.state.read().unwrap();
            let signal_sender = self.signal_sender.clone();
            
            match key {
                KeyEvent {
                    code: KeyCode::Up | KeyCode::Char('k'),
                    modifiers,
                    ..
                } if !modifiers.contains(KeyModifiers::CONTROL) => {
                    drop(state);
                    let _ = signal_sender.send(AppSignal::NavigateUp).await;
                    return Ok(EventPropagation::ConsumedRender);
                }
                KeyEvent {
                    code: KeyCode::Down | KeyCode::Char('j'),
                    modifiers,
                    ..
                } if !modifiers.contains(KeyModifiers::CONTROL) => {
                    drop(state);
                    let _ = signal_sender.send(AppSignal::NavigateDown).await;
                    return Ok(EventPropagation::ConsumedRender);
                }
                KeyEvent {
                    code: KeyCode::Enter,
                    ..
                } => {
                    drop(state);
                    let _ = signal_sender.send(AppSignal::EnterResultDetail).await;
                    return Ok(EventPropagation::ConsumedRender);
                }
                KeyEvent {
                    code: KeyCode::Char('s'),
                    modifiers,
                    ..
                } if modifiers.contains(KeyModifiers::CONTROL) => {
                    drop(state);
                    let _ = signal_sender.send(AppSignal::EnterSessionViewer).await;
                    return Ok(EventPropagation::ConsumedRender);
                }
                _ => {}
            }
        }
        
        Ok(EventPropagation::Propagate)
    }

    async fn render(
        &mut self,
        global_data: &mut GlobalData<AppState, AppSignal>,
        component_registry_map: &mut ComponentRegistryMap<AppState, AppSignal>,
        has_focus: &mut HasFocus,
        surface: &mut Surface,
    ) -> CommonResult<()> {
        let state = global_data.state.read().unwrap();
        let window_size = global_data.window_size;
        
        // Create layout with search bar at top and results below
        let mut render_pipeline = RenderPipeline::new();
        let mut render_ops = render_ops!();
        
        // Render search bar
        let search_bar_height = 3;
        let search_bar_area = Rect {
            x: 0,
            y: 0,
            width: window_size.col_count,
            height: search_bar_height,
        };
        
        render_ops.push(RenderOp::MoveCursorPositionAbs(Position::new(0, 0)));
        
        // Draw search box border
        let title = if state.is_searching {
            "Search [searching...]"
        } else {
            "Search"
        };
        
        render_ops.push(RenderOp::PaintTextWithAttributes(
            title.into(),
            *DEFAULT_STYLE_BOLD,
        ));
        
        // Draw search query
        render_ops.push(RenderOp::MoveCursorPositionAbs(Position::new(1, 1)));
        render_ops.push(RenderOp::PaintTextWithAttributes(
            format!("> {}", state.query).into(),
            *DEFAULT_STYLE,
        ));
        
        // Render results list
        let results_y_start = search_bar_height;
        let results_height = window_size.row_count - search_bar_height;
        
        // Draw results count
        render_ops.push(RenderOp::MoveCursorPositionAbs(Position::new(0, results_y_start)));
        render_ops.push(RenderOp::PaintTextWithAttributes(
            format!("Results: {}", state.search_results.len()).into(),
            *DEFAULT_STYLE_DIM,
        ));
        
        // Draw results
        let visible_results = state.search_results
            .iter()
            .skip(state.selected_index.saturating_sub(10))
            .take(results_height as usize - 2)
            .enumerate();
        
        for (i, message) in visible_results {
            let y = results_y_start + i as u16 + 1;
            let is_selected = state.selected_index.saturating_sub(state.selected_index.saturating_sub(10)) == i;
            
            render_ops.push(RenderOp::MoveCursorPositionAbs(Position::new(0, y)));
            
            let style = if is_selected {
                *DEFAULT_STYLE_REVERSED
            } else {
                *DEFAULT_STYLE
            };
            
            let display_text = format!(
                "[{:?}] {}",
                message.role,
                message.content
                    .as_ref()
                    .and_then(|c| c.get(0))
                    .and_then(|c| c.text.as_ref())
                    .map(|t| {
                        let first_line = t.lines().next().unwrap_or("");
                        if first_line.len() > 80 {
                            format!("{}...", &first_line[..77])
                        } else {
                            first_line.to_string()
                        }
                    })
                    .unwrap_or_else(|| "No content".to_string())
            );
            
            render_ops.push(RenderOp::PaintTextWithAttributes(
                display_text.into(),
                style,
            ));
        }
        
        // Add help text at bottom
        render_ops.push(RenderOp::MoveCursorPositionAbs(Position::new(
            0,
            window_size.row_count - 1,
        )));
        render_ops.push(RenderOp::PaintTextWithAttributes(
            "↑/↓: Navigate | Enter: View | Ctrl+S: Session | ?: Help | q: Quit".into(),
            *DEFAULT_STYLE_DIM,
        ));
        
        render_pipeline.push(ZOrder::Normal, render_ops);
        surface.render_pipeline = render_pipeline;
        
        Ok(())
    }
}