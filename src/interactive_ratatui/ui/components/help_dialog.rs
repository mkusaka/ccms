use crate::interactive_ratatui::ui::components::Component;
use crate::interactive_ratatui::ui::events::Message;
use crossterm::event::KeyEvent;
use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};

pub struct HelpDialog;

impl HelpDialog {
    pub fn new() -> Self {
        Self
    }

    fn get_help_text() -> Vec<Line<'static>> {
        vec![
            Line::from(vec![Span::styled(
                "Claude Session Message Search - Interactive Mode",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from(""),
            Line::from(vec![Span::styled(
                "Search Mode:",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from("  ↑/↓         - Navigate results"),
            Line::from("  Enter       - View result details"),
            Line::from("  s           - View full session"),
            Line::from("  Tab         - Toggle role filter (user/assistant/system)"),
            Line::from("  Esc         - Quit"),
            Line::from("  ?           - Show this help"),
            Line::from(""),
            Line::from(vec![Span::styled(
                "Result Detail Mode:",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from("  ↑/↓         - Scroll content"),
            Line::from("  s           - View full session"),
            Line::from("  c           - Copy message content to clipboard"),
            Line::from("  C           - Copy message as JSON to clipboard"),
            Line::from("  Backspace   - Back to search results"),
            Line::from("  Esc         - Back to search results"),
            Line::from(""),
            Line::from(vec![Span::styled(
                "Session Viewer Mode:",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from("  ↑/↓         - Navigate messages"),
            Line::from("  /           - Search within session"),
            Line::from("  c           - Copy selected message to clipboard"),
            Line::from("  C           - Copy all filtered messages to clipboard"),
            Line::from("  o           - Toggle sort order (ascending/descending/original)"),
            Line::from("  Backspace   - Back to search results (or clear search)"),
            Line::from("  Esc         - Back to search results"),
            Line::from(""),
            Line::from(vec![Span::styled(
                "Query Syntax:",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from("  word        - Search for 'word'"),
            Line::from("  \"phrase\"    - Search for exact phrase"),
            Line::from("  term1 AND term2 - Both terms must match"),
            Line::from("  term1 OR term2  - Either term must match"),
            Line::from("  NOT term    - Exclude matches"),
            Line::from("  /regex/     - Regular expression search"),
            Line::from(""),
            Line::from("Press any key to close this help..."),
        ]
    }
}

impl Component for HelpDialog {
    fn render(&mut self, f: &mut Frame, area: Rect) {
        let help_text = Self::get_help_text();

        // Calculate dimensions for the help dialog
        let width = 80.min(area.width - 4);
        let height = (help_text.len() as u16 + 4).min(area.height - 4);

        // Center the dialog
        let x = (area.width - width) / 2;
        let y = (area.height - height) / 2;

        let dialog_area = Rect::new(x, y, width, height);

        // Clear the area behind the dialog
        f.render_widget(Clear, dialog_area);

        let help = Paragraph::new(help_text)
            .block(
                Block::default()
                    .title(" Help ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Cyan)),
            )
            .style(Style::default().fg(Color::White))
            .alignment(Alignment::Left);

        f.render_widget(help, dialog_area);
    }

    fn handle_key(&mut self, _key: KeyEvent) -> Option<Message> {
        // Any key closes the help dialog
        Some(Message::CloseHelp)
    }
}
