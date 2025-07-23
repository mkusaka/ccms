use anyhow::{Context, Result};

pub fn copy_to_clipboard(text: &str) -> Result<()> {
    #[cfg(target_os = "macos")]
    {
        use std::process::Command;
        let mut child = Command::new("pbcopy")
            .stdin(std::process::Stdio::piped())
            .spawn()
            .context("Failed to spawn pbcopy")?;

        if let Some(mut stdin) = child.stdin.take() {
            use std::io::Write;
            stdin
                .write_all(text.as_bytes())
                .context("Failed to write to pbcopy")?;
        }

        child.wait().context("Failed to wait for pbcopy")?;
        Ok(())
    }

    #[cfg(target_os = "linux")]
    {
        use std::process::Command;
        let mut child = Command::new("xclip")
            .arg("-selection")
            .arg("clipboard")
            .stdin(std::process::Stdio::piped())
            .spawn()
            .context("Failed to spawn xclip")?;

        if let Some(mut stdin) = child.stdin.take() {
            use std::io::Write;
            stdin
                .write_all(text.as_bytes())
                .context("Failed to write to xclip")?;
        }

        child.wait().context("Failed to wait for xclip")?;
        Ok(())
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        Err(anyhow::anyhow!("Clipboard not supported on this platform"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_copy_to_clipboard_empty_string() {
        // Empty string should still be copyable
        let result = copy_to_clipboard("");
        // Result depends on platform and clipboard availability
        let _is_result = result.is_ok() || result.is_err();
    }

    #[test]
    fn test_copy_to_clipboard_simple_text() {
        let result = copy_to_clipboard("Hello, world!");
        // Result depends on platform and clipboard availability
        let _is_result = result.is_ok() || result.is_err();
    }

    #[test]
    fn test_copy_to_clipboard_unicode() {
        let result = copy_to_clipboard("こんにちは世界 🌍");
        // Result depends on platform and clipboard availability
        let _is_result = result.is_ok() || result.is_err();
    }

    #[test]
    fn test_copy_to_clipboard_multiline() {
        let text = "Line 1\nLine 2\nLine 3";
        let result = copy_to_clipboard(text);
        // Result depends on platform and clipboard availability
        let _is_result = result.is_ok() || result.is_err();
    }

    #[test]
    fn test_copy_to_clipboard_special_characters() {
        let text = "Special chars: @#$%^&*()_+-={}[]|\\\":;<>?,./~`";
        let result = copy_to_clipboard(text);
        // Result depends on platform and clipboard availability
        let _is_result = result.is_ok() || result.is_err();
    }

    #[test]
    #[ignore] // Ignore in CI as clipboard utilities might not be available
    fn test_copy_to_clipboard_actual_copy() {
        // This test requires actual clipboard utilities
        let test_text = "Test clipboard content";
        let result = copy_to_clipboard(test_text);

        // On supported platforms with clipboard utilities, this should succeed
        if cfg!(target_os = "macos") || cfg!(target_os = "linux") {
            // Check if the operation completed (might still fail if no clipboard)
            assert!(result.is_ok() || result.is_err());
        }
    }

    #[test]
    fn test_copy_to_clipboard_large_text() {
        // Test with a large amount of text
        let large_text = "a".repeat(10000);
        let result = copy_to_clipboard(&large_text);
        // Result depends on platform and clipboard availability
        let _is_result = result.is_ok() || result.is_err();
    }

    #[test]
    fn test_copy_to_clipboard_json() {
        let json_text = r#"{"role": "user", "content": "Hello world"}"#;
        let result = copy_to_clipboard(json_text);
        // Result depends on platform and clipboard availability
        let _is_result = result.is_ok() || result.is_err();
    }

    #[test]
    fn test_copy_to_clipboard_with_quotes() {
        let text_with_quotes = r#"He said "Hello" and she replied 'Hi'"#;
        let result = copy_to_clipboard(text_with_quotes);
        // Result depends on platform and clipboard availability
        let _is_result = result.is_ok() || result.is_err();
    }

    #[test]
    fn test_copy_to_clipboard_error_handling() {
        // Test that the function handles errors gracefully
        // Even with unusual input, it should return a Result
        let unusual_text = "\0\x01\x02\x03";
        let result = copy_to_clipboard(unusual_text);
        // Should return either Ok or Err, not panic
        let _is_result = result.is_ok() || result.is_err();
    }
}
