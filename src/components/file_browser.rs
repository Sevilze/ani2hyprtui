use super::Component;
use crate::event::AppMsg;
use crate::widgets::common::focused_block;
use crate::widgets::theme::get_theme;
use crossbeam_channel::Sender;
use crossterm::event::KeyCode;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    widgets::{List, ListItem, ListState, Scrollbar, ScrollbarOrientation, ScrollbarState, StatefulWidget, Widget},
};
use std::path::PathBuf;
use std::time::{Duration, Instant};

pub struct FileBrowserState {
    pub current_dir: PathBuf,
    pub initial_root: PathBuf,
    pub entries: Vec<PathBuf>,
    pub list_state: ListState,
    pub scroll_state: ScrollbarState,
    pub tx: Option<Sender<AppMsg>>,
    pub last_refresh: Instant,
}

impl Default for FileBrowserState {
    fn default() -> Self {
        let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let mut state = Self {
            current_dir: current_dir.clone(),
            initial_root: current_dir,
            entries: Vec::new(),
            list_state: ListState::default(),
            scroll_state: ScrollbarState::default(),
            tx: None,
            last_refresh: Instant::now(),
        };
        state.refresh_entries();
        if !state.entries.is_empty() {
            state.list_state.select(Some(0));
        }
        state
    }
}

impl FileBrowserState {
    pub fn set_sender(&mut self, tx: Sender<AppMsg>) {
        self.tx = Some(tx);
    }
}

impl FileBrowserState {
    fn refresh_entries(&mut self) {
        self.entries.clear();

        // Add parent directory entry if not at root and not at initial root
        if self.current_dir.parent().is_some() && self.current_dir != self.initial_root {
            self.entries.push(PathBuf::from(".."));
        }

        if let Ok(entries) = std::fs::read_dir(&self.current_dir) {
            let mut dirs = Vec::new();
            let mut files = Vec::new();

            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    dirs.push(path);
                } else {
                    files.push(path);
                }
            }

            dirs.sort();
            files.sort();

            self.entries.extend(dirs);
            self.entries.extend(files);
        }
    }

    fn enter_selected(&mut self) -> Option<PathBuf> {
        if let Some(idx) = self.list_state.selected() {
            if let Some(path) = self.entries.get(idx) {
                if path.to_string_lossy() == ".." {
                    if let Some(parent) = self.current_dir.parent() {
                        self.current_dir = parent.to_path_buf();
                        self.refresh_entries();
                        self.list_state.select(Some(0));
                        self.scroll_state = self.scroll_state.position(0);
                    }
                    None
                } else if path.is_dir() {
                    self.current_dir = path.clone();
                    self.refresh_entries();
                    self.list_state.select(Some(0));
                    self.scroll_state = self.scroll_state.position(0);
                    None
                } else {
                    Some(self.current_dir.clone())
                }
            } else {
                None
            }
        } else {
            None
        }
    }
}

impl Component for FileBrowserState {
    fn update(&mut self, msg: &AppMsg) -> Option<AppMsg> {
        match msg {
            AppMsg::Tick => {
                if self.last_refresh.elapsed() >= Duration::from_secs(1) {
                    self.refresh_entries();
                    self.last_refresh = Instant::now();
                    
                    // Ensure selection is valid
                    if let Some(selected) = self.list_state.selected()
                        && selected >= self.entries.len()
                    {
                        let new_selected = self.entries.len().saturating_sub(1);
                        self.list_state.select(Some(new_selected));
                    }
                }
            }
            AppMsg::Key(key) => {
                match key.code {
                KeyCode::Down | KeyCode::Char('j') => {
                    if self.entries.is_empty() {
                        return None;
                    }
                    let i = match self.list_state.selected() {
                        Some(i) => {
                            if i >= self.entries.len().saturating_sub(1) {
                                0
                            } else {
                                i + 1
                            }
                        }
                        None => 0,
                    };
                    self.list_state.select(Some(i));
                    self.scroll_state = self.scroll_state.position(i);
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    if self.entries.is_empty() {
                        return None;
                    }
                    let i = match self.list_state.selected() {
                        Some(i) => {
                            if i == 0 {
                                self.entries.len().saturating_sub(1)
                            } else {
                                i - 1
                            }
                        }
                        None => 0,
                    };
                    self.list_state.select(Some(i));
                    self.scroll_state = self.scroll_state.position(i);
                }
                KeyCode::Enter => {
                    if let Some(dir) = self.enter_selected()
                        && let Some(tx) = &self.tx
                    {
                        let _ = tx.send(AppMsg::CursorSelected(dir));
                    }
                }
                KeyCode::Char('l') => {
                    if let Some(tx) = &self.tx {
                        let _ = tx.send(AppMsg::CursorSelected(self.current_dir.clone()));
                    }
                }
                _ => {}
            }
        }
            _ => {}
        }
        None
    }

    fn render(&mut self, area: Rect, buf: &mut Buffer, is_focused: bool) {
        let theme = get_theme();

        let items: Vec<ListItem> = self
            .entries
            .iter()
            .map(|entry| {
                let icon = if entry.is_dir() { "📁" } else { "📄" };
                let name = entry.file_name().unwrap_or_default().to_string_lossy();
                ListItem::new(format!("{} {}", icon, name)).style(Style::default().fg(theme.text_primary))
            })
            .collect();

        let block = focused_block("File Browser", is_focused);
        let inner_area = block.inner(area);
        block.render(area, buf);

        let list = List::new(items)
            .highlight_style(
                Style::default()
                    .fg(theme.text_highlight)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol(">> ");

        StatefulWidget::render(list, inner_area, buf, &mut self.list_state);

        self.scroll_state = self.scroll_state.content_length(self.entries.len());

        let scrollbar = Scrollbar::default()
            .orientation(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("▲"))
            .end_symbol(Some("▼"));

        scrollbar.render(inner_area, buf, &mut self.scroll_state);
    }
}
