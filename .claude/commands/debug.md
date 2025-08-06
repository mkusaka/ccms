---
description: Debug TUI applications by adding file-based logging
allowed-tools: Write, Edit, MultiEdit, Read, Bash
argument-hint: [component-name]
---

# Debug TUI Application with File Logging

When debugging Terminal User Interface (TUI) applications where eprintln! and stdout would break the display, use file-based logging instead.

## Create Debug Logging Module

First, create a debug logging module at `src/interactive_ratatui/ui/debug_log.rs`:

```rust
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

pub fn write_debug_log(msg: &str) {
    let mut file_guard = DEBUG_FILE.lock().unwrap();
    if let Some(file) = file_guard.as_mut() {
        let now = chrono::Local::now();
        let _ = writeln!(file, "[{}] {}", now.format("%H:%M:%S%.3f"), msg);
        let _ = file.flush();
    }
}
```

## Add Module Declaration

Add the module to `src/interactive_ratatui/ui/mod.rs`:
```rust
pub mod debug_log;
```

## Use Debug Logging in Components

In the component you want to debug (e.g., `$ARGUMENTS`):

```rust
use crate::interactive_ratatui::ui::debug_log::{init_debug_log, write_debug_log};

// Initialize and write logs
init_debug_log();
write_debug_log(&format!("[ComponentName] Event: {:?}", event));
```

## Monitor Debug Output

In a separate terminal, monitor the debug log:
```bash
tail -f debug.log
```

## Clean Up After Debugging

**Important:** Remove all debug logging code before committing:
1. Delete the `debug_log.rs` file
2. Remove the module declaration from `mod.rs`
3. Remove all `init_debug_log()` and `write_debug_log()` calls
4. Clean up imports

## Example Usage

When debugging cursor position issues in text input:
```rust
write_debug_log(&format!(
    "[TextInput] Key: {:?}, text='{}', cursor_pos={}",
    key, self.text, self.cursor_position
));
```

This approach allows you to debug TUI applications without breaking the terminal display.