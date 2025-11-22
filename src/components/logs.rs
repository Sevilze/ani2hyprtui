use super::Component;
use crate::event::AppMsg;
use crate::widgets::common::focused_block;
use crossterm::event::KeyCode;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, StatefulWidget, Widget},
};

#[derive(Default)]
pub struct LogsState {
    pub logs: Vec<String>,
    scroll_state: ScrollbarState,
    scroll_offset: u16,
}

impl LogsState {
    pub fn add_log(&mut self, message: String) {
        self.logs.push(message);
        // Only auto-scroll if already near bottom
        let total = self.logs.len();
        if total <= 1 || self.scroll_offset as usize + 5 >= total {
            self.scroll_offset = total as u16;
        }
    }
}

impl Component for LogsState {
    fn update(&mut self, msg: &AppMsg) -> Option<AppMsg> {
        match msg {
            AppMsg::LogMessage(msg) => {
                self.add_log(msg.clone());
            }
            AppMsg::ErrorOccurred(err) => {
                self.add_log(format!("ERROR: {}", err));
            }
            AppMsg::Key(key) => match key.code {
                KeyCode::Up | KeyCode::Char('k') => {
                    self.scroll_offset = self.scroll_offset.saturating_sub(1);
                    self.scroll_state = self.scroll_state.position(self.scroll_offset as usize);
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    self.scroll_offset = self.scroll_offset.saturating_add(1);
                    self.scroll_state = self.scroll_state.position(self.scroll_offset as usize);
                }
                KeyCode::PageUp => {
                    self.scroll_offset = self.scroll_offset.saturating_sub(10);
                    self.scroll_state = self.scroll_state.position(self.scroll_offset as usize);
                }
                KeyCode::PageDown => {
                    self.scroll_offset = self.scroll_offset.saturating_add(10);
                    self.scroll_state = self.scroll_state.position(self.scroll_offset as usize);
                }
                _ => {}
            },
            _ => {}
        }
        None
    }

    fn render(&mut self, area: Rect, buf: &mut Buffer, is_focused: bool) {
        let block = focused_block("Logs", is_focused);

        let inner_area = block.inner(area);
        block.render(area, buf);

        let width = (inner_area.width as usize).saturating_sub(2);
        if width == 0 {
            return;
        }

        // Calculate wrapped lines to determine total height
        let mut total_height = 0;
        let mut wrapped_lines = Vec::new();

        for log in &self.logs {
            let lines = textwrap::wrap(log, width);
            total_height += lines.len();
            for line in lines {
                wrapped_lines.push(line.to_string());
            }
        }

        let viewport_height = inner_area.height as usize;
        let max_scroll = total_height.saturating_sub(viewport_height);

        self.scroll_state = self.scroll_state.content_length(total_height);

        if self.scroll_offset as usize > max_scroll {
            self.scroll_offset = max_scroll as u16;
        }

        let is_at_bottom = self.scroll_offset as usize >= max_scroll.saturating_sub(1);

        if is_at_bottom {
            self.scroll_offset = max_scroll as u16;
        }

        let styled_lines: Vec<Line> = wrapped_lines
            .iter()
            .map(|line| {
                let style = if line.contains("ERROR") {
                    Style::default().fg(Color::Red)
                } else if line.contains("completed") || line.contains("Success") {
                    Style::default().fg(Color::Green)
                } else {
                    Style::default()
                };
                Line::from(Span::styled(line.clone(), style))
            })
            .collect();

        let paragraph = Paragraph::new(styled_lines).scroll((self.scroll_offset, 0));

        paragraph.render(inner_area, buf);

        let scrollbar = Scrollbar::default()
            .orientation(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("▲"))
            .end_symbol(Some("▼"));

        StatefulWidget::render(scrollbar, inner_area, buf, &mut self.scroll_state);
    }
}
