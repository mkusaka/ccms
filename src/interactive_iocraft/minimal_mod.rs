use anyhow::Result;
use iocraft::prelude::*;

#[component]
pub fn MinimalApp(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
    let counter = hooks.use_state(|| 0);
    
    // Handle any key press to exit
    hooks.use_terminal_events({
        move |event| {
            if let TerminalEvent::Key(_) = event {
                std::process::exit(0);
            }
        }
    });
    
    let count = *counter.read();
    
    element! {
        View {
            Text(content: "Minimal Test App")
            Text(content: format!("Counter: {}", count))
            Text(content: "Press any key to exit")
        }
    }
}

pub async fn run_minimal_interactive() -> Result<()> {
    element! { MinimalApp() }.render_loop().await?;
    Ok(())
}