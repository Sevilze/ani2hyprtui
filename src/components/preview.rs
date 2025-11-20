use super::Component;
use crate::event::AppMsg;
use crate::model::cursor::CursorMeta;
use crossterm::event::{KeyCode, KeyEvent};
use image::GenericImageView;
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, StatefulWidget, Widget, Wrap},
};
use ratatui_image::{StatefulImage, picker::Picker, protocol::StatefulProtocol};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub struct PreviewState {
    pub frame_ix: usize,
    pub playing: bool,
    pub cursors: Vec<CursorMeta>,
    pub selected_cursor: usize,
    pub selected_variant: usize,
    pub picker: Arc<Mutex<Picker>>,
    pub image_cache: HashMap<String, StatefulProtocol>,
}

impl Default for PreviewState {
    fn default() -> Self {
        Self::new_with_picker(Picker::from_fontsize((8, 16)))
    }
}

impl PreviewState {
    pub fn new_with_picker(picker: Picker) -> Self {
        Self {
            frame_ix: 0,
            playing: true,
            cursors: Vec::new(),
            selected_cursor: 0,
            selected_variant: 0,
            picker: Arc::new(Mutex::new(picker)),
            image_cache: HashMap::new(),
        }
    }
}

impl PreviewState {
    fn current_variant_frames_len(&self) -> Option<usize> {
        self.cursors
            .get(self.selected_cursor)
            .and_then(|c| c.variants.get(self.selected_variant))
            .map(|v| v.frames.len())
    }

    fn next_frame(&mut self) {
        if let Some(len) = self.current_variant_frames_len() {
            if len > 0 {
                self.frame_ix = (self.frame_ix + 1) % len;
            }
        }
    }

    fn prev_frame(&mut self) {
        if let Some(len) = self.current_variant_frames_len() {
            if len > 0 {
                self.frame_ix = if self.frame_ix == 0 {
                    len - 1
                } else {
                    self.frame_ix - 1
                };
            }
        }
    }

    fn next_cursor(&mut self) {
        if self.selected_cursor < self.cursors.len().saturating_sub(1) {
            self.selected_cursor += 1;
            self.frame_ix = 0;
            self.selected_variant = 0;
        }
    }

    fn prev_cursor(&mut self) {
        if self.selected_cursor > 0 {
            self.selected_cursor -= 1;
            self.frame_ix = 0;
            self.selected_variant = 0;
        }
    }

    fn next_variant(&mut self) {
        if let Some(cursor) = self.cursors.get(self.selected_cursor) {
            if self.selected_variant < cursor.variants.len().saturating_sub(1) {
                self.selected_variant += 1;
                self.frame_ix = 0;
            }
        }
    }

    fn prev_variant(&mut self) {
        if self.selected_variant > 0 {
            self.selected_variant -= 1;
            self.frame_ix = 0;
        }
    }

    fn handle_key(&mut self, key: KeyEvent) -> Option<AppMsg> {
        match key.code {
            KeyCode::Char(' ') => {
                self.playing = !self.playing;
                Some(AppMsg::LogMessage(format!(
                    "Animation {}",
                    if self.playing { "playing" } else { "paused" }
                )))
            }
            KeyCode::Left => {
                self.prev_frame();
                None
            }
            KeyCode::Right => {
                self.next_frame();
                None
            }
            KeyCode::Up => {
                self.prev_cursor();
                None
            }
            KeyCode::Down => {
                self.next_cursor();
                None
            }
            KeyCode::Char('[') => {
                self.prev_variant();
                None
            }
            KeyCode::Char(']') => {
                self.next_variant();
                None
            }
            _ => None,
        }
    }

    fn ensure_image_cached(&mut self, path: &str) {
        if !self.image_cache.contains_key(path) {
            if let Ok(img) = image::open(path) {
                if let Ok(picker) = self.picker.lock() {
                    let proto = picker.new_resize_protocol(img);
                    self.image_cache.insert(path.to_string(), proto);
                }
            }
        }
    }

    fn render_cursor_list(&self, area: Rect, buf: &mut Buffer) {
        let items: Vec<ListItem> = self
            .cursors
            .iter()
            .enumerate()
            .map(|(i, cursor)| {
                let style = if i == self.selected_cursor {
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::Green)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };

                ListItem::new(Line::from(vec![
                    Span::styled(&cursor.x11_name, style),
                    Span::raw(" "),
                    Span::styled(
                        format!("({})", cursor.variants.len()),
                        style.fg(Color::DarkGray),
                    ),
                ]))
            })
            .collect();

        let list = List::new(items).block(
            Block::default()
                .title("Cursors (↑↓: select)")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Green)),
        );
        Widget::render(list, area, buf);
    }

    fn render_preview(&mut self, area: Rect, buf: &mut Buffer) {
        let path_to_cache = if let Some(cursor) = self.cursors.get(self.selected_cursor) {
            if let Some(variant) = cursor.variants.get(self.selected_variant) {
                if let Some(frame) = variant.frames.get(self.frame_ix) {
                    Some(frame.png_path.to_string_lossy().to_string())
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };

        if let Some(path) = &path_to_cache {
            self.ensure_image_cached(path);
        }

        let PreviewState {
            cursors,
            selected_cursor,
            selected_variant,
            frame_ix,
            playing,
            image_cache,
            ..
        } = self;

        let cursor = &cursors[*selected_cursor];
        let variant_info = if cursor.variants.len() > 1 {
            format!(
                " - Variant {}/{} ([/]: switch)",
                *selected_variant + 1,
                cursor.variants.len()
            )
        } else {
            String::new()
        };

        let block = Block::default()
            .title(format!(
                "Preview{} (Space: play/pause, ←→: frame)",
                variant_info
            ))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(if *playing {
                Color::Green
            } else {
                Color::Yellow
            }));

        let inner = block.inner(area);
        block.render(area, buf);

        let Some(variant) = cursor.variants.get(*selected_variant) else {
            return;
        };
        let Some(frame) = variant.frames.get(*frame_ix) else {
            return;
        };

        let path_str = frame.png_path.to_string_lossy().to_string();

        if let Some(proto) = image_cache.get_mut(&path_str) {
            let chunks = Layout::default()
                .constraints([Constraint::Min(10), Constraint::Length(3)])
                .direction(ratatui::layout::Direction::Vertical)
                .split(inner);

            StatefulImage::default().render(chunks[0], buf, proto);

            let info_text = if let Ok(img) = image::open(&frame.png_path) {
                let (w, h) = img.dimensions();
                format!(
                    "Frame: {}/{} | Delay: {}ms | Hotspot: ({}, {}) | Size: {}x{}",
                    *frame_ix + 1,
                    variant.frames.len(),
                    frame.delay_ms,
                    variant.hotspot.0,
                    variant.hotspot.1,
                    w,
                    h
                )
            } else {
                format!(
                    "Frame: {}/{} | Delay: {}ms | Hotspot: ({}, {})",
                    *frame_ix + 1,
                    variant.frames.len(),
                    frame.delay_ms,
                    variant.hotspot.0,
                    variant.hotspot.1
                )
            };

            Paragraph::new(info_text).render(chunks[1], buf);
        } else {
            Paragraph::new("Failed to load image").render(inner, buf);
        }
    }
}

impl Component for PreviewState {
    fn update(&mut self, msg: &AppMsg) -> Option<AppMsg> {
        match msg {
            AppMsg::Tick => {
                if self.playing {
                    self.next_frame();
                }
                None
            }
            AppMsg::CursorLoaded(cursors) => {
                self.cursors = cursors.clone();
                self.selected_cursor = 0;
                self.selected_variant = 0;
                self.frame_ix = 0;
                None
            }
            AppMsg::Key(key) => self.handle_key(*key),
            _ => None,
        }
    }

    fn render(&mut self, area: Rect, buf: &mut Buffer) {
        if self.cursors.is_empty() {
            let block = Block::default()
                .title("Preview")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Green));

            Paragraph::new(vec![
                Line::from("No cursor loaded"),
                Line::from(""),
                Line::from("Select a cursor from the File Browser"),
            ])
            .block(block)
            .wrap(Wrap { trim: true })
            .render(area, buf);
            return;
        }

        let chunks = Layout::default()
            .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
            .direction(ratatui::layout::Direction::Horizontal)
            .split(area);

        self.render_cursor_list(chunks[0], buf);
        self.render_preview(chunks[1], buf);
    }
}
