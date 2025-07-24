use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
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
