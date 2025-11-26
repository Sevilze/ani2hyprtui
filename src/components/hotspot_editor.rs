use super::Component;
use super::preview::PreviewState;
use crate::event::AppMsg;
use crate::model::cursor::CursorMeta;
use crate::widgets::common::focused_block;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        List, ListItem, ListState, Scrollbar, ScrollbarOrientation, ScrollbarState, StatefulWidget,
        Widget,
    },
};
use ratatui_image::picker::Picker;
use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

pub struct HotspotEditorState {
    pub frame_ix: usize,
    pub playing: bool,
    pub cursors: Vec<CursorMeta>,
    pub selected_cursor: usize,
    pub selected_variant: usize,

    // Edits
    pub modified_hotspots: HashSet<String>,
    pub list_state: ListState,
    pub scroll_state: ScrollbarState,
    pub preview: PreviewState,

    // Animation timing
    pub last_tick: Instant,
    pub accumulator: Duration,
    pub maximized: bool,
}

impl Default for HotspotEditorState {
    fn default() -> Self {
        Self::new_with_picker(Picker::from_fontsize((8, 16)))
    }
}

impl HotspotEditorState {
    pub fn new_with_picker(picker: Picker) -> Self {
        let picker_arc = Arc::new(Mutex::new(picker));
        Self {
            frame_ix: 0,
            playing: true,
            cursors: Vec::new(),
            selected_cursor: 0,
            selected_variant: 0,
            modified_hotspots: HashSet::new(),
            list_state: ListState::default(),
            scroll_state: ScrollbarState::default(),
            preview: PreviewState::new(picker_arc),
            last_tick: Instant::now(),
            accumulator: Duration::ZERO,
            maximized: false,
        }
    }

    fn current_frame_delay(&self) -> u64 {
        self.cursors
            .get(self.selected_cursor)
            .and_then(|c| c.variants.get(self.selected_variant))
            .and_then(|v| v.frames.get(self.frame_ix))
            .map(|f| f.delay_ms as u64)
            .unwrap_or(50)
    }

    fn current_variant_frames_len(&self) -> Option<usize> {
        self.cursors
            .get(self.selected_cursor)
            .and_then(|c| c.variants.get(self.selected_variant))
            .map(|v| v.frames.len())
    }

    fn reset_animation_timer(&mut self) {
        self.last_tick = Instant::now();
        self.accumulator = Duration::ZERO;
    }

    fn next_frame(&mut self) {
        if let Some(len) = self.current_variant_frames_len()
            && len > 0
        {
            self.frame_ix = (self.frame_ix + 1) % len;
        }
    }

    fn prev_frame(&mut self) {
        if let Some(len) = self.current_variant_frames_len()
            && len > 0
        {
            self.frame_ix = if self.frame_ix == 0 {
                len - 1
            } else {
                self.frame_ix - 1
            };
        }
    }

    fn next_cursor(&mut self) {
        if self.selected_cursor < self.cursors.len().saturating_sub(1) {
            self.selected_cursor += 1;
            self.frame_ix = 0;
            self.selected_variant = 0;
            self.list_state.select(Some(self.selected_cursor));
            self.scroll_state = self.scroll_state.position(self.selected_cursor);
            self.reset_animation_timer();
        }
    }

    fn prev_cursor(&mut self) {
        if self.selected_cursor > 0 {
            self.selected_cursor -= 1;
            self.frame_ix = 0;
            self.selected_variant = 0;
            self.list_state.select(Some(self.selected_cursor));
            self.scroll_state = self.scroll_state.position(self.selected_cursor);
            self.reset_animation_timer();
        }
    }

    fn next_variant(&mut self) {
        if let Some(cursor) = self.cursors.get(self.selected_cursor)
            && self.selected_variant < cursor.variants.len().saturating_sub(1)
        {
            self.selected_variant += 1;
            self.frame_ix = 0;
            self.reset_animation_timer();
        }
    }

    fn prev_variant(&mut self) {
        if self.selected_variant > 0 {
            self.selected_variant -= 1;
            self.frame_ix = 0;
            self.reset_animation_timer();
        }
    }

    fn move_hotspot(&mut self, dx: i32, dy: i32) {
        if let Some(cursor) = self.cursors.get_mut(self.selected_cursor)
            && let Some(variant) = cursor.variants.get_mut(self.selected_variant)
        {
            let (mut hx, mut hy) = variant.hotspot;

            hx = (hx as i32 + dx).max(0).min(variant.size as i32) as u32;
            hy = (hy as i32 + dy).max(0).min(variant.size as i32) as u32;

            if variant.hotspot != (hx, hy) {
                variant.hotspot = (hx, hy);
                self.modified_hotspots.insert(cursor.x11_name.clone());
                // Only invalidate protocol cache
                self.preview.invalidate_protocol_for_variant(variant);
            }
        }
    }

    fn handle_key(&mut self, key: KeyEvent) -> Option<AppMsg> {
        match key.code {
            KeyCode::Char(' ') => {
                if key
                    .modifiers
                    .contains(crossterm::event::KeyModifiers::CONTROL)
                {
                    self.maximized = !self.maximized;
                    None
                } else {
                    self.playing = !self.playing;
                    Some(AppMsg::LogMessage(format!(
                        "Animation {}",
                        if self.playing { "playing" } else { "paused" }
                    )))
                }
            }
            KeyCode::Left => {
                self.move_hotspot(-1, 0);
                None
            }
            KeyCode::Right => {
                self.move_hotspot(1, 0);
                None
            }
            KeyCode::Up => {
                self.move_hotspot(0, -1);
                None
            }
            KeyCode::Down => {
                self.move_hotspot(0, 1);
                None
            }
            KeyCode::Char('j') => {
                self.next_cursor();
                None
            }
            KeyCode::Char('k') => {
                self.prev_cursor();
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
            KeyCode::Char('s') => {
                if !self.modified_hotspots.is_empty() {
                    let modified: Vec<String> = self.modified_hotspots.drain().collect();
                    Some(AppMsg::HotspotsSaved(modified))
                } else {
                    None
                }
            }
            KeyCode::Char(',') => {
                self.playing = false;
                self.prev_frame();
                None
            }
            KeyCode::Char('.') => {
                self.playing = false;
                self.next_frame();
                None
            }
            _ => None,
        }
    }

    fn render_cursor_list(&mut self, area: Rect, buf: &mut Buffer, is_focused: bool) {
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

                let marker = if self.modified_hotspots.contains(&cursor.x11_name) {
                    "*"
                } else {
                    ""
                };

                ListItem::new(Line::from(vec![
                    Span::styled(format!("{}{}", cursor.x11_name, marker), style),
                    Span::raw(" "),
                    Span::styled(
                        format!("({})", cursor.variants.len()),
                        style.fg(Color::DarkGray),
                    ),
                ]))
            })
            .collect();

        let block = focused_block("Cursors (j/k: select)", is_focused);

        let inner_area = block.inner(area);
        block.render(area, buf);

        let list = List::new(items).highlight_style(
            Style::default()
                .fg(Color::Black)
                .bg(Color::Green)
                .add_modifier(Modifier::BOLD),
        );

        StatefulWidget::render(list, inner_area, buf, &mut self.list_state);

        self.scroll_state = self.scroll_state.content_length(self.cursors.len());
        let scrollbar = Scrollbar::default()
            .orientation(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("▲"))
            .end_symbol(Some("▼"));

        scrollbar.render(inner_area, buf, &mut self.scroll_state);
    }
}

impl Component for HotspotEditorState {
    fn update(&mut self, msg: &AppMsg) -> Option<AppMsg> {
        match msg {
            AppMsg::Tick => {
                if self.playing {
                    let now = Instant::now();
                    let mut delta = now.duration_since(self.last_tick);

                    // Clamp delta to prevent jittery frames
                    if delta > Duration::from_millis(100) {
                        delta = Duration::from_millis(100);
                    }

                    self.last_tick = now;
                    self.accumulator += delta;

                    let mut frame_delay = Duration::from_millis(self.current_frame_delay());

                    // Prevent infinite loop if delay is 0
                    if frame_delay.is_zero() {
                        frame_delay = Duration::from_millis(50);
                    }

                    while self.accumulator >= frame_delay {
                        self.accumulator -= frame_delay;
                        self.next_frame();
                        // Update frame delay for the new frame
                        frame_delay = Duration::from_millis(self.current_frame_delay());
                        if frame_delay.is_zero() {
                            frame_delay = Duration::from_millis(50);
                        }
                    }
                } else {
                    // Reset timer when not playing
                    self.last_tick = Instant::now();
                    self.accumulator = Duration::ZERO;
                }
                None
            }
            AppMsg::CursorLoaded(cursors) => {
                self.cursors = cursors.clone();
                self.selected_cursor = 0;

                // Default to 48x48
                self.selected_variant = 0;
                if let Some(cursor) = self.cursors.first()
                    && let Some(idx) = cursor.variants.iter().position(|v| v.size == 48)
                {
                    self.selected_variant = idx;
                }
                self.frame_ix = 0;
                self.modified_hotspots.clear();
                self.preview.clear_cache();
                self.list_state.select(Some(0));
                self.scroll_state = self
                    .scroll_state
                    .content_length(self.cursors.len())
                    .position(0);

                // Reset animation state
                self.last_tick = Instant::now();
                self.accumulator = Duration::ZERO;

                None
            }
            AppMsg::Key(key) => self.handle_key(*key),
            _ => None,
        }
    }

    fn render(&mut self, area: Rect, buf: &mut Buffer, is_focused: bool) {
        if self.cursors.is_empty() {
            let block = focused_block("Hotspot Editor", is_focused);

            ratatui::widgets::Paragraph::new("No cursor loaded")
                .block(block)
                .render(area, buf);
            return;
        }

        // Main Editor Block
        let block = focused_block("Hotspot Editor", is_focused);

        let inner = block.inner(area);
        block.render(area, buf);

        let chunks = if self.maximized {
            Layout::default()
                .constraints([Constraint::Percentage(0), Constraint::Percentage(100)])
                .direction(ratatui::layout::Direction::Horizontal)
                .split(inner)
        } else {
            Layout::default()
                .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
                .direction(ratatui::layout::Direction::Horizontal)
                .split(inner)
        };

        if !self.maximized {
            self.render_cursor_list(chunks[0], buf, false);
        }

        let path_string = if let Some(cursor) = self.cursors.get(self.selected_cursor) {
            if let Some(variant) = cursor.variants.get(self.selected_variant) {
                variant
                    .frames
                    .get(self.frame_ix)
                    .map(|frame| frame.png_path.to_string_lossy().to_string())
            } else {
                None
            }
        } else {
            None
        };

        let data = if let (Some(path), Some(cursor)) =
            (&path_string, self.cursors.get(self.selected_cursor))
        {
            if let Some(variant) = cursor.variants.get(self.selected_variant) {
                if let Some(frame) = variant.frames.get(self.frame_ix) {
                    Some((
                        path.as_str(),
                        variant.hotspot,
                        variant.size,
                        cursor,
                        variant,
                        frame,
                        self.frame_ix,
                    ))
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };

        self.preview.render(
            chunks[1],
            buf,
            is_focused,
            self.playing,
            self.maximized,
            data,
        );
    }
}
