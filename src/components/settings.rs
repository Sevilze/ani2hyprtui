use super::Component;
use crate::event::AppMsg;
use crate::widgets::common::focused_block;
use crate::widgets::theme::{ThemeType, get_current_theme_type, get_theme, set_theme};
use crossterm::event::KeyCode;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{List, ListItem, ListState, Paragraph, StatefulWidget, Widget, Wrap},
};

pub struct SettingsState {
    pub themes: Vec<ThemeType>,
    pub selected_index: usize,
    pub list_state: ListState,
}

impl Default for SettingsState {
    fn default() -> Self {
        let themes = ThemeType::all();
        let current_theme = get_current_theme_type();
        let selected_index = themes.iter().position(|t| t == &current_theme).unwrap_or(0);

        let mut list_state = ListState::default();
        list_state.select(Some(selected_index));

        Self {
            themes,
            selected_index,
            list_state,
        }
    }
}

impl SettingsState {
    pub fn apply_theme(&mut self) {
        if self.selected_index < self.themes.len() {
            set_theme(self.themes[self.selected_index]);
        }
    }
}

impl Component for SettingsState {
    fn update(&mut self, msg: &AppMsg) -> Option<AppMsg> {
        if let AppMsg::Key(key) = msg {
            match key.code {
                KeyCode::Up | KeyCode::Char('k') => {
                    if self.selected_index > 0 {
                        self.selected_index -= 1;
                        self.list_state.select(Some(self.selected_index));
                    } else {
                        self.selected_index = self.themes.len() - 1;
                        self.list_state.select(Some(self.selected_index));
                    }
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if self.selected_index < self.themes.len() - 1 {
                        self.selected_index += 1;
                        self.list_state.select(Some(self.selected_index));
                    } else {
                        self.selected_index = 0;
                        self.list_state.select(Some(self.selected_index));
                    }
                }
                KeyCode::Enter | KeyCode::Char(' ') => {
                    self.apply_theme();
                }
                KeyCode::Right | KeyCode::Char('l') => {
                    // Next theme (circular)
                    self.selected_index = (self.selected_index + 1) % self.themes.len();
                    self.list_state.select(Some(self.selected_index));
                    self.apply_theme();
                }
                KeyCode::Left | KeyCode::Char('h') => {
                    // Previous theme (circular)
                    self.selected_index =
                        (self.selected_index + self.themes.len() - 1) % self.themes.len();
                    self.list_state.select(Some(self.selected_index));
                    self.apply_theme();
                }
                _ => {}
            }
        }
        None
    }

    fn render(&mut self, area: Rect, buf: &mut Buffer, is_focused: bool) {
        let theme = get_theme();
        let block = focused_block("Settings", is_focused);

        let inner = block.inner(area);
        block.render(area, buf);

        // Show title - simplified for narrow width
        let title_text = vec![Line::from(Span::styled(
            "Theme",
            Style::default()
                .fg(theme.text_highlight)
                .add_modifier(Modifier::BOLD),
        ))];

        let title_height = 1u16;

        let title_para = Paragraph::new(title_text);
        let title_area = Rect::new(inner.x, inner.y, inner.width, title_height);
        title_para.render(title_area, buf);

        // Help text area - use 2 lines for narrow width
        let help_height = 2u16;
        let help_area = Rect::new(
            inner.x,
            inner.y + inner.height.saturating_sub(help_height),
            inner.width,
            help_height,
        );

        // List area - between title and help
        let list_area = Rect::new(
            inner.x,
            inner.y + title_height,
            inner.width,
            inner.height.saturating_sub(title_height + help_height),
        );

        let current_theme = get_current_theme_type();

        // Truncate theme names if needed for narrow width
        let max_name_len = inner.width.saturating_sub(4) as usize; // Account for marker and padding

        let items: Vec<ListItem> = self
            .themes
            .iter()
            .enumerate()
            .map(|(i, t)| {
                let is_current = t == &current_theme;
                let marker = if is_current { "●" } else { " " };
                let name = t.name();
                // Truncate long names with ellipsis
                let display_name = if name.len() > max_name_len {
                    format!("{}…", &name[..max_name_len.saturating_sub(1)])
                } else {
                    name.to_string()
                };
                let text = format!("{} {}", marker, display_name);

                let style = if i == self.selected_index {
                    Style::default()
                        .fg(theme.background)
                        .bg(theme.text_highlight)
                        .add_modifier(Modifier::BOLD)
                } else if is_current {
                    Style::default()
                        .fg(theme.status_completed)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme.text_primary)
                };

                ListItem::new(text).style(style)
            })
            .collect();

        let list = List::new(items).highlight_style(
            Style::default()
                .fg(theme.background)
                .bg(theme.text_highlight)
                .add_modifier(Modifier::BOLD),
        );

        StatefulWidget::render(list, list_area, buf, &mut self.list_state);

        // Help text - split into two lines for narrow width
        let help_lines = vec![
            Line::from(Span::styled(
                "↑↓: Select  Enter: Apply",
                Style::default().fg(theme.text_secondary),
            )),
            Line::from(Span::styled(
                "←→: Quick Switch",
                Style::default().fg(theme.text_secondary),
            )),
        ];

        let help_para = Paragraph::new(help_lines).wrap(Wrap { trim: true });
        help_para.render(help_area, buf);
    }
}
