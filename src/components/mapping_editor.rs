use super::Component;
use crate::event::AppMsg;
use crate::model::mapping::CursorMapping;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, BorderType, Borders, Clear, List, ListItem, ListState, Paragraph, Scrollbar,
        ScrollbarOrientation, ScrollbarState, StatefulWidget, Widget,
    },
};

#[derive(Default)]
pub struct MappingEditorState {
    pub mapping: CursorMapping,
    pub selected_index: usize,
    pub show_popup: bool,
    pub popup_state: ListState,
    pub mappings_list: Vec<(String, String)>,
    pub available_sources: Vec<String>,
    pub list_state: ListState,
    pub scroll_state: ScrollbarState,
    pub popup_scroll_state: ScrollbarState,
}

impl MappingEditorState {
    pub fn new(mapping: CursorMapping) -> Self {
        let mut mappings_list: Vec<(String, String)> = mapping
            .x11_to_win
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        mappings_list.sort_by(|a, b| a.0.cmp(&b.0));

        Self {
            mapping,
            selected_index: 0,
            show_popup: false,
            popup_state: ListState::default(),
            mappings_list,
            available_sources: Vec::new(),
            list_state: ListState::default(),
            scroll_state: ScrollbarState::default(),
            popup_scroll_state: ScrollbarState::default(),
        }
    }

    pub fn set_available_sources(&mut self, sources: Vec<String>) {
        self.available_sources = sources;
        self.available_sources.sort();
    }

    #[allow(dead_code)]
    pub fn load_mapping(&mut self, mapping: CursorMapping) {
        self.mappings_list = mapping
            .x11_to_win
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        self.mappings_list.sort_by(|a, b| a.0.cmp(&b.0));
        self.mapping = mapping;
        self.selected_index = 0;
        self.list_state.select(Some(0));
        self.scroll_state = self
            .scroll_state
            .content_length(self.mappings_list.len())
            .position(0);
    }

    fn handle_key(&mut self, key: KeyEvent) -> Option<AppMsg> {
        if self.show_popup {
            match key.code {
                KeyCode::Enter => {
                    if let Some(idx) = self.popup_state.selected()
                        && idx < self.available_sources.len() {
                            let x11_name = self.mappings_list[self.selected_index].0.clone();
                            let new_win_name = self.available_sources[idx].clone();

                            self.mapping
                                .set_mapping(x11_name.clone(), new_win_name.clone());
                            self.mappings_list[self.selected_index].1 = new_win_name.clone();
                            self.show_popup = false;
                            return Some(AppMsg::MappingChanged(x11_name, new_win_name));
                        }
                    self.show_popup = false;
                    None
                }
                KeyCode::Esc => {
                    self.show_popup = false;
                    None
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    let i = match self.popup_state.selected() {
                        Some(i) => {
                            if i == 0 {
                                self.available_sources.len().saturating_sub(1)
                            } else {
                                i - 1
                            }
                        }
                        None => 0,
                    };
                    self.popup_state.select(Some(i));
                    self.popup_scroll_state = self.popup_scroll_state.position(i);
                    None
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    let i = match self.popup_state.selected() {
                        Some(i) => {
                            if i >= self.available_sources.len().saturating_sub(1) {
                                0
                            } else {
                                i + 1
                            }
                        }
                        None => 0,
                    };
                    self.popup_state.select(Some(i));
                    self.popup_scroll_state = self.popup_scroll_state.position(i);
                    None
                }
                _ => None,
            }
        } else {
            match key.code {
                KeyCode::Up | KeyCode::Char('k') => {
                    if self.selected_index > 0 {
                        self.selected_index -= 1;
                        self.list_state.select(Some(self.selected_index));
                        self.scroll_state = self.scroll_state.position(self.selected_index);
                    }
                    None
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if self.selected_index < self.mappings_list.len().saturating_sub(1) {
                        self.selected_index += 1;
                        self.list_state.select(Some(self.selected_index));
                        self.scroll_state = self.scroll_state.position(self.selected_index);
                    }
                    None
                }
                KeyCode::Enter | KeyCode::Char('e') => {
                    if self.selected_index < self.mappings_list.len() {
                        self.show_popup = true;
                        let current_val = &self.mappings_list[self.selected_index].1;
                        // Find current selection in available sources
                        let initial_idx = self
                            .available_sources
                            .iter()
                            .position(|s| s == current_val)
                            .unwrap_or(0);
                        self.popup_state.select(Some(initial_idx));
                        self.popup_scroll_state = self
                            .popup_scroll_state
                            .content_length(self.available_sources.len())
                            .position(initial_idx);
                    }
                    None
                }
                KeyCode::Char('s') => Some(AppMsg::MappingSaved),
                _ => None,
            }
        }
    }
}

impl Component for MappingEditorState {
    fn update(&mut self, msg: &AppMsg) -> Option<AppMsg> {
        match msg {
            AppMsg::Key(key) => self.handle_key(*key),
            _ => None,
        }
    }

    fn render(&mut self, area: Rect, buf: &mut Buffer, is_focused: bool) {
        let chunks = Layout::default()
            .constraints([Constraint::Min(0), Constraint::Length(3)])
            .split(area);

        let title = if self.show_popup {
            "Mapping Editor (Selecting)"
        } else {
            "Mapping Editor"
        };

        let border_color = if self.show_popup {
            Color::Yellow
        } else if is_focused {
            Color::Rgb(118, 227, 73)
        } else {
            Color::Gray
        };
        let border_type = if is_focused {
            BorderType::Thick
        } else {
            BorderType::Plain
        };

        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_type(border_type)
            .border_style(Style::default().fg(border_color));

        let inner_area = block.inner(chunks[0]);
        block.render(chunks[0], buf);

        let items: Vec<ListItem> = self
            .mappings_list
            .iter()
            .enumerate()
            .map(|(i, (x11_name, win_name))| {
                let style = if i == self.selected_index {
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };

                let display_win = win_name;

                // Check if source exists in available_sources
                // "Normal" is a special fallback case, usually valid if present
                let exists = self.available_sources.contains(display_win);
                let is_normal = display_win == "Normal";

                // Visual indication logic:
                // - Cyan: Exists in input folder
                // - Red: Missing from input folder (will fallback to Normal if available)
                // - Green: Valid fallback (Normal)

                let source_color = if exists {
                    if is_normal {
                        Color::Green
                    } else {
                        Color::Cyan
                    }
                } else {
                    Color::Red
                };

                let status_text = if !exists {
                    if self.available_sources.contains(&"Normal".to_string()) {
                        " (Missing, using Normal)"
                    } else {
                        " (Missing)"
                    }
                } else {
                    ""
                };

                // Calculate available width for the source part
                let available_width = (inner_area.width as usize).saturating_sub(27);

                let full_source_text = format!("{}{}", display_win, status_text);
                let wrapped_source = textwrap::wrap(&full_source_text, available_width);

                let mut lines = Vec::new();

                let first_source_line = if !wrapped_source.is_empty() {
                    wrapped_source[0].to_string()
                } else {
                    String::new()
                };

                lines.push(Line::from(vec![
                    Span::styled(format!("{:<20}", x11_name), style),
                    Span::raw(" ← "),
                    Span::styled(
                        first_source_line,
                        style.fg(if i == self.selected_index && !self.show_popup {
                            Color::Black
                        } else {
                            source_color
                        }),
                    ),
                ]));

                // Subsequent lines: indent <- source_line_n
                for line in wrapped_source.iter().skip(1) {
                    lines.push(Line::from(vec![
                        Span::raw(" ".repeat(23)),
                        Span::styled(
                            line.to_string(),
                            style.fg(if i == self.selected_index && !self.show_popup {
                                Color::Black
                            } else {
                                source_color
                            }),
                        ),
                    ]));
                }

                ListItem::new(lines)
            })
            .collect();

        let list = List::new(items).highlight_style(
            Style::default()
                .fg(Color::Black)
                .bg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        );

        let mut list_area = inner_area;
        if list_area.width > 0 {
            list_area.width -= 1;
        }
        StatefulWidget::render(list, list_area, buf, &mut self.list_state);

        self.scroll_state = self.scroll_state.content_length(self.mappings_list.len());
        let scrollbar = Scrollbar::default()
            .orientation(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("▲"))
            .end_symbol(Some("▼"));

        scrollbar.render(inner_area, buf, &mut self.scroll_state);

        let help_text = if self.show_popup {
            "Select source: j/k to move, Enter to confirm, Esc to cancel".to_string()
        } else {
            format!(
                "{} mappings | {} available sources",
                self.mappings_list.len(),
                self.available_sources.len()
            )
        };

        // Truncate help text if too long
        let help_text = if help_text.len() > area.width as usize - 4 {
            format!("{}...", &help_text[..area.width as usize - 7])
        } else {
            help_text
        };

        let help = Paragraph::new(help_text)
            .block(Block::default().borders(Borders::ALL).title("Info"))
            .style(Style::default().fg(Color::Gray))
            .wrap(ratatui::widgets::Wrap { trim: true });
        help.render(chunks[1], buf);

        if self.show_popup {
            let popup_area = centered_rect(60, 50, area);
            Clear.render(popup_area, buf);

            let block = Block::default()
                .title("Select Source")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Blue));

            let inner_popup = block.inner(popup_area);
            block.render(popup_area, buf);

            let items: Vec<ListItem> = self
                .available_sources
                .iter()
                .map(|s| ListItem::new(s.as_str()))
                .collect();

            let list = List::new(items).highlight_style(
                Style::default()
                    .bg(Color::Blue)
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            );

            ratatui::widgets::StatefulWidget::render(list, inner_popup, buf, &mut self.popup_state);

            self.popup_scroll_state = self
                .popup_scroll_state
                .content_length(self.available_sources.len());
            let popup_scrollbar = Scrollbar::default()
                .orientation(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("▲"))
                .end_symbol(Some("▼"));

            popup_scrollbar.render(inner_popup, buf, &mut self.popup_scroll_state);
        }
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(ratatui::layout::Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
