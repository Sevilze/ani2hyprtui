use super::Component;
use crate::event::AppMsg;
use crate::widgets::common::focused_block;
use crate::widgets::theme::{ThemeType, get_current_theme_type, get_theme, set_theme};
use crossterm::event::KeyCode;
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{List, ListItem, ListState, Paragraph, StatefulWidget, Widget, Wrap},
};

#[derive(PartialEq)]
pub enum SettingsSection {
    Theme,
    Performance,
}

pub struct SettingsState {
    pub themes: Vec<ThemeType>,
    pub selected_index: usize,
    pub list_state: ListState,
    pub active_section: SettingsSection,
    pub thread_count: usize,
    pub max_thread_count: usize,
}

impl Default for SettingsState {
    fn default() -> Self {
        let themes = ThemeType::all();
        let current_theme = get_current_theme_type();
        let selected_index = themes.iter().position(|t| t == &current_theme).unwrap_or(0);

        let mut list_state = ListState::default();
        list_state.select(Some(selected_index));

        let max_thread_count = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4);

        Self {
            themes,
            selected_index,
            list_state,
            active_section: SettingsSection::Theme,
            thread_count: 0,
            max_thread_count,
        }
    }
}

impl SettingsState {
    pub fn apply_theme(&mut self) {
        if self.selected_index < self.themes.len() {
            set_theme(self.themes[self.selected_index]);
        }
    }

    pub fn set_thread_count(&mut self, count: usize) {
        self.thread_count = count;
    }
}

impl Component for SettingsState {
    fn update(&mut self, msg: &AppMsg) -> Option<AppMsg> {
        if let AppMsg::Key(key) = msg {
            match key.code {
                KeyCode::Up | KeyCode::Char('k') => {
                    match self.active_section {
                        SettingsSection::Theme => {
                            if self.selected_index > 0 {
                                self.selected_index -= 1;
                                self.list_state.select(Some(self.selected_index));
                            } else {
                                self.selected_index = self.themes.len() - 1;
                                self.list_state.select(Some(self.selected_index));
                            }
                        }
                        SettingsSection::Performance => {
                            self.active_section = SettingsSection::Theme;
                            self.selected_index = self.themes.len() - 1;
                            self.list_state.select(Some(self.selected_index));
                        }
                    }
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    match self.active_section {
                        SettingsSection::Theme => {
                            if self.selected_index < self.themes.len() - 1 {
                                self.selected_index += 1;
                                self.list_state.select(Some(self.selected_index));
                            } else {
                                self.active_section = SettingsSection::Performance;
                                self.list_state.select(None);
                            }
                        }
                        SettingsSection::Performance => {
                            self.active_section = SettingsSection::Theme;
                            self.selected_index = 0;
                            self.list_state.select(Some(self.selected_index));
                        }
                    }
                }
                KeyCode::Enter | KeyCode::Char(' ') => {
                    if self.active_section == SettingsSection::Theme {
                        self.apply_theme();
                    }
                }
                KeyCode::Right | KeyCode::Char('l') => {
                    match self.active_section {
                        SettingsSection::Theme => {
                            // Next theme (circular)
                            let current = self.themes[self.selected_index];
                            let next = current.next();
                            if let Some(idx) = self.themes.iter().position(|t| *t == next) {
                                self.selected_index = idx;
                                self.list_state.select(Some(self.selected_index));
                                self.apply_theme();
                            }
                        }
                        SettingsSection::Performance => {
                            if self.thread_count < self.max_thread_count {
                                self.thread_count += 1;
                                return Some(AppMsg::ThreadCountChanged(self.thread_count));
                            }
                        }
                    }
                }
                KeyCode::Left | KeyCode::Char('h') => {
                    match self.active_section {
                        SettingsSection::Theme => {
                            // Previous theme (circular)
                            let current = self.themes[self.selected_index];
                            let prev = current.prev();
                            if let Some(idx) = self.themes.iter().position(|t| *t == prev) {
                                self.selected_index = idx;
                                self.list_state.select(Some(self.selected_index));
                                self.apply_theme();
                            }
                        }
                        SettingsSection::Performance => {
                            // Decrease thread count
                            if self.thread_count > 0 {
                                self.thread_count -= 1;
                                return Some(AppMsg::ThreadCountChanged(self.thread_count));
                            }
                        }
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

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(5),    // Theme list
                Constraint::Length(1), // Separator
                Constraint::Length(4), // Performance settings
            ])
            .split(inner);

        let theme_area = chunks[0];
        let title_text = vec![Line::from(Span::styled(
            "Theme",
            Style::default()
                .fg(if self.active_section == SettingsSection::Theme {
                    theme.text_highlight
                } else {
                    theme.text_secondary
                })
                .add_modifier(Modifier::BOLD),
        ))];

        let title_height = 1u16;
        let title_para = Paragraph::new(title_text);
        let title_area = Rect::new(theme_area.x, theme_area.y, theme_area.width, title_height);
        title_para.render(title_area, buf);

        // List area
        let list_area = Rect::new(
            theme_area.x,
            theme_area.y + title_height,
            theme_area.width,
            theme_area.height.saturating_sub(title_height),
        );

        let current_theme = get_current_theme_type();
        let max_name_len = theme_area.width.saturating_sub(4) as usize;

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

                let style =
                    if i == self.selected_index && self.active_section == SettingsSection::Theme {
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

        let separator = "─".repeat(chunks[1].width as usize);
        let sep_para = Paragraph::new(separator).style(Style::default().fg(theme.border_unfocused));
        sep_para.render(chunks[1], buf);

        let perf_area = chunks[2];

        let perf_title = vec![Line::from(Span::styled(
            "Performance",
            Style::default()
                .fg(if self.active_section == SettingsSection::Performance {
                    theme.text_highlight
                } else {
                    theme.text_secondary
                })
                .add_modifier(Modifier::BOLD),
        ))];

        let perf_title_para = Paragraph::new(perf_title);
        let perf_title_area = Rect::new(perf_area.x, perf_area.y, perf_area.width, 1);
        perf_title_para.render(perf_title_area, buf);

        let thread_text = if self.thread_count == 0 {
            "Auto".to_string()
        } else {
            format!("{}", self.thread_count)
        };

        let thread_style = if self.active_section == SettingsSection::Performance {
            Style::default()
                .fg(theme.background)
                .bg(theme.text_highlight)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.text_primary)
        };

        let thread_setting = Paragraph::new(Line::from(vec![
            Span::raw("Threads: "),
            Span::styled(format!("< {} >", thread_text), thread_style),
        ]));

        let thread_area = Rect::new(perf_area.x, perf_area.y + 1, perf_area.width, 1);
        thread_setting.render(thread_area, buf);

        // Help text
        let help_lines = vec![Line::from(Span::styled(
            "↑↓: Navigate  ←→: Adjust",
            Style::default().fg(theme.text_secondary),
        ))];

        let help_para = Paragraph::new(help_lines).wrap(Wrap { trim: true });
        let help_area = Rect::new(perf_area.x, perf_area.y + 2, perf_area.width, 1);
        help_para.render(help_area, buf);
    }
}
