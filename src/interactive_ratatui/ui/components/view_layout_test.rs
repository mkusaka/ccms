#[cfg(test)]
mod tests {
    use super::super::view_layout::{ColorScheme, Styles, ViewLayout};
    use ratatui::{
        Terminal,
        backend::TestBackend,
        buffer::Buffer,
        style::{Color, Modifier},
    };

    #[test]
    fn test_view_layout_basic() {
        let backend = TestBackend::new(80, 20);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|f| {
                let layout = ViewLayout::new("Test Title".to_string());
                layout.render(f, f.area(), |_f, area| {
                    // Just ensure content area is provided
                    assert!(area.height > 0);
                    assert!(area.width > 0);
                });
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        // Verify title is rendered
        assert!(buffer_contains_text(buffer, "Test Title"));
        // Verify default status text is rendered
        assert!(buffer_contains_text(buffer, "Navigate"));
    }

    #[test]
    fn test_view_layout_with_subtitle() {
        let backend = TestBackend::new(80, 20);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|f| {
                let layout = ViewLayout::new("Test Title".to_string())
                    .with_subtitle("Test Subtitle".to_string());
                layout.render(f, f.area(), |_f, area| {
                    assert!(area.height > 0);
                });
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        assert!(buffer_contains_text(buffer, "Test Title"));
        assert!(buffer_contains_text(buffer, "Test Subtitle"));
    }

    #[test]
    fn test_view_layout_with_custom_status() {
        let backend = TestBackend::new(80, 20);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|f| {
                let layout = ViewLayout::new("Test Title".to_string())
                    .with_status_text("Custom Status".to_string());
                layout.render(f, f.area(), |_f, area| {
                    assert!(area.height > 0);
                });
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        assert!(buffer_contains_text(buffer, "Custom Status"));
    }

    #[test]
    fn test_view_layout_without_status_bar() {
        let backend = TestBackend::new(80, 20);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|f| {
                let layout = ViewLayout::new("Test Title".to_string()).with_status_bar(false);
                let full_area = f.area();
                layout.render(f, full_area, |_f, area| {
                    // Without status bar, content area should be larger
                    // Title bar height is now 2 (title + bottom border)
                    assert_eq!(area.height, full_area.height - 2); // Only title bar
                });
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        assert!(buffer_contains_text(buffer, "Test Title"));
        // Should not contain default status text
        assert!(!buffer_contains_text(buffer, "Navigate"));
    }

    #[test]
    fn test_color_scheme_constants() {
        assert_eq!(ColorScheme::PRIMARY, Color::Cyan);
        assert_eq!(ColorScheme::SECONDARY, Color::Yellow);
        assert_eq!(ColorScheme::ACCENT, Color::Magenta);
        assert_eq!(ColorScheme::TEXT, Color::White);
        assert_eq!(ColorScheme::TEXT_DIM, Color::DarkGray);
        assert_eq!(ColorScheme::BACKGROUND, Color::Black);
        assert_eq!(ColorScheme::SELECTION, Color::DarkGray);
        assert_eq!(ColorScheme::SUCCESS, Color::Green);
        assert_eq!(ColorScheme::WARNING, Color::Yellow);
        assert_eq!(ColorScheme::ERROR, Color::Red);
    }

    #[test]
    fn test_styles() {
        // Test title style
        let title_style = Styles::title();
        assert_eq!(title_style.fg, Some(ColorScheme::PRIMARY));
        assert!(title_style.add_modifier.contains(Modifier::BOLD));

        // Test subtitle style
        let subtitle_style = Styles::subtitle();
        assert_eq!(subtitle_style.fg, Some(ColorScheme::TEXT_DIM));

        // Test label style
        let label_style = Styles::label();
        assert_eq!(label_style.fg, Some(ColorScheme::SECONDARY));

        // Test selected style
        let selected_style = Styles::selected();
        assert_eq!(selected_style.bg, Some(ColorScheme::SELECTION));
        assert!(selected_style.add_modifier.contains(Modifier::BOLD));

        // Test normal style
        let normal_style = Styles::normal();
        assert_eq!(normal_style.fg, Some(ColorScheme::TEXT));

        // Test dimmed style
        let dimmed_style = Styles::dimmed();
        assert_eq!(dimmed_style.fg, Some(ColorScheme::TEXT_DIM));

        // Test action key style
        let action_key_style = Styles::action_key();
        assert_eq!(action_key_style.fg, Some(ColorScheme::SECONDARY));

        // Test action description style
        let action_desc_style = Styles::action_description();
        assert_eq!(action_desc_style.fg, Some(ColorScheme::TEXT));

        // Test success style
        let success_style = Styles::success();
        assert_eq!(success_style.fg, Some(ColorScheme::SUCCESS));
        assert!(success_style.add_modifier.contains(Modifier::BOLD));

        // Test warning style
        let warning_style = Styles::warning();
        assert_eq!(warning_style.fg, Some(ColorScheme::WARNING));
        assert!(warning_style.add_modifier.contains(Modifier::BOLD));

        // Test error style
        let error_style = Styles::error();
        assert_eq!(error_style.fg, Some(ColorScheme::ERROR));
        assert!(error_style.add_modifier.contains(Modifier::BOLD));
    }

    // Helper function to check if buffer contains text
    fn buffer_contains_text(buffer: &Buffer, text: &str) -> bool {
        let content = buffer
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        content.contains(text)
    }
}
