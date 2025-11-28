use ratatui::{
    style::{Style, Modifier},
    widgets::{Block, BorderType, Borders},
};
use super::theme::get_theme;

pub fn focused_block<'a>(title: &'a str, is_focused: bool) -> Block<'a> {
    let theme = get_theme();
    let border_color = if is_focused {
        theme.border_focused
    } else {
        theme.border_unfocused
    };

    let border_type = if is_focused {
        BorderType::Thick
    } else {
        BorderType::Plain
    };

    let title_style = if is_focused {
        Style::default().fg(theme.text_highlight).add_modifier(Modifier::BOLD)
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
