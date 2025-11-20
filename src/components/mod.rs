pub mod file_browser;
pub mod mapping_editor;
pub mod hotspot_editor;
pub mod preview;
pub mod runner;
pub mod symlinks;
pub mod theme_writer;

use ratatui::{buffer::Buffer, layout::Rect};
use crate::event::AppMsg;

pub trait Component {
    fn update(&mut self, _msg: &AppMsg) -> Option<AppMsg> {
        None
    }
    fn render(&mut self, area: Rect, buf: &mut Buffer);
}

