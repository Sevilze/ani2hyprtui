use super::Component;
use crate::event::AppMsg;
use crate::widgets::common::focused_block;
use crate::widgets::theme::get_theme;
use crossterm::event::KeyCode;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, StatefulWidget, Widget},
};

#[derive(Debug)]
pub struct LogsState {
    pub logs: Vec<String>,
    scroll_state: ScrollbarState,
    scroll_offset: u16,
    stick_to_bottom: bool,
}

impl Default for LogsState {
    fn default() -> Self {
        Self {
            logs: Vec::new(),
            scroll_state: ScrollbarState::default(),
            scroll_offset: 0,
            stick_to_bottom: true,
        }
    }
}

impl LogsState {
    pub fn add_log(&mut self, message: String) {
        self.logs.push(message);
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
                    self.stick_to_bottom = false;
                    self.scroll_offset = self.scroll_offset.saturating_sub(1);
                    self.scroll_state = self.scroll_state.position(self.scroll_offset as usize);
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    self.scroll_offset = self.scroll_offset.saturating_add(1);
                    self.scroll_state = self.scroll_state.position(self.scroll_offset as usize);
                }
                KeyCode::PageUp => {
                    self.stick_to_bottom = false;
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

        if self.stick_to_bottom {
            self.scroll_offset = max_scroll as u16;
        } else if self.scroll_offset as usize > max_scroll {
            self.scroll_offset = max_scroll as u16;
            self.stick_to_bottom = true;
        }

        // If user manually scrolled to bottom, re-enable stickiness
        if !self.stick_to_bottom && self.scroll_offset as usize >= max_scroll {
            self.stick_to_bottom = true;
        }

        let styled_lines: Vec<Line> = wrapped_lines
            .iter()
            .map(|line| {
                let theme = get_theme();
                let style = if line.contains("ERROR") {
                    Style::default().fg(theme.status_failed)
                } else if line.contains("completed") || line.contains("Success") {
                    Style::default().fg(theme.status_completed)
                } else {
                    Style::default().fg(theme.text_primary)
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
