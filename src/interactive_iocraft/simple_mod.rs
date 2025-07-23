use anyhow::Result;
use iocraft::prelude::*;
use iocraft::{KeyCode, KeyModifiers};

#[component]
pub fn SimpleApp(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
    let query = hooks.use_state(|| String::new());
    let cursor_pos = hooks.use_state(|| 0);
    let results = hooks.use_state(|| vec!["Result 1".to_string(), "Result 2".to_string(), "Result 3".to_string()]);
    let selected_index = hooks.use_state(|| 0);
    
    // Handle keyboard events
    hooks.use_terminal_events({
        let mut query = query.clone();
        let mut cursor_pos = cursor_pos.clone();
        let mut selected_index = selected_index.clone();
        
        move |event| {
            if let TerminalEvent::Key(key) = event {
                match key.code {
                    KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        std::process::exit(0);
                    }
                    KeyCode::Esc => {
                        std::process::exit(0);
                    }
                    KeyCode::Char(c) => {
                        let pos = *cursor_pos.read();
                        let mut q = query.write();
                        if pos <= q.len() {
                            q.insert(pos, c);
                        }
                        drop(q);
                        *cursor_pos.write() = pos + 1;
                    }
                    KeyCode::Backspace => {
                        let pos = *cursor_pos.read();
                        if pos > 0 {
                            let mut q = query.write();
                            if pos <= q.len() && pos > 0 {
                                q.remove(pos - 1);
                            }
                            drop(q);
                            *cursor_pos.write() = pos - 1;
                        }
                    }
                    KeyCode::Left => {
                        let pos = *cursor_pos.read();
                        if pos > 0 {
                            *cursor_pos.write() = pos - 1;
                        }
                    }
                    KeyCode::Right => {
                        let pos = *cursor_pos.read();
                        let q_len = query.read().len();
                        if pos < q_len {
                            *cursor_pos.write() = pos + 1;
                        }
                    }
                    KeyCode::Up => {
                        let mut idx = selected_index.write();
                        if *idx > 0 {
                            *idx -= 1;
                        }
                    }
                    KeyCode::Down => {
                        let mut idx = selected_index.write();
                        let max_idx = results.read().len().saturating_sub(1);
                        if *idx < max_idx {
                            *idx += 1;
                        }
                    }
                    _ => {}
                }
            }
        }
    });
    
    let query_str = query.read().clone();
    let cursor_position = *cursor_pos.read();
    let selected_idx = *selected_index.read();
    let results_list = results.read().clone();
    
    element! {
        View(flex_direction: FlexDirection::Column) {
            // Header
            Text(content: "Simple Interactive Search", weight: Weight::Bold, color: Color::Cyan)
            Text(content: "Type to search, ↑/↓ to navigate, Ctrl+C or Esc to exit")
            Text(content: "")
            
            // Search input
            View(flex_direction: FlexDirection::Row) {
                Text(content: "Search: ")
                #({
                    let mut display = String::new();
                    for (i, ch) in query_str.chars().enumerate() {
                        if i == cursor_position {
                            display.push('|');
                        }
                        display.push(ch);
                    }
                    if cursor_position == query_str.len() {
                        display.push('|');
                    }
                    vec![element! { Text(content: display) }]
                })
            }
            
            Text(content: "")
            
            // Results
            #(results_list.iter().enumerate().map(|(i, result)| {
                let is_selected = i == selected_idx;
                element! {
                    View(flex_direction: FlexDirection::Row) {
                        Text(
                            content: if is_selected { "> " } else { "  " },
                            color: if is_selected { Color::Cyan } else { Color::White }
                        )
                        Text(
                            content: result.clone(),
                            color: if is_selected { Color::White } else { Color::Grey }
                        )
                    }
                }
            }))
        }
    }
}

pub async fn run_simple_interactive() -> Result<()> {
    element! { SimpleApp() }.render_loop().await?;
    Ok(())
}