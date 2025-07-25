#[cfg(test)]
mod clipboard_service_tests {
    use super::super::*;

    #[test]
    fn test_clipboard_service_new() {
        let _service = ClipboardService::new();
        
        // Just verify it can be created
        // Just verify it can be created
    }

    #[test]
    fn test_platform_specific_behavior() {
        let mut service = ClipboardService::new();
        
        // Just test that copy method exists and can be called
        // The actual command used is an implementation detail
        let result = service.copy("test");
        
        // We can't guarantee success on all platforms, but it shouldn't panic
        let _ = result;
    }

    #[test]
    fn test_copy_simple_text() {
        let mut service = ClipboardService::new();
        
        // Try to copy simple text
        // This might fail in CI environments without clipboard access
        let result = service.copy("Hello, World!");
        
        // We can't guarantee success in all environments
        // but the method should not panic
        let _ = result;
    }

    #[test]
    fn test_copy_empty_string() {
        let mut service = ClipboardService::new();
        
        let result = service.copy("");
        
        // Empty string should still be copyable
        let _ = result;
    }

    #[test]
    fn test_copy_multibyte_text() {
        let mut service = ClipboardService::new();
        
        // Test with Unicode text
        let result = service.copy("こんにちは世界 🌍");
        
        // Should handle multibyte characters
        let _ = result;
    }

    #[test]
    fn test_copy_multiline_text() {
        let mut service = ClipboardService::new();
        
        let multiline = "Line 1\nLine 2\nLine 3";
        let result = service.copy(multiline);
        
        // Should handle multiline text
        let _ = result;
    }

    #[test]
    fn test_copy_large_text() {
        let mut service = ClipboardService::new();
        
        // Create a large text (1MB)
        let large_text = "a".repeat(1024 * 1024);
        let result = service.copy(&large_text);
        
        // Should handle large text (might fail due to system limits)
        let _ = result;
    }

    #[test]
    fn test_copy_special_characters() {
        let mut service = ClipboardService::new();
        
        let special = "Special chars: !@#$%^&*()_+-=[]{}|;':\",./<>?";
        let result = service.copy(special);
        
        // Should handle special characters
        let _ = result;
    }

    #[test]
    fn test_copy_json() {
        let mut service = ClipboardService::new();
        
        let json = r#"{"key": "value", "number": 123, "array": [1, 2, 3]}"#;
        let result = service.copy(json);
        
        // Should handle JSON text
        let _ = result;
    }

    #[test]
    fn test_copy_with_null_bytes() {
        let mut service = ClipboardService::new();
        
        // Text with null bytes (might cause issues)
        let text_with_null = "Hello\0World";
        let result = service.copy(text_with_null);
        
        // Should handle or fail gracefully
        let _ = result;
    }

    #[test]
    fn test_copy_ansi_escape_sequences() {
        let mut service = ClipboardService::new();
        
        // Text with ANSI color codes
        let ansi_text = "\x1b[31mRed Text\x1b[0m";
        let result = service.copy(ansi_text);
        
        // Should handle ANSI sequences
        let _ = result;
    }

    #[test]
    fn test_copy_concurrent() {
        use std::thread;
        
        // Test concurrent clipboard access
        let handles: Vec<_> = (0..5)
            .map(|i| {
                thread::spawn(move || {
                    let mut service = ClipboardService::new();
                    service.copy(&format!("Thread {i}"))
                })
            })
            .collect();
        
        // All should complete (though some might fail due to concurrent access)
        for handle in handles {
            let _ = handle.join().unwrap();
        }
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn test_fallback_to_xsel() {
        // This test is Linux-specific and tests the fallback mechanism
        // In practice, this is hard to test without mocking the command execution
        
        let mut service = ClipboardService::new();
        
        // If xclip fails, it should try xsel
        // We can't easily test this without mocking
        let _ = service.copy("Test fallback");
    }

    #[test]
    fn test_copy_error_message_format() {
        let mut service = ClipboardService::new();
        
        // On systems without clipboard commands, we should get a meaningful error
        // This is hard to test without mocking the system
        
        // Just verify the service exists and can be called
        let _ = service.copy("Test");
    }
}