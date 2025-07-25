//! Clipboard functionality hook

use arboard::Clipboard;
use iocraft::prelude::*;
use std::time::Duration;

pub struct UseClipboardResult {
    pub message: Option<String>,
}

impl UseClipboardResult {
    pub fn copy(&self, hooks: &mut Hooks, text: String) {
        let mut message = hooks.use_state(|| None::<String>);
        
        // Use arboard for secure clipboard operations
        match copy_to_clipboard(&text) {
            Ok(_) => {
                message.set(Some("Copied to clipboard!".to_string()));
                
                // Clear message after delay
                hooks.use_future({
                    let mut message = message.clone();
                    async move {
                        smol::Timer::after(Duration::from_secs(2)).await;
                        message.set(None);
                    }
                });
            }
            Err(e) => {
                message.set(Some(format!("Clipboard error: {}", e)));
            }
        }
    }
}

/// Hook for clipboard operations
pub fn use_clipboard(hooks: &mut Hooks) -> UseClipboardResult {
    let message = hooks.use_state(|| None::<String>);
    
    UseClipboardResult {
        message: message.read().clone(),
    }
}

/// Convenience function to copy text to clipboard using arboard
pub fn copy_to_clipboard(text: &str) -> Result<(), String> {
    let mut clipboard = Clipboard::new()
        .map_err(|e| format!("Failed to access clipboard: {}", e))?;
    
    clipboard.set_text(text)
        .map_err(|e| format!("Failed to copy to clipboard: {}", e))
}