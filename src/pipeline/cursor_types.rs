// Cursor data types for pipeline operations

use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Frame {
    pub png_path: PathBuf,
    pub delay_ms: u32,
}

#[derive(Debug, Clone)]
pub struct SizeVariant {
    pub size: u32,
    pub frames: Vec<Frame>,
    pub hotspot: (u32, u32),
}

#[derive(Debug, Clone)]
pub struct CursorMeta {
    pub x11_name: String,
    pub win_names: Vec<String>,
    pub variants: Vec<SizeVariant>,
    pub src_cursor_path: Option<PathBuf>,
}
