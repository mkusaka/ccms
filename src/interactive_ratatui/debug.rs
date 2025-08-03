use std::fs::OpenOptions;
use std::io::Write;
use std::sync::Mutex;

// Static mutex for thread-safe debug logging
lazy_static::lazy_static! {
    static ref DEBUG_FILE: Mutex<()> = Mutex::new(());
}

#[macro_export]
macro_rules! debug_log {
    ($($arg:tt)*) => {
        {
            let _ = $crate::interactive_ratatui::debug::write_debug_log(&format!($($arg)*));
        }
    };
}

pub fn write_debug_log(message: &str) -> std::io::Result<()> {
    let _lock = DEBUG_FILE.lock().unwrap();
    
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open("./debug.log")?;
    
    let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
    writeln!(file, "[{}] {}", timestamp, message)?;
    file.flush()?;
    
    Ok(())
}

pub fn clear_debug_log() -> std::io::Result<()> {
    let _lock = DEBUG_FILE.lock().unwrap();
    std::fs::write("./debug.log", "")?;
    Ok(())
}