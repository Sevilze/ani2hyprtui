use super::Component;
use crate::event::AppMsg;
use crossbeam_channel::Sender;
use crossterm::event::KeyCode;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem, ListState, StatefulWidget},
};
use std::path::PathBuf;

pub struct FileBrowserState {
    pub current_dir: PathBuf,
    pub entries: Vec<PathBuf>,
    pub list_state: ListState,
    pub tx: Option<Sender<AppMsg>>,
}

impl Default for FileBrowserState {
    fn default() -> Self {
        let mut state = Self {
            current_dir: PathBuf::from("."),
            entries: Vec::new(),
            list_state: ListState::default(),
            tx: None,
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

        // Add parent directory entry if not at root
        if self.current_dir.parent().is_some() {
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
                    }
                    None
                } else if path.is_dir() {
                    self.current_dir = path.clone();
                    self.refresh_entries();
                    self.list_state.select(Some(0));
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
            AppMsg::Key(key) => match key.code {
                KeyCode::Down | KeyCode::Char('j') => {
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
                }
                KeyCode::Up | KeyCode::Char('k') => {
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
                }
                KeyCode::Enter => {
                    if let Some(dir) = self.enter_selected() {
                        if let Some(tx) = &self.tx {
                            let _ = tx.send(AppMsg::CursorSelected(dir));
                        }
                    }
                }
                KeyCode::Char('l') => {
                    if let Some(tx) = &self.tx {
                        let _ = tx.send(AppMsg::CursorSelected(self.current_dir.clone()));
                    }
                }
                _ => {}
            },
            _ => {}
        }
        None
    }
    
    fn render(&mut self, area: Rect, buf: &mut Buffer) {
        let items: Vec<ListItem> = self
            .entries
            .iter()
            .map(|entry| {
                let icon = if entry.is_dir() { "📁" } else { "📄" };
                let name = entry.file_name().unwrap_or_default().to_string_lossy();
                ListItem::new(format!("{} {}", icon, name))
            })
            .collect();

        let block = Block::default()
            .title("File Browser")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Blue));

        let list = List::new(items)
            .block(block)
            .highlight_style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol(">> ");

        StatefulWidget::render(list, area, buf, &mut self.list_state);
    }
}
