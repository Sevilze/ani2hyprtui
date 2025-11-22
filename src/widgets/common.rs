use ratatui::{
    style::{Style, Modifier},
    widgets::{Block, BorderType, Borders},
};
use super::theme::THEME;

pub fn focused_block<'a>(title: &'a str, is_focused: bool) -> Block<'a> {
    let border_color = if is_focused {
        THEME.border_focused
    } else {
        THEME.border_unfocused
    };

    let border_type = if is_focused {
        BorderType::Thick
    } else {
        BorderType::Plain
    };

    let title_style = if is_focused {
        Style::default().fg(THEME.text_highlight).add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };

    Block::default()
        .title(title)
        .title_style(title_style)
        .borders(Borders::ALL)
        .border_type(border_type)
        .border_style(Style::default().fg(border_color))
}
