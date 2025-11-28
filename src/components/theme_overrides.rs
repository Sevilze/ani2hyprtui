use super::Component;
use crate::event::AppMsg;
use crate::widgets::common::focused_block;
use crate::widgets::theme::get_theme;
use crossterm::event::KeyCode;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    text::Span,
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, StatefulWidget, Widget},
};
use std::collections::HashSet;

pub struct ThemeOverridesState {
    pub output_name: String,
    pub available_sizes: Vec<u32>,
    pub selected_sizes: HashSet<u32>,
    pub selector_index: usize,
    pub list_state: ListState,
}

impl Default for ThemeOverridesState {
    fn default() -> Self {
        let available_sizes = vec![16, 24, 32, 48, 64, 72, 96, 128];
        let mut selected_sizes = HashSet::new();
        // Default selection
        selected_sizes.insert(24);
        selected_sizes.insert(32);
        selected_sizes.insert(48);

        let mut list_state = ListState::default();
        list_state.select(Some(0));

        Self {
            output_name: String::new(),
            available_sizes,
            selected_sizes,
            selector_index: 0,
            list_state,
        }
    }
}

impl Component for ThemeOverridesState {
    fn update(&mut self, msg: &AppMsg) -> Option<AppMsg> {
        if let AppMsg::Key(key) = msg {
            match key.code {
                KeyCode::Up => {
                    if self.selector_index > 0 {
                        self.selector_index -= 1;
                        self.list_state.select(Some(self.selector_index));
                    }
                }
                KeyCode::Down => {
                    if self.selector_index < self.available_sizes.len() - 1 {
                        self.selector_index += 1;
                        self.list_state.select(Some(self.selector_index));
                    }
                }
                KeyCode::Enter => {
                    let size = self.available_sizes[self.selector_index];
                    if self.selected_sizes.contains(&size) {
                        self.selected_sizes.remove(&size);
                    } else {
                        self.selected_sizes.insert(size);
                    }
                }
                KeyCode::Char(c) => {
                    // Allow alphanumeric, dash, underscore, and space
                    if c.is_alphanumeric() || c == '-' || c == '_' || c == ' ' {
                        self.output_name.push(c);
                    }
                }
                KeyCode::Backspace => {
                    self.output_name.pop();
                }
                _ => {}
            }
        }
        None
    }

    fn render(&mut self, area: Rect, buf: &mut Buffer, is_focused: bool) {
        let theme = get_theme();
        let block = focused_block("Theme Overrides", is_focused);

        let inner = block.inner(area);
        block.render(area, buf);

        let chunks = ratatui::layout::Layout::default()
            .direction(ratatui::layout::Direction::Vertical)
            .constraints([
                ratatui::layout::Constraint::Length(3), // Output Name
                ratatui::layout::Constraint::Min(1),    // Sizes
            ])
            .split(inner);

        // Output Name Field
        let name_style = if is_focused {
            Style::default().fg(theme.text_highlight)
        } else {
            Style::default().fg(theme.text_primary)
        };
        let name_block = Block::default()
            .title("Output Name")
            .borders(Borders::ALL)
            .style(name_style);

        // Add cursor
        let name_text = if is_focused {
            format!("{}_", self.output_name)
        } else {
            self.output_name.clone()
        };

        Paragraph::new(name_text)
            .block(name_block)
            .render(chunks[0], buf);

        // Sizes Field
        let size_block = Block::default()
            .title("Sizes (Enter to toggle)")
            .borders(Borders::ALL);
        let inner_size_area = size_block.inner(chunks[1]);
        size_block.render(chunks[1], buf);

        let items: Vec<ListItem> = self
            .available_sizes
            .iter()
            .enumerate()
            .map(|(i, size)| {
                let is_selected = self.selected_sizes.contains(size);
                let checkbox = if is_selected { "[x]" } else { "[ ]" };
                let content = format!("{} {}x{}", checkbox, size, size);

                let style = if i == self.selector_index && is_focused {
                    Style::default()
                        .fg(theme.background)
                        .bg(theme.text_highlight)
                } else {
                    Style::default().fg(theme.text_primary)
                };

                ListItem::new(Span::styled(content, style))
            })
            .collect();

        let list = List::new(items);
        StatefulWidget::render(list, inner_size_area, buf, &mut self.list_state);
    }
}
