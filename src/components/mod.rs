pub mod file_browser;
pub mod mapping_editor;
pub mod hotspot_editor;
pub mod preview;
pub mod runner;
pub mod symlinks;
pub mod theme_writer;

pub trait Component {
    fn update(&mut self, _msg: &crate::event::AppMsg) {}
}

