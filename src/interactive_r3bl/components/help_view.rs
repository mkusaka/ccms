use r3bl_tui::{
    Component, GlobalData, ComponentRegistryMap, HasFocus, InputEvent,
    KeyEvent, KeyCode, EventPropagation, CommonResult,
    RenderPipeline, Surface, render_ops, RenderOp, Position,
    ZOrder, BoxedSafeComponent, async_trait,
    DEFAULT_STYLE, DEFAULT_STYLE_BOLD, DEFAULT_STYLE_DIM,
    style, TuiColor, TuiTextAttrib,
};

use crate::interactive_r3bl::state::{AppState, AppSignal};

pub struct HelpView;

impl HelpView {
    pub fn new() -> Self {
        Self
    }
    
    pub fn new_boxed() -> BoxedSafeComponent<AppState, AppSignal> {
        Box::new(Self::new())
    }
}

#[async_trait]
impl Component<AppState, AppSignal> for HelpView {
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
                    code: KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('?'),
                    ..
                } => {
                    let tx = global_data.get_signal_sender();
                    let _ = tx.send(AppSignal::ExitCurrentView).await;
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
        let window_size = global_data.window_size;
        
        let mut render_pipeline = RenderPipeline::new();
        let mut render_ops = render_ops!();
        
        // Title
        render_ops.push(RenderOp::MoveCursorPositionAbs(Position::new(0, 0)));
        render_ops.push(RenderOp::PaintTextWithAttributes(
            "CCMS Interactive Search - Help".into(),
            style! {
                color: TuiColor::Blue,
                attrib: *TuiTextAttrib::Bold
            },
        ));
        
        // Help content
        let help_items = vec![
            ("", "Navigation:"),
            ("↑/k", "Move up"),
            ("↓/j", "Move down"),
            ("Enter", "View message details"),
            ("Ctrl+S", "View full session"),
            ("", ""),
            ("", "Search:"),
            ("Type", "Search query"),
            ("/", "Focus search bar"),
            ("", ""),
            ("", "Global:"),
            ("?", "Toggle this help"),
            ("q", "Quit / Go back"),
            ("Esc", "Go back"),
            ("Ctrl+C", "Quit application"),
        ];
        
        let mut y = 2;
        for (key, desc) in help_items {
            if y >= window_size.row_count - 1 {
                break;
            }
            
            render_ops.push(RenderOp::MoveCursorPositionAbs(Position::new(2, y)));
            
            if key.is_empty() && desc.ends_with(':') {
                // Section header
                render_ops.push(RenderOp::PaintTextWithAttributes(
                    desc.into(),
                    *DEFAULT_STYLE_BOLD,
                ));
            } else if !key.is_empty() {
                // Key binding
                render_ops.push(RenderOp::PaintTextWithAttributes(
                    format!("{:<12} {}", key, desc).into(),
                    *DEFAULT_STYLE,
                ));
            }
            
            y += 1;
        }
        
        // Footer
        render_ops.push(RenderOp::MoveCursorPositionAbs(Position::new(
            0,
            window_size.row_count - 1,
        )));
        render_ops.push(RenderOp::PaintTextWithAttributes(
            "Press Esc, q, or ? to close help".into(),
            *DEFAULT_STYLE_DIM,
        ));
        
        render_pipeline.push(ZOrder::Normal, render_ops);
        surface.render_pipeline = render_pipeline;
        
        Ok(())
    }
}