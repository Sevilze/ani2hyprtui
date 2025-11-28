use crate::event::AppMsg;
use ratatui::{buffer::Buffer, layout::Rect};

pub mod file_browser;
pub mod hotspot_editor;
pub mod logs;
pub mod mapping_editor;
pub mod preview;
pub mod runner;
pub mod settings;
pub mod theme_overrides;

pub trait Component {
    fn update(&mut self, msg: &AppMsg) -> Option<AppMsg>;

    fn render(&mut self, area: Rect, buf: &mut Buffer, is_focused: bool);
}
