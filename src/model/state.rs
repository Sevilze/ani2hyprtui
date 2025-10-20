use std::path::PathBuf;
use super::cursor::CursorMeta;

#[derive(Default)]
pub struct AppState {
    pub input_dir: PathBuf,
    pub output_dir: PathBuf,
    pub theme_name: String,
    pub cursors: Vec<CursorMeta>,
    pub active_view: ActiveView,
    pub frame_ix: usize,
}

#[derive(Default)]
pub enum ActiveView {
    #[default]
    FileBrowser,
    MappingEditor,
    HotspotEditor,
    Preview,
    Runner,
    Symlinks,
    ThemeWriter,
}

