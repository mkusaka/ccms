use tuirealm::command::{Cmd, CmdResult};
use tuirealm::props::{AttrValue, Attribute, Props};
use tuirealm::{Component, Event, MockComponent, NoUserEvent, State, StateValue};

use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::interactive_ratatui::ui::tuirealm_components::messages::AppMessage;
use crate::interactive_ratatui::ui::tuirealm_components::text_input::TextInput;

/// Search bar component for tui-realm
pub struct SearchBar {
    props: Props,
    /// The text input component
    text_input: TextInput,
    /// Whether we are actively searching
    is_searching: bool,
    /// Message to display
    message: Option<String>,
    /// Role filter
    role_filter: Option<String>,
}

impl Default for SearchBar {
    fn default() -> Self {
        Self::new()
    }
}

impl SearchBar {
    pub fn new() -> Self {
        let mut text_input = TextInput::default();
        text_input.attr(
            Attribute::Custom("id"),
            AttrValue::String("search_bar".to_string()),
        );

        Self {
            props: Props::default(),
            text_input,
            is_searching: false,
            message: None,
            role_filter: None,
        }
    }

    pub fn set_query(&mut self, query: String) {
        self.text_input.set_text(query);
    }

    pub fn set_searching(&mut self, is_searching: bool) {
        self.is_searching = is_searching;
    }

    pub fn set_message(&mut self, message: Option<String>) {
        self.message = message;
    }

    pub fn set_role_filter(&mut self, role_filter: Option<String>) {
        self.role_filter = role_filter;
    }

    pub fn get_query(&self) -> &str {
        self.text_input.text()
    }

    pub fn is_searching(&self) -> bool {
        self.is_searching
    }
}

impl MockComponent for SearchBar {
    fn view(&mut self, frame: &mut Frame, area: Rect) {
        let mut title = "Search".to_string();
        if let Some(role) = &self.role_filter {
            title.push_str(&format!(" [role:{role}]"));
        }
        if let Some(msg) = &self.message {
            title.push_str(&format!(" - {msg}"));
        }

        // Get the cursor spans from text input
        let spans = self.text_input.render_cursor_spans();

        let input = Paragraph::new(Line::from(spans))
            .block(Block::default().title(title).borders(Borders::ALL))
            .style(Style::default().fg(Color::Yellow));

        frame.render_widget(input, area);
    }

    fn query(&self, attr: Attribute) -> Option<AttrValue> {
        self.props.get(attr)
    }

    fn attr(&mut self, attr: Attribute, value: AttrValue) {
        self.props.set(attr, value);
    }

    fn state(&self) -> State {
        State::One(StateValue::String(self.text_input.text().to_string()))
    }

    fn perform(&mut self, cmd: Cmd) -> CmdResult {
        // Delegate to text input
        self.text_input.perform(cmd)
    }
}

impl Component<AppMessage, NoUserEvent> for SearchBar {
    fn on(&mut self, ev: Event<NoUserEvent>) -> Option<AppMessage> {
        // Delegate keyboard events to text input
        if let Event::Keyboard(_) = ev {
            self.text_input.on(ev)
        } else {
            None
        }
    }
}

