use std::fs::OpenOptions;
use std::io::Write;
use std::sync::Mutex;

static DEBUG_FILE: Mutex<Option<std::fs::File>> = Mutex::new(None);

pub fn init_debug_log() {
    let mut file_guard = DEBUG_FILE.lock().unwrap();
    if file_guard.is_none() {
        if let Ok(file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open("debug.log")
        {
            *file_guard = Some(file);
        }
    }
}

#[macro_export]
macro_rules! debug_log {
    ($($arg:tt)*) => {
        $crate::interactive_ratatui::ui::debug_log::write_debug_log(&format!($($arg)*));
    };
}

pub fn write_debug_log(msg: &str) {
    let mut file_guard = DEBUG_FILE.lock().unwrap();
    if let Some(file) = file_guard.as_mut() {
        let now = chrono::Local::now();
        let _ = writeln!(file, "[{}] {}", now.format("%H:%M:%S%.3f"), msg);
        let _ = file.flush();
    }
}
