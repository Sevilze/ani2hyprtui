use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{List, ListItem, Paragraph, StatefulWidget, Widget, Wrap},
};

use crate::widgets::common::focused_block;
use crate::widgets::theme::get_theme;

use super::HyprctlState;

pub fn render_hyprctl(state: &mut HyprctlState, area: Rect, buf: &mut Buffer, is_focused: bool) {
    let theme = get_theme();
    let block = focused_block("Hyprctl", is_focused);

    let inner = block.inner(area);
    block.render(area, buf);

    if inner.height == 0 || inner.width == 0 {
        return;
    }

    // Build size strings and calculate required wrapped lines
    let curr_sz = state.current_size();
    let size_str = format!("< {} >", curr_sz);

    let (sizes_summary, size_style) = if let Some(t) = state.current_theme() {
        let str_list: Vec<String> = t.available_sizes.iter().map(|s| s.to_string()).collect();
        let summary = format!(" ({})", str_list.join(", "));
        let style = if is_focused {
            Style::default()
                .fg(theme.text_highlight)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.text_primary)
        };
        (summary, style)
    } else {
        (String::new(), Style::default().fg(theme.text_primary))
    };

    let full_size_text = format!("Size: {} {}", size_str, sizes_summary);
    let wrap_width = (inner.width as usize).max(10);
    let wrapped_lines = textwrap::wrap(&full_size_text, wrap_width);
    let needed_lines = (wrapped_lines.len() as u16).max(1);
    let max_size_lines = inner.height.saturating_sub(2).max(1);
    let size_height = needed_lines.min(max_size_lines);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(1),              // Theme list
            Constraint::Length(size_height), // Size selector wraps across lines
            Constraint::Length(1),           // Status message
        ])
        .split(inner);

    let list_area = chunks[0];
    let max_name_len = list_area.width.saturating_sub(8) as usize;

    let items: Vec<ListItem> = state
        .themes
        .iter()
        .enumerate()
        .map(|(i, item)| {
            let is_active = item.name == state.active_theme;
            let marker = if is_active { "●" } else { " " };
            let tag = if item.is_hyprcursor { "[H]" } else { "[X]" };

            let display_name = if item.name.chars().count() > max_name_len {
                let truncated: String = item
                    .name
                    .chars()
                    .take(max_name_len.saturating_sub(1))
                    .collect();
                format!("{}…", truncated)
            } else {
                item.name.clone()
            };

            let text = format!("{} {} {}", marker, tag, display_name);

            let style = if i == state.selected_theme_idx {
                Style::default()
                    .fg(theme.background)
                    .bg(theme.text_highlight)
                    .add_modifier(Modifier::BOLD)
            } else if is_active {
                Style::default()
                    .fg(theme.status_completed)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.text_primary)
            };

            ListItem::new(text).style(style)
        })
        .collect();

    let list = List::new(items);
    StatefulWidget::render(list, list_area, buf, &mut state.theme_list_state);

    // Size selector with wrapping
    let size_area = chunks[1];
    let size_line = Line::from(vec![
        Span::styled("Size: ", Style::default().fg(theme.text_secondary)),
        Span::styled(size_str, size_style),
        Span::styled(sizes_summary, Style::default().fg(theme.text_secondary)),
    ]);
    Paragraph::new(size_line)
        .wrap(Wrap { trim: true })
        .render(size_area, buf);

    // Status or Hint
    let status_area = chunks[2];
    let status_line = if state.is_applying {
        Line::from(Span::styled(
            "Applying...",
            Style::default()
                .fg(theme.text_highlight)
                .add_modifier(Modifier::BOLD),
        ))
    } else if let Some((ref msg, is_err)) = state.status_message {
        let style = if is_err {
            Style::default().fg(theme.status_failed)
        } else {
            Style::default().fg(theme.status_completed)
        };
        Line::from(Span::styled(msg.as_str(), style))
    } else {
        Line::from(vec![Span::styled(
            "Enter: Apply | ←→: Size",
            Style::default().fg(theme.text_secondary),
        )])
    };
    Paragraph::new(status_line).render(status_area, buf);
}
