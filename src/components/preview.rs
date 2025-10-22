use super::Component;
use crate::event::AppMsg;
use crate::model::cursor::CursorMeta;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Widget},
};

pub struct PreviewState {
    pub frame_ix: usize,
    pub playing: bool,
    pub cursors: Vec<CursorMeta>,
    pub selected_cursor: usize,
}

impl Default for PreviewState {
    fn default() -> Self {
        Self {
            frame_ix: 0,
            playing: true,
            cursors: Vec::new(),
            selected_cursor: 0,
        }
    }
}

impl PreviewState {
    pub fn render(&self, area: Rect, buf: &mut Buffer) {
        let content = if self.cursors.is_empty() {
            vec![
                Line::from("No cursor loaded"),
                Line::from(""),
                Line::from("Select a directory in the file browser"),
            ]
        } else if let Some(cursor) = self.cursors.get(self.selected_cursor) {
            let mut lines = vec![
                Line::from(vec![
                    Span::styled("Cursor: ", Style::default().fg(Color::Cyan)),
                    Span::raw(&cursor.x11_name),
                ]),
                Line::from(""),
            ];

            if let Some(variant) = cursor.variants.first() {
                lines.push(Line::from(vec![
                    Span::styled("Size: ", Style::default().fg(Color::Cyan)),
                    Span::raw(format!("{}x{}", variant.size, variant.size)),
                ]));
                lines.push(Line::from(vec![
                    Span::styled("Hotspot: ", Style::default().fg(Color::Cyan)),
                    Span::raw(format!("({}, {})", variant.hotspot.0, variant.hotspot.1)),
                ]));
                lines.push(Line::from(vec![
                    Span::styled("Frames: ", Style::default().fg(Color::Cyan)),
                    Span::raw(format!("{}", variant.frames.len())),
                ]));

                if self.playing {
                    lines.push(Line::from(""));
                    lines.push(Line::from(vec![
                        Span::styled("Frame: ", Style::default().fg(Color::Green)),
                        Span::raw(format!(
                            "{}/{}",
                            self.frame_ix + 1,
                            variant.frames.len()
                        )),
                    ]));
                }
            }

            lines.push(Line::from(""));
            lines.push(Line::from(vec![Span::styled(
                if self.playing { "▶ Playing" } else { "⏸ Paused" },
                Style::default().fg(if self.playing {
                    Color::Green
                } else {
                    Color::Yellow
                }),
            )]));

            lines
        } else {
            vec![Line::from("Invalid cursor selection")]
        };

        let paragraph = Paragraph::new(content).block(
            Block::default()
                .borders(Borders::ALL)
                .title("Cursor Preview"),
        );

        paragraph.render(area, buf);
    }
}

impl Component for PreviewState {
    fn update(&mut self, msg: &AppMsg) {
        match msg {
            AppMsg::Tick => {
                if self.playing && !self.cursors.is_empty() {
                    if let Some(cursor) = self.cursors.get(self.selected_cursor) {
                        if let Some(variant) = cursor.variants.first() {
                            if !variant.frames.is_empty() {
                                self.frame_ix = (self.frame_ix + 1) % variant.frames.len();
                            }
                        }
                    }
                }
            }
            AppMsg::CursorLoaded(cursors) => {
                self.cursors = cursors.clone();
                self.selected_cursor = 0;
                self.frame_ix = 0;
            }
            _ => {}
        }
    }
}

