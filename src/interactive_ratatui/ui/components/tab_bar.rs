use crate::interactive_ratatui::domain::models::SearchTab;
use crate::interactive_ratatui::ui::components::Component;
use crate::interactive_ratatui::ui::events::Message;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::{Block, Borders},
};

#[derive(Default)]
pub struct TabBar {
    current_tab: SearchTab,
}

impl TabBar {
    pub fn new() -> Self {
        Self {
            current_tab: SearchTab::Search,
        }
    }

    pub fn set_current_tab(&mut self, tab: SearchTab) {
        self.current_tab = tab;
    }
}

impl Component for TabBar {
    fn render(&mut self, f: &mut Frame, area: Rect) {
        use ratatui::text::{Line, Span};
        use ratatui::widgets::Paragraph;

        // Create tab titles with better visual separation
        let search_tab = if self.current_tab == SearchTab::Search {
            Span::styled(
                " ▸ Search ",
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )
        } else {
            Span::styled(
                "   Search ",
                Style::default().fg(Color::Gray).add_modifier(Modifier::DIM),
            )
        };

        let session_tab = if self.current_tab == SearchTab::SessionList {
            Span::styled(
                " ▸ Session List ",
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )
        } else {
            Span::styled(
                "   Session List ",
                Style::default().fg(Color::Gray).add_modifier(Modifier::DIM),
            )
        };

        // Add separators and create the tab line
        let tab_line = Line::from(vec![
            Span::raw(" "),
            search_tab,
            Span::styled(" │ ", Style::default().fg(Color::DarkGray)),
            session_tab,
            Span::raw(" "),
        ]);

        // Create a block with top and bottom borders for the tab bar
        let tab_block = Block::default()
            .borders(Borders::TOP | Borders::BOTTOM)
            .border_style(Style::default().fg(Color::DarkGray));

        let tabs = Paragraph::new(vec![tab_line])
            .block(tab_block)
            .style(Style::default().bg(Color::Reset));

        f.render_widget(tabs, area);
    }

    fn handle_key(&mut self, key: KeyEvent) -> Option<Message> {
        use crossterm::event::KeyModifiers;

        match key.code {
            // Use Shift+Tab to switch tabs (Tab alone is for role filter in Search tab)
            KeyCode::BackTab => Some(match self.current_tab {
                SearchTab::Search => Message::SwitchToSessionListTab,
                SearchTab::SessionList => Message::SwitchToSearchTab,
            }),
            KeyCode::Left if key.modifiers.contains(KeyModifiers::CONTROL) => {
                if self.current_tab == SearchTab::SessionList {
                    Some(Message::SwitchToSearchTab)
                } else {
                    None
                }
            }
            KeyCode::Right if key.modifiers.contains(KeyModifiers::CONTROL) => {
                if self.current_tab == SearchTab::Search {
                    Some(Message::SwitchToSessionListTab)
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}
