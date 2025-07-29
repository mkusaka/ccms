use crate::interactive_ratatui::constants::*;
use crate::interactive_ratatui::ui::components::Component;
use crate::interactive_ratatui::ui::events::Message;
use crossterm::event::KeyEvent;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};

#[derive(Default)]
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
                "Global Shortcuts:",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from("  Alt+←       - Navigate back through history"),
            Line::from("  Alt+→       - Navigate forward through history"),
            Line::from("  Ctrl+T      - Toggle message truncation"),
            Line::from("  ?           - Show this help"),
            Line::from(""),
            Line::from(vec![Span::styled(
                "Search Mode:",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from("  ↑/↓         - Navigate results"),
            Line::from("  Ctrl+u/d    - Half-page scrolling (up/down)"),
            Line::from("  Enter       - View result details"),
            Line::from("  Ctrl+S      - Jump directly to session viewer"),
            Line::from("  Tab         - Toggle role filter (user/assistant/system)"),
            Line::from("  Esc         - Quit"),
            Line::from(""),
            Line::from(vec![Span::styled(
                "Text Editing Shortcuts (Search & Session Viewer):",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from("  Ctrl+A      - Move cursor to beginning of line"),
            Line::from("  Ctrl+E      - Move cursor to end of line"),
            Line::from("  Ctrl+B      - Move cursor backward one character"),
            Line::from("  Ctrl+F      - Move cursor forward one character"),
            Line::from("  Alt+B       - Move cursor backward one word"),
            Line::from("  Alt+F       - Move cursor forward one word"),
            Line::from("  Ctrl+W      - Delete word before cursor"),
            Line::from("  Ctrl+U      - Delete from cursor to beginning of line"),
            Line::from("  Ctrl+K      - Delete from cursor to end of line"),
            Line::from("  Ctrl+D      - Delete character under cursor"),
            Line::from("  Ctrl+H      - Delete character before cursor"),
            Line::from(""),
            Line::from(vec![Span::styled(
                "Result Detail Mode:",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from("  ↑/↓         - Scroll content"),
            Line::from("  Ctrl+u/d    - Half-page scrolling (up/down)"),
            Line::from("  Ctrl+S      - Jump to session viewer"),
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
            Line::from("  Ctrl+u/d    - Half-page scrolling (up/down)"),
            Line::from("  Tab         - Toggle role filter (user/assistant/system)"),
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

        // Calculate dialog dimensions using constraints
        let dialog_width = HELP_DIALOG_MAX_WIDTH.min(area.width.saturating_sub(HELP_DIALOG_MARGIN));
        let dialog_height = (help_text.len() as u16 + HELP_DIALOG_MARGIN)
            .min(area.height.saturating_sub(HELP_DIALOG_MARGIN));

        // Create centered layout using ratatui's Layout system
        let vertical_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length((area.height - dialog_height) / 2),
                Constraint::Length(dialog_height),
                Constraint::Min(0),
            ])
            .split(area);

        let horizontal_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length((area.width - dialog_width) / 2),
                Constraint::Length(dialog_width),
                Constraint::Min(0),
            ])
            .split(vertical_chunks[1]);

        let dialog_area = horizontal_chunks[1];

        // Clear the background area
        f.render_widget(Clear, dialog_area);

        let help = Paragraph::new(help_text)
            .block(
                Block::default()
                    .title(" Help ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Cyan))
                    .style(Style::default().bg(Color::Black)),
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
