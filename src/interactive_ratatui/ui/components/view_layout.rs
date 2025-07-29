use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
};

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

    fn calculate_title_bar_height(&self, width: u16) -> u16 {
        // Create a temporary paragraph to measure its height
        let mut lines = vec![Line::from(vec![Span::styled(
            &self.title,
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )])];

        if let Some(ref subtitle) = self.subtitle {
            for line in subtitle.lines() {
                lines.push(Line::from(vec![
                    Span::styled("", Style::default().fg(Color::DarkGray)),
                    Span::raw(line),
                ]));
            }
        }

        let paragraph = Paragraph::new(lines).wrap(Wrap { trim: true });

        // Use ratatui's line_count method to get the actual height needed
        let content_height = paragraph.line_count(width.saturating_sub(2)) as u16;

        // Add 1 for the bottom border
        content_height + 1
    }

    fn calculate_status_bar_height(&self, width: u16) -> u16 {
        let status_text = self
            .status_text
            .as_deref()
            .unwrap_or("↑/↓: Navigate | Enter: Select | Esc: Back | ?: Help");

        let paragraph = Paragraph::new(status_text).wrap(Wrap { trim: true });

        // Use ratatui's line_count method to get the actual height needed
        // Status bar doesn't have borders, so width is full
        let content_height = paragraph.line_count(width) as u16;

        // Ensure at least 1 line, max 3 lines for status bar
        content_height.clamp(1, 3)
    }

    pub fn render<F>(&self, f: &mut Frame, area: Rect, render_content: F)
    where
        F: FnOnce(&mut Frame, Rect),
    {
        // Calculate title bar height based on content and available width
        let title_bar_height = self.calculate_title_bar_height(area.width);

        let constraints = if self.show_status_bar {
            // Calculate status bar height based on content and available width
            let status_bar_height = self.calculate_status_bar_height(area.width);
            vec![
                Constraint::Length(title_bar_height),  // Title bar
                Constraint::Min(0),                    // Content
                Constraint::Length(status_bar_height), // Status bar
            ]
        } else {
            vec![
                Constraint::Length(title_bar_height), // Title bar
                Constraint::Min(0),                   // Content
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
            // Split subtitle by newlines to support multi-line subtitles
            for line in subtitle.lines() {
                title_lines.push(Line::from(vec![
                    Span::styled("", Style::default().fg(Color::DarkGray)),
                    Span::raw(line),
                ]));
            }
        }

        let title_block = Paragraph::new(title_lines)
            .block(Block::default().borders(Borders::BOTTOM))
            .alignment(ratatui::layout::Alignment::Left)
            .wrap(Wrap { trim: true });

        f.render_widget(title_block, area);
    }

    fn render_status_bar(&self, f: &mut Frame, area: Rect) {
        let status_text = self
            .status_text
            .as_deref()
            .unwrap_or("↑/↓: Navigate | Enter: Select | Esc: Back | ?: Help");

        let status_bar = Paragraph::new(status_text)
            .style(Style::default().fg(Color::DarkGray))
            .alignment(ratatui::layout::Alignment::Center)
            .wrap(Wrap { trim: true });

        f.render_widget(status_bar, area);
    }
}

// Helper struct for consistent color scheme
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

// Helper struct for consistent styling
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
