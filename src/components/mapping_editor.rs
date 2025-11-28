use super::Component;
use crate::event::AppMsg;
use crate::model::mapping::CursorMapping;
use crate::widgets::common::focused_block;
use crate::widgets::theme::get_theme;
use crossbeam_channel::Sender;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, Borders, Clear, List, ListItem, ListState, Paragraph, Scrollbar,
        ScrollbarOrientation, ScrollbarState, StatefulWidget, Widget,
    },
};

// Scores how well a source name matches a target standard name.
fn score_match(source: &str, target: &str) -> Option<usize> {
    let source_lower = source.to_lowercase();
    let target_lower = target.to_lowercase();

    let source_words: Vec<&str> = source_lower
        .split(|c: char| c.is_whitespace() || c == '-' || c == '_')
        .filter(|w| w.len() >= 2)
        .collect();

    let target_words: Vec<&str> = target_lower
        .split(|c: char| c.is_whitespace() || c == '-' || c == '_')
        .filter(|w| w.len() >= 2)
        .collect();

    let mut total_score = 0usize;
    let mut matched_any = false;

    for target_word in &target_words {
        let mut best_word_score = 0usize;

        for source_word in &source_words {
            let score = if source_word == target_word {
                // Exact match, highest priority
                target_word.len() * 10
            } else if source_word.starts_with(target_word) || target_word.starts_with(source_word) {
                // Prefix match, one starts with the other
                // Score based on the length of the shorter (matched) portion
                let common_len = source_word.len().min(target_word.len());
                common_len * 5
            } else if source_word.contains(target_word) || target_word.contains(source_word) {
                // Substring match
                let common_len = source_word.len().min(target_word.len());
                common_len * 2
            } else {
                0
            };

            best_word_score = best_word_score.max(score);
        }

        if best_word_score > 0 {
            matched_any = true;
            total_score += best_word_score;
        }
    }

    if matched_any { Some(total_score) } else { None }
}

// Finds the best matching source for a given target name.
// Returns the source with the highest score, preferring shorter names on ties.
fn find_best_match<'a>(sources: &'a [String], target: &str) -> Option<&'a String> {
    sources
        .iter()
        .filter_map(|source| score_match(source, target).map(|score| (source, score)))
        .max_by(|(src_a, score_a), (src_b, score_b)| {
            // Compare by score, then prefer shorter source names
            score_a
                .cmp(score_b)
                .then_with(|| src_b.len().cmp(&src_a.len()))
        })
        .map(|(source, _)| source)
}

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

    pub fn set_available_sources(&mut self, sources: Vec<String>, tx: &Sender<AppMsg>) {
        self.available_sources = sources;
        self.available_sources.sort();

        if !self.available_sources.is_empty() {
            let default_mapping = CursorMapping::default();

            for (x11_name, win_name) in &mut self.mappings_list {
                let standard_win_name = default_mapping
                    .x11_to_win
                    .get(x11_name)
                    .cloned()
                    .unwrap_or_else(|| "Normal".to_string());

                if let Some(matched_source) =
                    find_best_match(&self.available_sources, &standard_win_name)
                {
                    tx.send(AppMsg::LogMessage(format!(
                        "Matched {} (std: {}) -> {}",
                        x11_name, standard_win_name, matched_source
                    )))
                    .ok();

                    *win_name = matched_source.clone();
                    self.mapping.set_mapping(x11_name.clone(), win_name.clone());
                } else {
                    // No match found, keep the standard name (will show as Missing)
                    *win_name = standard_win_name;
                    self.mapping.set_mapping(x11_name.clone(), win_name.clone());
                }
            }
        }
    }

    fn handle_key(&mut self, key: KeyEvent) -> Option<AppMsg> {
        if self.show_popup {
            match key.code {
                KeyCode::Enter => {
                    if let Some(idx) = self.popup_state.selected()
                        && idx < self.available_sources.len()
                    {
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
        let theme = get_theme();

        let title = if self.show_popup {
            "Mapping Editor (Selecting)"
        } else {
            "Mapping Editor"
        };

        let mut block = focused_block(title, is_focused);
        if self.show_popup {
            block = block.border_style(Style::default().fg(theme.text_highlight));
        }

        let inner_area = block.inner(area);
        block.render(area, buf);

        if self.available_sources.is_empty() {
            let placeholder_text = vec![
                Line::from(Span::styled(
                    "No input folder loaded",
                    Style::default()
                        .fg(theme.text_secondary)
                        .add_modifier(Modifier::ITALIC),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    "Load an input folder to view cursor mappings",
                    Style::default().fg(theme.text_secondary),
                )),
            ];

            let placeholder = Paragraph::new(placeholder_text)
                .alignment(ratatui::layout::Alignment::Center)
                .block(Block::default());

            let v_layout = Layout::default()
                .direction(ratatui::layout::Direction::Vertical)
                .constraints([
                    Constraint::Percentage(40),
                    Constraint::Length(3),
                    Constraint::Percentage(60),
                ])
                .split(inner_area);

            placeholder.render(v_layout[1], buf);
            return;
        }

        let default_mapping = CursorMapping::default();

        let items: Vec<ListItem> = self
            .mappings_list
            .iter()
            .enumerate()
            .map(|(i, (x11_name, win_name))| {
                let style = if i == self.selected_index {
                    Style::default()
                        .fg(theme.background)
                        .bg(theme.text_highlight)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme.text_primary)
                };

                let display_win = win_name;

                let standard_mapping = default_mapping
                    .x11_to_win
                    .get(x11_name)
                    .cloned()
                    .unwrap_or_else(|| "Normal".to_string());

                // Check if source exists in available_sources
                let exists = self.available_sources.contains(display_win);
                let is_normal = display_win == "Normal";

                let source_color = if exists {
                    if is_normal {
                        theme.status_running
                    } else {
                        theme.status_completed
                    }
                } else {
                    theme.status_failed
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

                let full_source_text = if display_win != &standard_mapping {
                    format!("{}{} (std: {})", display_win, status_text, standard_mapping)
                } else {
                    format!("{}{}", display_win, status_text)
                };

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
                            theme.background
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
                                theme.background
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
                .fg(theme.background)
                .bg(theme.text_highlight)
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

        if self.show_popup {
            let popup_area = centered_rect(60, 50, area);
            Clear.render(popup_area, buf);

            let block = Block::default()
                .title("Select Source")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.border_focused));

            let inner_popup = block.inner(popup_area);
            block.render(popup_area, buf);

            let items: Vec<ListItem> = self
                .available_sources
                .iter()
                .map(|s| ListItem::new(s.as_str()).style(Style::default().fg(theme.text_primary)))
                .collect();

            let list = List::new(items).highlight_style(
                Style::default()
                    .bg(theme.border_focused)
                    .fg(theme.background)
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
