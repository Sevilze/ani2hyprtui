use crossbeam_channel::Sender;
use crossterm::event::KeyCode;
use ratatui::{buffer::Buffer, layout::Rect, widgets::ListState};
use std::path::PathBuf;
use std::time::{Duration, Instant};

use crate::components::Component;
use crate::config::AppConfig;
use crate::event::AppMsg;

use super::apply;
use super::discovery;
use super::types::CursorThemeItem;
use super::ui;

pub struct HyprctlState {
    pub themes: Vec<CursorThemeItem>,
    pub selected_theme_idx: usize,
    pub theme_list_state: ListState,
    pub selected_size_idx: usize,
    pub active_theme: String,
    pub active_size: u32,
    pub status_message: Option<(String, bool)>,
    pub last_refresh: Instant,
    pub is_applying: bool,
    pub extra_dirs: Vec<PathBuf>,
    pub tx: Option<Sender<AppMsg>>,
}

impl Default for HyprctlState {
    fn default() -> Self {
        let themes = discovery::scan_cursor_themes();
        let (detected_theme, detected_size) = discovery::detect_active_cursor();

        let selected_theme_idx = if !detected_theme.is_empty() {
            themes
                .iter()
                .position(|t| t.name == detected_theme)
                .or_else(|| {
                    themes
                        .iter()
                        .position(|t| t.name.eq_ignore_ascii_case(&detected_theme))
                })
                .unwrap_or(0)
        } else {
            0
        };

        let mut theme_list_state = ListState::default();
        if !themes.is_empty() {
            theme_list_state.select(Some(selected_theme_idx));
        }

        let active_theme = if let Some(theme) = themes.get(selected_theme_idx) {
            theme.name.clone()
        } else {
            detected_theme
        };

        let selected_size_idx = if let Some(theme) = themes.get(selected_theme_idx) {
            theme
                .available_sizes
                .iter()
                .position(|&s| s == detected_size)
                .unwrap_or_else(|| {
                    theme
                        .available_sizes
                        .iter()
                        .position(|&s| s >= detected_size)
                        .unwrap_or(0)
                })
        } else {
            0
        };

        let active_size = if let Some(theme) = themes.get(selected_theme_idx) {
            theme
                .available_sizes
                .get(selected_size_idx)
                .copied()
                .unwrap_or(detected_size)
        } else {
            detected_size
        };

        let status_message = Some((format!("Applied: {}px", active_size), false));

        Self {
            themes,
            selected_theme_idx,
            theme_list_state,
            selected_size_idx,
            active_theme,
            active_size,
            status_message,
            last_refresh: Instant::now(),
            is_applying: false,
            extra_dirs: Vec::new(),
            tx: None,
        }
    }
}

impl HyprctlState {
    pub fn set_sender(&mut self, tx: Sender<AppMsg>) {
        self.tx = Some(tx);
    }

    pub fn set_extra_dirs(&mut self, dirs: Vec<PathBuf>) {
        if self.extra_dirs != dirs {
            self.extra_dirs = dirs;
            self.refresh_themes();
        }
    }

    pub fn set_extra_dir(&mut self, dir: Option<PathBuf>) {
        let dirs = match dir {
            Some(d) => vec![d],
            None => Vec::new(),
        };
        self.set_extra_dirs(dirs);
    }

    pub fn refresh_themes(&mut self) {
        let new_themes = discovery::scan_cursor_themes_with_extra(&self.extra_dirs);
        if new_themes == self.themes {
            return;
        }

        let selected_name = self
            .themes
            .get(self.selected_theme_idx)
            .map(|t| t.name.clone());
        let selected_size = self.current_size();

        self.themes = new_themes;

        if self.themes.is_empty() {
            self.selected_theme_idx = 0;
            self.theme_list_state.select(None);
            self.selected_size_idx = 0;
            if let Some(deleted_name) = selected_name {
                self.status_message = Some((
                    format!("Theme '{}' was deleted from disk", deleted_name),
                    true,
                ));
            }
            return;
        }

        if let Some(ref name) = selected_name {
            if let Some(pos) = self.themes.iter().position(|t| &t.name == name) {
                self.selected_theme_idx = pos;
            } else {
                if self.selected_theme_idx >= self.themes.len() {
                    self.selected_theme_idx = self.themes.len() - 1;
                }
                self.status_message =
                    Some((format!("Theme '{}' was deleted from disk", name), true));
            }
        } else if self.selected_theme_idx >= self.themes.len() {
            self.selected_theme_idx = self.themes.len() - 1;
        }

        self.theme_list_state.select(Some(self.selected_theme_idx));

        if let Some(theme) = self.themes.get(self.selected_theme_idx) {
            self.selected_size_idx = theme
                .available_sizes
                .iter()
                .position(|&s| s == selected_size)
                .unwrap_or_else(|| {
                    theme
                        .available_sizes
                        .iter()
                        .position(|&s| s >= selected_size)
                        .unwrap_or(0)
                });
        } else {
            self.selected_size_idx = 0;
        }
    }

    pub fn current_theme(&self) -> Option<&CursorThemeItem> {
        self.themes.get(self.selected_theme_idx)
    }

    pub fn current_size(&self) -> u32 {
        if let Some(theme) = self.current_theme() {
            theme
                .available_sizes
                .get(self.selected_size_idx)
                .copied()
                .unwrap_or(24)
        } else {
            24
        }
    }

    pub fn apply_cursor(&mut self) -> Option<AppMsg> {
        apply::apply_cursor(self)
    }

    pub fn select_theme_idx(&mut self, idx: usize) {
        if self.themes.is_empty() {
            return;
        }
        let old_size = self.current_size();
        self.selected_theme_idx = idx;
        self.theme_list_state.select(Some(idx));

        if let Some(new_theme) = self.current_theme() {
            self.selected_size_idx = new_theme
                .available_sizes
                .iter()
                .position(|&s| s == old_size)
                .unwrap_or_else(|| {
                    new_theme
                        .available_sizes
                        .iter()
                        .position(|&s| s >= old_size)
                        .unwrap_or(0)
                });
        }
    }
}

impl Component for HyprctlState {
    fn update(&mut self, msg: &AppMsg) -> Option<AppMsg> {
        match msg {
            AppMsg::Tick => {
                if self.last_refresh.elapsed() >= Duration::from_millis(500) {
                    self.refresh_themes();
                    self.last_refresh = Instant::now();
                }
            }
            AppMsg::PipelineCompleted(_) | AppMsg::XCursorGenerated(_) => {
                self.refresh_themes();
                self.last_refresh = Instant::now();
            }
            AppMsg::OutputDirSelected(path) => {
                self.set_extra_dir(Some(path.clone()));
            }
            AppMsg::CursorThemeApplied {
                theme_name,
                size,
                success,
                message,
            } => {
                self.is_applying = false;
                self.status_message = Some((message.clone(), !success));
                if *success {
                    self.active_theme = theme_name.clone();
                    self.active_size = *size;
                    AppConfig::save_cursor(theme_name, *size);
                    return Some(AppMsg::LogMessage(format!(
                        "hyprctl setcursor {} {} ({})",
                        theme_name, size, message
                    )));
                } else {
                    return Some(AppMsg::LogMessage(format!(
                        "hyprctl setcursor failed: {}",
                        message
                    )));
                }
            }
            AppMsg::Key(key) => match key.code {
                KeyCode::Up | KeyCode::Char('k') => {
                    if !self.themes.is_empty() {
                        let new_idx = if self.selected_theme_idx > 0 {
                            self.selected_theme_idx - 1
                        } else {
                            self.themes.len() - 1
                        };
                        self.select_theme_idx(new_idx);
                    }
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if !self.themes.is_empty() {
                        let new_idx = if self.selected_theme_idx + 1 < self.themes.len() {
                            self.selected_theme_idx + 1
                        } else {
                            0
                        };
                        self.select_theme_idx(new_idx);
                    }
                }
                KeyCode::Left | KeyCode::Char('h') => {
                    if let Some(theme) = self.current_theme()
                        && !theme.available_sizes.is_empty()
                    {
                        if self.selected_size_idx > 0 {
                            self.selected_size_idx -= 1;
                        } else {
                            self.selected_size_idx = theme.available_sizes.len() - 1;
                        }
                    }
                }
                KeyCode::Right | KeyCode::Char('l') => {
                    if let Some(theme) = self.current_theme()
                        && !theme.available_sizes.is_empty()
                    {
                        if self.selected_size_idx + 1 < theme.available_sizes.len() {
                            self.selected_size_idx += 1;
                        } else {
                            self.selected_size_idx = 0;
                        }
                    }
                }
                KeyCode::Enter | KeyCode::Char(' ') => {
                    if self.is_applying {
                        return None;
                    }
                    return self.apply_cursor();
                }
                _ => {}
            },
            _ => {}
        }
        None
    }

    fn render(&mut self, area: Rect, buf: &mut Buffer, is_focused: bool) {
        ui::render_hyprctl(self, area, buf, is_focused);
    }
}
