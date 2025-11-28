use ratatui::style::Color;
use std::sync::{LazyLock, RwLock};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemeType {
    CatppuccinMocha,
    CatppuccinMacchiato,
    CatppuccinFrappe,
    CatppuccinLatte,
    TokyoNight,
    TokyoNightStorm,
    TokyoNightDay,
    Gruvbox,
    GruvboxLight,
    Nord,
    Dracula,
    OneDark,
    SolarizedDark,
    SolarizedLight,
}

impl ThemeType {
    pub fn all() -> Vec<ThemeType> {
        vec![
            ThemeType::CatppuccinMocha,
            ThemeType::CatppuccinMacchiato,
            ThemeType::CatppuccinFrappe,
            ThemeType::CatppuccinLatte,
            ThemeType::TokyoNight,
            ThemeType::TokyoNightStorm,
            ThemeType::TokyoNightDay,
            ThemeType::Gruvbox,
            ThemeType::GruvboxLight,
            ThemeType::Nord,
            ThemeType::Dracula,
            ThemeType::OneDark,
            ThemeType::SolarizedDark,
            ThemeType::SolarizedLight,
        ]
    }

    pub fn name(&self) -> &str {
        match self {
            ThemeType::CatppuccinMocha => "Catppuccin Mocha",
            ThemeType::CatppuccinMacchiato => "Catppuccin Macchiato",
            ThemeType::CatppuccinFrappe => "Catppuccin Frappé",
            ThemeType::CatppuccinLatte => "Catppuccin Latte",
            ThemeType::TokyoNight => "Tokyo Night",
            ThemeType::TokyoNightStorm => "Tokyo Night Storm",
            ThemeType::TokyoNightDay => "Tokyo Night Day",
            ThemeType::Gruvbox => "Gruvbox Dark",
            ThemeType::GruvboxLight => "Gruvbox Light",
            ThemeType::Nord => "Nord",
            ThemeType::Dracula => "Dracula",
            ThemeType::OneDark => "One Dark",
            ThemeType::SolarizedDark => "Solarized Dark",
            ThemeType::SolarizedLight => "Solarized Light",
        }
    }

    pub fn next(&self) -> ThemeType {
        let all = Self::all();
        let idx = all.iter().position(|t| t == self).unwrap_or(0);
        all[(idx + 1) % all.len()]
    }

    pub fn prev(&self) -> ThemeType {
        let all = Self::all();
        let idx = all.iter().position(|t| t == self).unwrap_or(0);
        all[(idx + all.len() - 1) % all.len()]
    }
}

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
    pub background: Color,
    pub surface: Color,
}

impl Theme {
    pub fn from_type(theme_type: ThemeType) -> Self {
        match theme_type {
            ThemeType::CatppuccinMocha => Self::catppuccin_mocha(),
            ThemeType::CatppuccinMacchiato => Self::catppuccin_macchiato(),
            ThemeType::CatppuccinFrappe => Self::catppuccin_frappe(),
            ThemeType::CatppuccinLatte => Self::catppuccin_latte(),
            ThemeType::TokyoNight => Self::tokyo_night(),
            ThemeType::TokyoNightStorm => Self::tokyo_night_storm(),
            ThemeType::TokyoNightDay => Self::tokyo_night_day(),
            ThemeType::Gruvbox => Self::gruvbox(),
            ThemeType::GruvboxLight => Self::gruvbox_light(),
            ThemeType::Nord => Self::nord(),
            ThemeType::Dracula => Self::dracula(),
            ThemeType::OneDark => Self::one_dark(),
            ThemeType::SolarizedDark => Self::solarized_dark(),
            ThemeType::SolarizedLight => Self::solarized_light(),
        }
    }

    fn catppuccin_mocha() -> Self {
        Self {
            border_focused: Color::Rgb(137, 220, 235),   // Sky
            border_unfocused: Color::Rgb(108, 112, 134), // Overlay0
            text_primary: Color::Rgb(205, 214, 244),     // Text
            text_secondary: Color::Rgb(147, 153, 178),   // Subtext0
            text_highlight: Color::Rgb(249, 226, 175),   // Yellow
            status_idle: Color::Rgb(166, 227, 161),      // Green
            status_running: Color::Rgb(137, 180, 250),   // Blue
            status_completed: Color::Rgb(166, 227, 161), // Green
            status_failed: Color::Rgb(243, 139, 168),    // Red
            background: Color::Rgb(30, 30, 46),          // Base
            surface: Color::Rgb(49, 50, 68),             // Surface0
        }
    }

    fn catppuccin_macchiato() -> Self {
        Self {
            border_focused: Color::Rgb(145, 215, 227),
            border_unfocused: Color::Rgb(110, 115, 141),
            text_primary: Color::Rgb(202, 211, 245),
            text_secondary: Color::Rgb(147, 154, 183),
            text_highlight: Color::Rgb(238, 212, 159),
            status_idle: Color::Rgb(166, 218, 149),
            status_running: Color::Rgb(138, 173, 244),
            status_completed: Color::Rgb(166, 218, 149),
            status_failed: Color::Rgb(237, 135, 150),
            background: Color::Rgb(36, 39, 58),
            surface: Color::Rgb(54, 58, 79),
        }
    }

    fn catppuccin_frappe() -> Self {
        Self {
            border_focused: Color::Rgb(153, 209, 219),
            border_unfocused: Color::Rgb(115, 121, 148),
            text_primary: Color::Rgb(198, 208, 245),
            text_secondary: Color::Rgb(150, 157, 190),
            text_highlight: Color::Rgb(229, 200, 144),
            status_idle: Color::Rgb(166, 209, 137),
            status_running: Color::Rgb(140, 170, 238),
            status_completed: Color::Rgb(166, 209, 137),
            status_failed: Color::Rgb(231, 130, 132),
            background: Color::Rgb(48, 52, 70),
            surface: Color::Rgb(65, 69, 89),
        }
    }

    fn catppuccin_latte() -> Self {
        Self {
            border_focused: Color::Rgb(4, 165, 229),
            border_unfocused: Color::Rgb(156, 160, 176),
            text_primary: Color::Rgb(76, 79, 105),
            text_secondary: Color::Rgb(108, 111, 133),
            text_highlight: Color::Rgb(223, 142, 29),
            status_idle: Color::Rgb(64, 160, 43),
            status_running: Color::Rgb(30, 102, 245),
            status_completed: Color::Rgb(64, 160, 43),
            status_failed: Color::Rgb(210, 15, 57),
            background: Color::Rgb(239, 241, 245),
            surface: Color::Rgb(230, 233, 239),
        }
    }

    fn tokyo_night() -> Self {
        Self {
            border_focused: Color::Rgb(125, 207, 255),
            border_unfocused: Color::Rgb(86, 95, 137),
            text_primary: Color::Rgb(169, 177, 214),
            text_secondary: Color::Rgb(120, 127, 168),
            text_highlight: Color::Rgb(224, 175, 104),
            status_idle: Color::Rgb(158, 206, 106),
            status_running: Color::Rgb(122, 162, 247),
            status_completed: Color::Rgb(158, 206, 106),
            status_failed: Color::Rgb(247, 118, 142),
            background: Color::Rgb(26, 27, 38),
            surface: Color::Rgb(30, 32, 48),
        }
    }

    fn tokyo_night_storm() -> Self {
        Self {
            border_focused: Color::Rgb(125, 207, 255),
            border_unfocused: Color::Rgb(86, 95, 137),
            text_primary: Color::Rgb(169, 177, 214),
            text_secondary: Color::Rgb(120, 127, 168),
            text_highlight: Color::Rgb(224, 175, 104),
            status_idle: Color::Rgb(158, 206, 106),
            status_running: Color::Rgb(122, 162, 247),
            status_completed: Color::Rgb(158, 206, 106),
            status_failed: Color::Rgb(247, 118, 142),
            background: Color::Rgb(36, 40, 59),
            surface: Color::Rgb(42, 46, 68),
        }
    }

    fn tokyo_night_day() -> Self {
        Self {
            border_focused: Color::Rgb(52, 124, 203),
            border_unfocused: Color::Rgb(142, 145, 165),
            text_primary: Color::Rgb(52, 59, 88),
            text_secondary: Color::Rgb(90, 100, 126),
            text_highlight: Color::Rgb(143, 94, 21),
            status_idle: Color::Rgb(51, 153, 51),
            status_running: Color::Rgb(52, 124, 203),
            status_completed: Color::Rgb(51, 153, 51),
            status_failed: Color::Rgb(204, 51, 51),
            background: Color::Rgb(230, 236, 248),
            surface: Color::Rgb(220, 226, 240),
        }
    }

    fn gruvbox() -> Self {
        Self {
            border_focused: Color::Rgb(142, 192, 124),
            border_unfocused: Color::Rgb(146, 131, 116),
            text_primary: Color::Rgb(235, 219, 178),
            text_secondary: Color::Rgb(168, 153, 132),
            text_highlight: Color::Rgb(250, 189, 47),
            status_idle: Color::Rgb(184, 187, 38),
            status_running: Color::Rgb(131, 165, 152),
            status_completed: Color::Rgb(142, 192, 124),
            status_failed: Color::Rgb(251, 73, 52),
            background: Color::Rgb(40, 40, 40),
            surface: Color::Rgb(60, 56, 54),
        }
    }

    fn gruvbox_light() -> Self {
        Self {
            border_focused: Color::Rgb(121, 116, 14),
            border_unfocused: Color::Rgb(189, 174, 147),
            text_primary: Color::Rgb(60, 56, 54),
            text_secondary: Color::Rgb(102, 92, 84),
            text_highlight: Color::Rgb(181, 118, 20),
            status_idle: Color::Rgb(121, 116, 14),
            status_running: Color::Rgb(69, 133, 136),
            status_completed: Color::Rgb(121, 116, 14),
            status_failed: Color::Rgb(204, 36, 29),
            background: Color::Rgb(251, 241, 199),
            surface: Color::Rgb(235, 219, 178),
        }
    }

    fn nord() -> Self {
        Self {
            border_focused: Color::Rgb(136, 192, 208),
            border_unfocused: Color::Rgb(76, 86, 106),
            text_primary: Color::Rgb(236, 239, 244),
            text_secondary: Color::Rgb(216, 222, 233),
            text_highlight: Color::Rgb(235, 203, 139),
            status_idle: Color::Rgb(163, 190, 140),
            status_running: Color::Rgb(129, 161, 193),
            status_completed: Color::Rgb(163, 190, 140),
            status_failed: Color::Rgb(191, 97, 106),
            background: Color::Rgb(46, 52, 64),
            surface: Color::Rgb(59, 66, 82),
        }
    }

    fn dracula() -> Self {
        Self {
            border_focused: Color::Rgb(139, 233, 253),
            border_unfocused: Color::Rgb(98, 114, 164),
            text_primary: Color::Rgb(248, 248, 242),
            text_secondary: Color::Rgb(189, 147, 249),
            text_highlight: Color::Rgb(241, 250, 140),
            status_idle: Color::Rgb(80, 250, 123),
            status_running: Color::Rgb(139, 233, 253),
            status_completed: Color::Rgb(80, 250, 123),
            status_failed: Color::Rgb(255, 85, 85),
            background: Color::Rgb(40, 42, 54),
            surface: Color::Rgb(68, 71, 90),
        }
    }

    fn one_dark() -> Self {
        Self {
            border_focused: Color::Rgb(97, 175, 239),
            border_unfocused: Color::Rgb(92, 99, 112),
            text_primary: Color::Rgb(171, 178, 191),
            text_secondary: Color::Rgb(130, 137, 151),
            text_highlight: Color::Rgb(229, 192, 123),
            status_idle: Color::Rgb(152, 195, 121),
            status_running: Color::Rgb(97, 175, 239),
            status_completed: Color::Rgb(152, 195, 121),
            status_failed: Color::Rgb(224, 108, 117),
            background: Color::Rgb(40, 44, 52),
            surface: Color::Rgb(53, 59, 69),
        }
    }

    fn solarized_dark() -> Self {
        Self {
            border_focused: Color::Rgb(42, 161, 152),
            border_unfocused: Color::Rgb(88, 110, 117),
            text_primary: Color::Rgb(131, 148, 150),
            text_secondary: Color::Rgb(101, 123, 131),
            text_highlight: Color::Rgb(181, 137, 0),
            status_idle: Color::Rgb(133, 153, 0),
            status_running: Color::Rgb(38, 139, 210),
            status_completed: Color::Rgb(133, 153, 0),
            status_failed: Color::Rgb(220, 50, 47),
            background: Color::Rgb(0, 43, 54),
            surface: Color::Rgb(7, 54, 66),
        }
    }

    fn solarized_light() -> Self {
        Self {
            border_focused: Color::Rgb(38, 139, 210),
            border_unfocused: Color::Rgb(147, 161, 161),
            text_primary: Color::Rgb(101, 123, 131),
            text_secondary: Color::Rgb(131, 148, 150),
            text_highlight: Color::Rgb(181, 137, 0),
            status_idle: Color::Rgb(133, 153, 0),
            status_running: Color::Rgb(38, 139, 210),
            status_completed: Color::Rgb(133, 153, 0),
            status_failed: Color::Rgb(220, 50, 47),
            background: Color::Rgb(253, 246, 227),
            surface: Color::Rgb(238, 232, 213),
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::catppuccin_mocha()
    }
}

pub static CURRENT_THEME: LazyLock<RwLock<ThemeType>> =
    LazyLock::new(|| RwLock::new(ThemeType::CatppuccinMocha));

pub fn get_theme() -> Theme {
    let theme_type = *CURRENT_THEME.read().unwrap_or_else(|e| e.into_inner());
    Theme::from_type(theme_type)
}

pub fn set_theme(theme_type: ThemeType) {
    if let Ok(mut guard) = CURRENT_THEME.write() {
        *guard = theme_type;
    }
}

pub fn get_current_theme_type() -> ThemeType {
    *CURRENT_THEME.read().unwrap_or_else(|e| e.into_inner())
}
