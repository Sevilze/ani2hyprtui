use ratatui::style::Color;

#[derive(Debug, Clone)]
pub struct Theme {
    pub border_focused: Color,
    pub border_unfocused: Color,
    pub text_primary: Color,
    pub text_secondary: Color,
    pub text_highlight: Color,
    pub status_idle: Color,
    pub status_running: Color,
    pub status_completed: Color,
    pub status_failed: Color,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            border_focused: Color::Rgb(118, 227, 73),
            border_unfocused: Color::White,
            text_primary: Color::White,
            text_secondary: Color::Gray,
            text_highlight: Color::Yellow,
            status_idle: Color::Yellow,
            status_running: Color::Blue,
            status_completed: Color::Green,
            status_failed: Color::Red,
        }
    }
}

pub static THEME: std::sync::LazyLock<Theme> = std::sync::LazyLock::new(Theme::default);
