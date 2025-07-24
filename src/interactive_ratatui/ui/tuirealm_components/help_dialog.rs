use tuirealm::command::{Cmd, CmdResult};
use tuirealm::event::KeyEvent;
use tuirealm::props::{AttrValue, Attribute, Props};
use tuirealm::{Component, Event, MockComponent, NoUserEvent, State};

use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

use crate::interactive_ratatui::ui::tuirealm_components::messages::AppMessage;

/// Help dialog component for tui-realm
pub struct HelpDialog {
    props: Props,
}

impl Default for HelpDialog {
    fn default() -> Self {
        Self::new()
    }
}

impl HelpDialog {
    pub fn new() -> Self {
        Self {
            props: Props::default(),
        }
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

impl MockComponent for HelpDialog {
    fn view(&mut self, frame: &mut Frame, area: Rect) {
        let help_text = Self::get_help_text();

        // Calculate dimensions for the help dialog
        let width = 85.min(area.width - 4);
        let height = (help_text.len() as u16 + 4).min(area.height - 4);

        // Center the dialog
        let x = (area.width - width) / 2;
        let y = (area.height - height) / 2;

        let dialog_area = Rect::new(x, y, width, height);

        // Clear the area behind the dialog
        frame.render_widget(Clear, dialog_area);

        let help = Paragraph::new(help_text)
            .block(
                Block::default()
                    .title(" Help ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Cyan)),
            )
            .style(Style::default().fg(Color::White))
            .alignment(Alignment::Left);

        frame.render_widget(help, dialog_area);
    }

    fn query(&self, attr: Attribute) -> Option<AttrValue> {
        self.props.get(attr)
    }

    fn attr(&mut self, attr: Attribute, value: AttrValue) {
        self.props.set(attr, value);
    }

    fn state(&self) -> State {
        State::None
    }

    fn perform(&mut self, _cmd: Cmd) -> CmdResult {
        // No commands are handled by the help dialog
        CmdResult::None
    }
}

impl Component<AppMessage, NoUserEvent> for HelpDialog {
    fn on(&mut self, ev: Event<NoUserEvent>) -> Option<AppMessage> {
        match ev {
            Event::Keyboard(KeyEvent { .. }) => {
                // Any key closes the help dialog
                Some(AppMessage::ExitHelp)
            }
            _ => None,
        }
    }
}