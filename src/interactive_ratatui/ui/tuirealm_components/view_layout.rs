use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

/// ViewLayout helper for consistent UI layout in tui-realm components
/// This is not a component itself, but a helper for other components
pub struct ViewLayout {
    title: String,
    subtitle: Option<String>,
    show_status_bar: bool,
    status_text: Option<String>,
}

impl ViewLayout {
    pub fn new(title: String) -> Self {
        Self {
            title,
            subtitle: None,
            show_status_bar: true,
            status_text: None,
        }
    }

    pub fn with_subtitle(mut self, subtitle: String) -> Self {
        self.subtitle = Some(subtitle);
        self
    }

    pub fn with_status_bar(mut self, show: bool) -> Self {
        self.show_status_bar = show;
        self
    }

    pub fn with_status_text(mut self, text: String) -> Self {
        self.status_text = Some(text);
        self
    }

    pub fn render<F>(&self, f: &mut Frame, area: Rect, render_content: F)
    where
        F: FnOnce(&mut Frame, Rect),
    {
        let constraints = if self.show_status_bar {
            vec![
                Constraint::Length(3), // Title bar
                Constraint::Min(0),    // Content
                Constraint::Length(2), // Status bar
            ]
        } else {
            vec![
                Constraint::Length(3), // Title bar
                Constraint::Min(0),    // Content
            ]
        };

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .split(area);

        // Render title bar
        self.render_title_bar(f, chunks[0]);

        // Render content (delegate to caller)
        render_content(f, chunks[1]);

        // Render status bar if enabled
        if self.show_status_bar && chunks.len() > 2 {
            self.render_status_bar(f, chunks[2]);
        }
    }

    fn render_title_bar(&self, f: &mut Frame, area: Rect) {
        let mut title_lines = vec![Line::from(vec![Span::styled(
            &self.title,
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )])];

        if let Some(ref subtitle) = self.subtitle {
            title_lines.push(Line::from(vec![
                Span::styled("", Style::default().fg(Color::DarkGray)),
                Span::raw(subtitle),
            ]));
        }

        let title_block = Paragraph::new(title_lines)
            .block(Block::default().borders(Borders::BOTTOM))
            .alignment(ratatui::layout::Alignment::Left);

        f.render_widget(title_block, area);
    }

    fn render_status_bar(&self, f: &mut Frame, area: Rect) {
        let status_text = self
            .status_text
            .as_deref()
            .unwrap_or("↑/↓ or j/k: Navigate | Enter: Select | Esc: Back | ?: Help");

        let status_bar = Paragraph::new(status_text)
            .style(Style::default().fg(Color::DarkGray))
            .alignment(ratatui::layout::Alignment::Center);

        f.render_widget(status_bar, area);
    }

    /// Split area into header, content, and optional footer
    pub fn split_area(area: Rect, has_footer: bool) -> (Rect, Rect, Option<Rect>) {
        let constraints = if has_footer {
            vec![
                Constraint::Length(3), // Header
                Constraint::Min(0),    // Content
                Constraint::Length(2), // Footer
            ]
        } else {
            vec![
                Constraint::Length(3), // Header
                Constraint::Min(0),    // Content
            ]
        };

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .split(area);

        if has_footer && chunks.len() > 2 {
            (chunks[0], chunks[1], Some(chunks[2]))
        } else {
            (chunks[0], chunks[1], None)
        }
    }
}

/// Helper struct for consistent color scheme
pub struct ColorScheme;

impl ColorScheme {
    pub const PRIMARY: Color = Color::Cyan;
    pub const SECONDARY: Color = Color::Yellow;
    pub const ACCENT: Color = Color::Magenta;
    pub const TEXT: Color = Color::White;
    pub const TEXT_DIM: Color = Color::DarkGray;
    pub const BACKGROUND: Color = Color::Black;
    pub const SELECTION: Color = Color::DarkGray;
    pub const SUCCESS: Color = Color::Green;
    pub const WARNING: Color = Color::Yellow;
    pub const ERROR: Color = Color::Red;
}

/// Helper struct for consistent styling
pub struct Styles;

impl Styles {
    pub fn title() -> Style {
        Style::default()
            .fg(ColorScheme::PRIMARY)
            .add_modifier(Modifier::BOLD)
    }

    pub fn subtitle() -> Style {
        Style::default().fg(ColorScheme::TEXT_DIM)
    }

    pub fn label() -> Style {
        Style::default().fg(ColorScheme::SECONDARY)
    }

    pub fn selected() -> Style {
        Style::default()
            .bg(ColorScheme::SELECTION)
            .add_modifier(Modifier::BOLD)
    }

    pub fn normal() -> Style {
        Style::default().fg(ColorScheme::TEXT)
    }

    pub fn dimmed() -> Style {
        Style::default().fg(ColorScheme::TEXT_DIM)
    }

    pub fn action_key() -> Style {
        Style::default().fg(ColorScheme::SECONDARY)
    }

    pub fn action_description() -> Style {
        Style::default().fg(ColorScheme::TEXT)
    }

    pub fn success() -> Style {
        Style::default()
            .fg(ColorScheme::SUCCESS)
            .add_modifier(Modifier::BOLD)
    }

    pub fn warning() -> Style {
        Style::default()
            .fg(ColorScheme::WARNING)
            .add_modifier(Modifier::BOLD)
    }

    pub fn error() -> Style {
        Style::default()
            .fg(ColorScheme::ERROR)
            .add_modifier(Modifier::BOLD)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let title_style = Styles::title();
        assert_eq!(title_style.fg, Some(ColorScheme::PRIMARY));
        assert!(title_style.add_modifier.contains(Modifier::BOLD));

        let subtitle_style = Styles::subtitle();
        assert_eq!(subtitle_style.fg, Some(ColorScheme::TEXT_DIM));

        let selected_style = Styles::selected();
        assert_eq!(selected_style.bg, Some(ColorScheme::SELECTION));
        assert!(selected_style.add_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn test_view_layout_builder() {
        let layout = ViewLayout::new("Test Title".to_string())
            .with_subtitle("Test Subtitle".to_string())
            .with_status_bar(false)
            .with_status_text("Custom Status".to_string());

        assert_eq!(layout.title, "Test Title");
        assert_eq!(layout.subtitle, Some("Test Subtitle".to_string()));
        assert!(!layout.show_status_bar);
        assert_eq!(layout.status_text, Some("Custom Status".to_string()));
    }

    #[test]
    fn test_split_area() {
        let area = Rect::new(0, 0, 100, 100);
        
        // With footer
        let (header, content, footer) = ViewLayout::split_area(area, true);
        assert_eq!(header.height, 3);
        assert!(content.height > 0);
        assert_eq!(footer.unwrap().height, 2);

        // Without footer
        let (header, content, footer) = ViewLayout::split_area(area, false);
        assert_eq!(header.height, 3);
        assert!(content.height > 0);
        assert!(footer.is_none());
    }
}