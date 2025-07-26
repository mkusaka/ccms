use r3bl_tui::*;
use r3bl_ansi_color::*;
use tokio::sync::mpsc;
use std::fmt::Debug;

use crate::SearchOptions;
use super::state::{AppState, AppSignal};

pub struct SearchApp {
    file_pattern: String,
    options: SearchOptions,
    signal_sender: mpsc::Sender<AppSignal>,
}

impl SearchApp {
    pub fn new(file_pattern: String, options: SearchOptions, signal_sender: mpsc::Sender<AppSignal>) -> Self {
        Self {
            file_pattern,
            options,
            signal_sender,
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
        // Set initial focus
        has_focus.set_id(ComponentId::from(0));
        
        // Start with empty search
        let sender = self.signal_sender.clone();
        tokio::spawn(async move {
            // Simulate initial search
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            let _ = sender.send(AppSignal::SearchCompleted(vec![
                "Result 1".to_string(),
                "Result 2".to_string(),
                "Result 3".to_string(),
            ])).await;
        });
        
        Ok(())
    }

    async fn app_handle_input_event(
        &mut self,
        input_event: InputEvent,
        state: &mut AppState,
        _component_registry_map: &mut ComponentRegistryMap<Self::State, Self::Signal>,
        _has_focus: &mut HasFocus,
    ) -> CommonResult<EventPropagation> {
        match input_event {
            InputEvent::Keyboard(KeyEvent::Char('q')) => {
                return Ok(EventPropagation::ConsumedReqExit);
            }
            InputEvent::Keyboard(KeyEvent::Char('?')) => {
                state.show_help = !state.show_help;
                return Ok(EventPropagation::ConsumedReqRedraw);
            }
            InputEvent::Keyboard(KeyEvent::Up) | InputEvent::Keyboard(KeyEvent::Char('k')) => {
                if state.selected_index > 0 {
                    state.selected_index -= 1;
                }
                return Ok(EventPropagation::ConsumedReqRedraw);
            }
            InputEvent::Keyboard(KeyEvent::Down) | InputEvent::Keyboard(KeyEvent::Char('j')) => {
                if state.selected_index < state.results.len().saturating_sub(1) {
                    state.selected_index += 1;
                }
                return Ok(EventPropagation::ConsumedReqRedraw);
            }
            InputEvent::Keyboard(KeyEvent::Char(c)) => {
                state.query.push(c);
                state.is_searching = true;
                
                // Trigger search
                let query = state.query.clone();
                let sender = self.signal_sender.clone();
                tokio::spawn(async move {
                    tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;
                    let results = vec![
                        format!("Result for '{}'", query),
                        format!("Another result for '{}'", query),
                    ];
                    let _ = sender.send(AppSignal::SearchCompleted(results)).await;
                });
                
                return Ok(EventPropagation::ConsumedReqRedraw);
            }
            InputEvent::Keyboard(KeyEvent::Backspace) => {
                state.query.pop();
                state.is_searching = true;
                
                // Trigger search
                let query = state.query.clone();
                let sender = self.signal_sender.clone();
                tokio::spawn(async move {
                    tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;
                    let results = if query.is_empty() {
                        vec![
                            "Result 1".to_string(),
                            "Result 2".to_string(),
                            "Result 3".to_string(),
                        ]
                    } else {
                        vec![
                            format!("Result for '{}'", query),
                            format!("Another result for '{}'", query),
                        ]
                    };
                    let _ = sender.send(AppSignal::SearchCompleted(results)).await;
                });
                
                return Ok(EventPropagation::ConsumedReqRedraw);
            }
            _ => {}
        }
        
        Ok(EventPropagation::Propagate)
    }

    async fn app_handle_signal(
        &mut self,
        signal: Self::Signal,
        state: &mut AppState,
        _component_registry_map: &mut ComponentRegistryMap<Self::State, Self::Signal>,
        _has_focus: &mut HasFocus,
    ) -> CommonResult<EventPropagation> {
        match signal {
            AppSignal::SearchCompleted(results) => {
                state.results = results;
                state.is_searching = false;
                state.selected_index = 0;
            }
            AppSignal::UpdateQuery(query) => {
                state.query = query;
            }
            AppSignal::NavigateUp => {
                if state.selected_index > 0 {
                    state.selected_index -= 1;
                }
            }
            AppSignal::NavigateDown => {
                if state.selected_index < state.results.len().saturating_sub(1) {
                    state.selected_index += 1;
                }
            }
            AppSignal::ShowHelp => {
                state.show_help = !state.show_help;
            }
            AppSignal::Quit => {
                return Ok(EventPropagation::ConsumedReqExit);
            }
        }
        
        Ok(EventPropagation::ConsumedReqRedraw)
    }

    async fn app_render(
        &mut self,
        state: &mut AppState,
        _component_registry_map: &mut ComponentRegistryMap<Self::State, Self::Signal>,
        _has_focus: &mut HasFocus,
    ) -> CommonResult<RenderPipeline> {
        let mut pipeline = RenderPipeline::new();
        let mut render_ops = vec![];
        
        // Clear screen
        render_ops.push(RenderOp::ClearScreen);
        
        // Title
        render_ops.push(RenderOp::MoveCursorPositionAbs(position!(col_index: 0, row_index: 0)));
        render_ops.push(RenderOp::SetFgColor(Color::Blue));
        render_ops.push(RenderOp::SetBoldPrint);
        render_ops.push(RenderOp::PaintTextWithAttributes("CCMS Search - R3BL TUI PoC".into(), None));
        render_ops.push(RenderOp::ResetColor);
        
        // Search bar
        render_ops.push(RenderOp::MoveCursorPositionAbs(position!(col_index: 0, row_index: 2)));
        render_ops.push(RenderOp::PaintTextWithAttributes("Search: ".into(), None));
        render_ops.push(RenderOp::SetFgColor(Color::Yellow));
        render_ops.push(RenderOp::PaintTextWithAttributes(state.query.clone().into(), None));
        if state.is_searching {
            render_ops.push(RenderOp::PaintTextWithAttributes(" [searching...]".into(), None));
        }
        render_ops.push(RenderOp::ResetColor);
        
        // Results
        render_ops.push(RenderOp::MoveCursorPositionAbs(position!(col_index: 0, row_index: 4)));
        render_ops.push(RenderOp::PaintTextWithAttributes(format!("Results ({})", state.results.len()).into(), None));
        
        // Display results
        for (i, result) in state.results.iter().enumerate() {
            render_ops.push(RenderOp::MoveCursorPositionAbs(position!(
                col_index: 0, 
                row_index: 6 + i as u16
            )));
            
            if i == state.selected_index {
                render_ops.push(RenderOp::SetBgColor(Color::Rgb(80, 80, 80)));
                render_ops.push(RenderOp::SetFgColor(Color::White));
                render_ops.push(RenderOp::PaintTextWithAttributes(format!("> {}", result).into(), None));
                render_ops.push(RenderOp::ResetColor);
            } else {
                render_ops.push(RenderOp::PaintTextWithAttributes(format!("  {}", result).into(), None));
            }
        }
        
        // Help
        if state.show_help {
            render_ops.push(RenderOp::MoveCursorPositionAbs(position!(col_index: 0, row_index: 12)));
            render_ops.push(RenderOp::SetFgColor(Color::Green));
            render_ops.push(RenderOp::PaintTextWithAttributes("Help:".into(), None));
            render_ops.push(RenderOp::ResetColor);
            render_ops.push(RenderOp::MoveCursorPositionAbs(position!(col_index: 0, row_index: 13)));
            render_ops.push(RenderOp::PaintTextWithAttributes("  ↑/k: Move up".into(), None));
            render_ops.push(RenderOp::MoveCursorPositionAbs(position!(col_index: 0, row_index: 14)));
            render_ops.push(RenderOp::PaintTextWithAttributes("  ↓/j: Move down".into(), None));
            render_ops.push(RenderOp::MoveCursorPositionAbs(position!(col_index: 0, row_index: 15)));
            render_ops.push(RenderOp::PaintTextWithAttributes("  ?: Toggle help".into(), None));
            render_ops.push(RenderOp::MoveCursorPositionAbs(position!(col_index: 0, row_index: 16)));
            render_ops.push(RenderOp::PaintTextWithAttributes("  q: Quit".into(), None));
        } else {
            render_ops.push(RenderOp::MoveCursorPositionAbs(position!(col_index: 0, row_index: 12)));
            render_ops.push(RenderOp::SetFgColor(Color::Rgb(80, 80, 80)));
            render_ops.push(RenderOp::PaintTextWithAttributes("Press ? for help, q to quit".into(), None));
            render_ops.push(RenderOp::ResetColor);
        }
        
        pipeline.push(ZOrder::Normal, render_ops);
        Ok(pipeline)
    }
}