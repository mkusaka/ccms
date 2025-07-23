use anyhow::Result;
use iocraft::prelude::*;

pub async fn run_minimal_interactive() -> Result<()> {
    println!("Starting minimal interactive mode...");
    println!("Press any key to exit.");
    
    // Just read one key and exit
    use crossterm::{
        event::{read, Event},
        terminal::{disable_raw_mode, enable_raw_mode},
    };
    
    enable_raw_mode()?;
    
    loop {
        if let Event::Key(_) = read()? {
            break;
        }
    }
    
    disable_raw_mode()?;
    println!("\nExiting...");
    
    Ok(())
}