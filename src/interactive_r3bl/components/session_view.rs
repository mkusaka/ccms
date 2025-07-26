use r3bl_tui::{
    Component, GlobalData, ComponentRegistryMap, HasFocus, InputEvent,
    KeyEvent, KeyCode, EventPropagation, CommonResult,
    RenderPipeline, Surface, render_ops, RenderOp, Position,
    ZOrder, BoxedSafeComponent, async_trait,
    DEFAULT_STYLE, DEFAULT_STYLE_BOLD, DEFAULT_STYLE_DIM,
    style, TuiColor, TuiTextAttrib,
};

use crate::interactive_r3bl::state::{AppState, AppSignal};

pub struct SessionView;

impl SessionView {
    pub fn new() -> Self {
        Self
    }
    
    pub fn new_boxed() -> BoxedSafeComponent<AppState, AppSignal> {
        Box::new(Self::new())
    }
}

#[async_trait]
impl Component<AppState, AppSignal> for SessionView {
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
                    if state.session_scroll_offset > 0 {
                        state.session_scroll_offset -= 1;
                    }
                    return Ok(EventPropagation::ConsumedRender);
                }
                KeyEvent {
                    code: KeyCode::Down | KeyCode::Char('j'),
                    ..
                } => {
                    let mut state = global_data.state.write().unwrap();
                    state.session_scroll_offset += 1;
                    return Ok(EventPropagation::ConsumedRender);
                }
                KeyEvent {
                    code: KeyCode::PageUp,
                    ..
                } => {
                    let mut state = global_data.state.write().unwrap();
                    state.session_scroll_offset = state.session_scroll_offset.saturating_sub(10);
                    return Ok(EventPropagation::ConsumedRender);
                }
                KeyEvent {
                    code: KeyCode::PageDown,
                    ..
                } => {
                    let mut state = global_data.state.write().unwrap();
                    state.session_scroll_offset += 10;
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
        let title = if let Some(path) = &state.selected_session_path {
            format!("Session: {}", path)
        } else {
            "Session Viewer".to_string()
        };
        render_ops.push(RenderOp::PaintTextWithAttributes(
            title.into(),
            *DEFAULT_STYLE_BOLD,
        ));
        
        // Message count
        render_ops.push(RenderOp::MoveCursorPositionAbs(Position::new(0, 1)));
        render_ops.push(RenderOp::PaintTextWithAttributes(
            format!("Messages: {}", state.session_messages.len()).into(),
            *DEFAULT_STYLE_DIM,
        ));
        
        // Messages
        let messages_start_y = 3;
        let available_height = window_size.row_count - messages_start_y - 1;
        
        let visible_messages = state.session_messages
            .iter()
            .skip(state.session_scroll_offset)
            .take(available_height as usize);
        
        let mut y = messages_start_y;
        for message in visible_messages {
            if y >= window_size.row_count - 1 {
                break;
            }
            
            // Role
            render_ops.push(RenderOp::MoveCursorPositionAbs(Position::new(0, y)));
            let role_style = match message.role {
                crate::schemas::MessageRole::User => style! {
                    color: TuiColor::Blue,
                    attrib: *TuiTextAttrib::Bold
                },
                crate::schemas::MessageRole::Assistant => style! {
                    color: TuiColor::Green,
                    attrib: *TuiTextAttrib::Bold
                },
                crate::schemas::MessageRole::System => style! {
                    color: TuiColor::Red,
                    attrib: *TuiTextAttrib::Bold
                },
                crate::schemas::MessageRole::Summary => style! {
                    color: TuiColor::Yellow,
                    attrib: *TuiTextAttrib::Bold
                },
            };
            render_ops.push(RenderOp::PaintTextWithAttributes(
                format!("[{:?}]", message.role).into(),
                role_style,
            ));
            y += 1;
            
            // Content preview
            if let Some(content_blocks) = &message.content {
                for block in content_blocks {
                    if let Some(text) = &block.text {
                        let preview = text.lines().next().unwrap_or("").to_string();
                        let preview = if preview.len() > 80 {
                            format!("{}...", &preview[..77])
                        } else {
                            preview
                        };
                        
                        if y < window_size.row_count - 1 {
                            render_ops.push(RenderOp::MoveCursorPositionAbs(Position::new(2, y)));
                            render_ops.push(RenderOp::PaintTextWithAttributes(
                                preview.into(),
                                *DEFAULT_STYLE,
                            ));
                            y += 1;
                        }
                    }
                }
            }
            
            // Separator
            if y < window_size.row_count - 1 {
                render_ops.push(RenderOp::MoveCursorPositionAbs(Position::new(0, y)));
                render_ops.push(RenderOp::PaintTextWithAttributes(
                    "─".repeat(window_size.col_count as usize).into(),
                    *DEFAULT_STYLE_DIM,
                ));
                y += 1;
            }
        }
        
        // Help text at bottom
        render_ops.push(RenderOp::MoveCursorPositionAbs(Position::new(
            0,
            window_size.row_count - 1,
        )));
        render_ops.push(RenderOp::PaintTextWithAttributes(
            "↑/↓: Scroll | PgUp/PgDn: Page | Esc/q: Back".into(),
            *DEFAULT_STYLE_DIM,
        ));
        
        render_pipeline.push(ZOrder::Normal, render_ops);
        surface.render_pipeline = render_pipeline;
        
        Ok(())
    }
}