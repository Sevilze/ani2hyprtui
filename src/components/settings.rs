use super::Component;
use crate::config::AppConfig;
use crate::event::AppMsg;
use crate::widgets::common::focused_block;
use crate::widgets::theme::{ThemeType, get_current_theme_type, get_theme, set_theme};
use crossterm::event::KeyCode;
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{List, ListItem, ListState, Paragraph, StatefulWidget, Widget},
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
    pub fn select_theme(&mut self, theme_type: ThemeType) {
        if let Some(idx) = self.themes.iter().position(|t| t == &theme_type) {
            self.selected_index = idx;
            self.list_state.select(Some(idx));
        }
    }

    pub fn apply_theme(&mut self) {
        if self.selected_index < self.themes.len() {
            let chosen = self.themes[self.selected_index];
            set_theme(chosen);
            AppConfig::save_theme(chosen.name());
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
                    } else {
                        self.selected_index = self.themes.len().saturating_sub(1);
                    }
                    self.list_state.select(Some(self.selected_index));
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if self.selected_index < self.themes.len().saturating_sub(1) {
                        self.selected_index += 1;
                    } else {
                        self.selected_index = 0;
                    }
                    self.list_state.select(Some(self.selected_index));
                }
                KeyCode::Enter | KeyCode::Char(' ') => {
                    self.apply_theme();
                }
                KeyCode::Right | KeyCode::Char('l') => {
                    let current = self.themes[self.selected_index];
                    let next = current.next();
                    if let Some(idx) = self.themes.iter().position(|t| *t == next) {
                        self.selected_index = idx;
                        self.list_state.select(Some(self.selected_index));
                        self.apply_theme();
                    }
                }
                KeyCode::Left | KeyCode::Char('h') => {
                    let current = self.themes[self.selected_index];
                    let prev = current.prev();
                    if let Some(idx) = self.themes.iter().position(|t| *t == prev) {
                        self.selected_index = idx;
                        self.list_state.select(Some(self.selected_index));
                        self.apply_theme();
                    }
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

        if inner.height == 0 || inner.width == 0 {
            return;
        }

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(3),    // Theme list
                Constraint::Length(1), // Help / keybindings hint
            ])
            .split(inner);

        let list_area = chunks[0];
        let current_theme = get_current_theme_type();
        let max_name_len = list_area.width.saturating_sub(4) as usize;

        let items: Vec<ListItem> = self
            .themes
            .iter()
            .enumerate()
            .map(|(i, t)| {
                let is_current = t == &current_theme;
                let marker = if is_current { "●" } else { " " };
                let name = t.name();
                let display_name = if name.chars().count() > max_name_len {
                    let truncated: String =
                        name.chars().take(max_name_len.saturating_sub(1)).collect();
                    format!("{}…", truncated)
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

        let list = List::new(items);
        StatefulWidget::render(list, list_area, buf, &mut self.list_state);

        let help_line = Line::from(vec![Span::styled(
            "Enter: Apply & Save | ←→: Cycle",
            Style::default().fg(theme.text_secondary),
        )]);
        let help_para = Paragraph::new(help_line);
        help_para.render(chunks[1], buf);
    }
}
