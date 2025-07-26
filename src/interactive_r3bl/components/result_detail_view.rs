use r3bl_tui::{
    Component, GlobalData, ComponentRegistryMap, HasFocus, InputEvent,
    KeyEvent, KeyCode, EventPropagation, CommonResult,
    RenderPipeline, Surface, render_ops, RenderOp, Position,
    ZOrder, BoxedSafeComponent, async_trait,
    DEFAULT_STYLE, DEFAULT_STYLE_BOLD, DEFAULT_STYLE_DIM,
};

use crate::interactive_r3bl::state::{AppState, AppSignal};

pub struct ResultDetailView;

impl ResultDetailView {
    pub fn new() -> Self {
        Self
    }
    
    pub fn new_boxed() -> BoxedSafeComponent<AppState, AppSignal> {
        Box::new(Self::new())
    }
}

#[async_trait]
impl Component<AppState, AppSignal> for ResultDetailView {
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
            match key {
                KeyEvent {
                    code: KeyCode::Esc | KeyCode::Char('q'),
                    ..
                } => {
                    let tx = global_data.get_signal_sender();
                    let _ = tx.send(AppSignal::ExitCurrentView).await;
                    return Ok(EventPropagation::ConsumedRender);
                }
                KeyEvent {
                    code: KeyCode::Up | KeyCode::Char('k'),
                    ..
                } => {
                    let mut state = global_data.state.write().unwrap();
                    if state.result_detail_scroll_offset > 0 {
                        state.result_detail_scroll_offset -= 1;
                    }
                    return Ok(EventPropagation::ConsumedRender);
                }
                KeyEvent {
                    code: KeyCode::Down | KeyCode::Char('j'),
                    ..
                } => {
                    let mut state = global_data.state.write().unwrap();
                    state.result_detail_scroll_offset += 1;
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
        _component_registry_map: &mut ComponentRegistryMap<AppState, AppSignal>,
        _has_focus: &mut HasFocus,
        surface: &mut Surface,
    ) -> CommonResult<()> {
        let state = global_data.state.read().unwrap();
        let window_size = global_data.window_size;
        
        let mut render_pipeline = RenderPipeline::new();
        let mut render_ops = render_ops!();
        
        // Title bar
        render_ops.push(RenderOp::MoveCursorPositionAbs(Position::new(0, 0)));
        render_ops.push(RenderOp::PaintTextWithAttributes(
            "Message Detail".into(),
            *DEFAULT_STYLE_BOLD,
        ));
        
        if let Some(message) = state.get_selected_message() {
            // Message metadata
            render_ops.push(RenderOp::MoveCursorPositionAbs(Position::new(0, 2)));
            render_ops.push(RenderOp::PaintTextWithAttributes(
                format!("Role: {:?}", message.role).into(),
                *DEFAULT_STYLE,
            ));
            
            if let Some(timestamp) = &message.timestamp {
                render_ops.push(RenderOp::MoveCursorPositionAbs(Position::new(0, 3)));
                render_ops.push(RenderOp::PaintTextWithAttributes(
                    format!("Time: {}", timestamp).into(),
                    *DEFAULT_STYLE,
                ));
            }
            
            if let Some(session) = &message.session {
                render_ops.push(RenderOp::MoveCursorPositionAbs(Position::new(0, 4)));
                render_ops.push(RenderOp::PaintTextWithAttributes(
                    format!("Session: {}", session).into(),
                    *DEFAULT_STYLE,
                ));
            }
            
            // Content
            render_ops.push(RenderOp::MoveCursorPositionAbs(Position::new(0, 6)));
            render_ops.push(RenderOp::PaintTextWithAttributes(
                "Content:".into(),
                *DEFAULT_STYLE_BOLD,
            ));
            
            if let Some(content_blocks) = &message.content {
                let mut y = 7;
                for block in content_blocks {
                    if let Some(text) = &block.text {
                        let lines: Vec<&str> = text.lines().collect();
                        let visible_lines = lines
                            .iter()
                            .skip(state.result_detail_scroll_offset)
                            .take((window_size.row_count - 8) as usize);
                        
                        for line in visible_lines {
                            render_ops.push(RenderOp::MoveCursorPositionAbs(Position::new(0, y)));
                            render_ops.push(RenderOp::PaintTextWithAttributes(
                                (*line).into(),
                                *DEFAULT_STYLE,
                            ));
                            y += 1;
                            if y >= window_size.row_count - 1 {
                                break;
                            }
                        }
                    }
                }
            }
        }
        
        // Help text at bottom
        render_ops.push(RenderOp::MoveCursorPositionAbs(Position::new(
            0,
            window_size.row_count - 1,
        )));
        render_ops.push(RenderOp::PaintTextWithAttributes(
            "↑/↓: Scroll | Esc/q: Back".into(),
            *DEFAULT_STYLE_DIM,
        ));
        
        render_pipeline.push(ZOrder::Normal, render_ops);
        surface.render_pipeline = render_pipeline;
        
        Ok(())
    }
}