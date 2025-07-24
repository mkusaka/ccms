use arboard::Clipboard;

/// Service for handling clipboard operations
pub struct ClipboardService {
    clipboard: Option<Clipboard>,
}

impl ClipboardService {
    pub fn new() -> Self {
        let clipboard = Clipboard::new().ok();
        Self { clipboard }
    }
    
    /// Copy text to clipboard
    pub fn copy(&mut self, text: &str) -> anyhow::Result<()> {
        if let Some(clipboard) = &mut self.clipboard {
            clipboard
                .set_text(text)
                .map_err(|e| anyhow::anyhow!("Failed to copy to clipboard: {}", e))?;
            Ok(())
        } else {
            Err(anyhow::anyhow!("Clipboard not available"))
        }
    }
    
    /// Check if clipboard is available
    pub fn is_available(&self) -> bool {
        self.clipboard.is_some()
    }
}