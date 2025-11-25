use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct Frame {
    pub png_path: PathBuf,
    pub delay_ms: u32,
}

#[derive(Clone, Debug)]
pub struct SizeVariant {
    pub size: u32,
    pub frames: Vec<Frame>,
    pub hotspot: (u32, u32),
}

#[derive(Clone, Debug, Default)]
pub struct CursorMeta {
    pub x11_name: String,
    pub variants: Vec<SizeVariant>,
}
