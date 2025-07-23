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

    // Helper function to check if clipboard commands are available
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    fn clipboard_available() -> bool {
        #[cfg(target_os = "macos")]
        let cmd = "pbcopy";
        #[cfg(target_os = "linux")]
        let cmd = "xclip";

        std::process::Command::new("which")
            .arg(cmd)
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    fn clipboard_available() -> bool {
        false
    }

    #[test]
    fn test_copy_to_clipboard_empty_string() {
        let result = copy_to_clipboard("");
        if clipboard_available() {
            // Should succeed with clipboard available
            assert!(result.is_ok());
        } else {
            // Platform check
            #[cfg(not(any(target_os = "macos", target_os = "linux")))]
            assert!(result.is_err());
        }
    }

    #[test]
    fn test_copy_to_clipboard_simple_text() {
        let result = copy_to_clipboard("Hello, world!");
        if clipboard_available() {
            assert!(result.is_ok());
        } else {
            #[cfg(not(any(target_os = "macos", target_os = "linux")))]
            assert!(result.is_err());
        }
    }

    #[test]
    fn test_copy_to_clipboard_unicode() {
        let result = copy_to_clipboard("こんにちは世界 🌍");
        if clipboard_available() {
            assert!(result.is_ok());
        } else {
            #[cfg(not(any(target_os = "macos", target_os = "linux")))]
            assert!(result.is_err());
        }
    }

    #[test]
    fn test_copy_to_clipboard_multiline() {
        let text = "Line 1\nLine 2\nLine 3";
        let result = copy_to_clipboard(text);
        if clipboard_available() {
            assert!(result.is_ok());
        } else {
            #[cfg(not(any(target_os = "macos", target_os = "linux")))]
            assert!(result.is_err());
        }
    }

    #[test]
    fn test_copy_to_clipboard_special_characters() {
        let text = "Special chars: @#$%^&*()_+-={}[]|\\\":;<>?,./~`";
        let result = copy_to_clipboard(text);
        if clipboard_available() {
            assert!(result.is_ok());
        } else {
            #[cfg(not(any(target_os = "macos", target_os = "linux")))]
            assert!(result.is_err());
        }
    }

    #[test]
    fn test_copy_to_clipboard_large_text() {
        let large_text = "a".repeat(10000);
        let result = copy_to_clipboard(&large_text);
        if clipboard_available() {
            assert!(result.is_ok());
        } else {
            #[cfg(not(any(target_os = "macos", target_os = "linux")))]
            assert!(result.is_err());
        }
    }

    #[test]
    fn test_copy_to_clipboard_json() {
        let json_text = r#"{"role": "user", "content": "Hello world"}"#;
        let result = copy_to_clipboard(json_text);
        if clipboard_available() {
            assert!(result.is_ok());
        } else {
            #[cfg(not(any(target_os = "macos", target_os = "linux")))]
            assert!(result.is_err());
        }
    }

    #[test]
    fn test_copy_to_clipboard_with_quotes() {
        let text_with_quotes = r#"He said "Hello" and she replied 'Hi'"#;
        let result = copy_to_clipboard(text_with_quotes);
        if clipboard_available() {
            assert!(result.is_ok());
        } else {
            #[cfg(not(any(target_os = "macos", target_os = "linux")))]
            assert!(result.is_err());
        }
    }

    #[test]
    fn test_copy_to_clipboard_error_message() {
        #[cfg(not(any(target_os = "macos", target_os = "linux")))]
        {
            let result = copy_to_clipboard("test");
            assert!(result.is_err());
            assert_eq!(
                result.unwrap_err().to_string(),
                "Clipboard not supported on this platform"
            );
        }
    }

    #[test]
    fn test_copy_to_clipboard_returns_result() {
        // This test ensures the function always returns a Result type
        let result = copy_to_clipboard("test");
        // The function should return Result<()>
        match result {
            Ok(()) => {
                // Success case
                assert!(clipboard_available());
            }
            Err(_) => {
                // Error case - either no clipboard or unsupported platform
                assert!(
                    !clipboard_available()
                        || cfg!(not(any(target_os = "macos", target_os = "linux")))
                );
            }
        }
    }
}
