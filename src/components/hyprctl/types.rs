use std::path::PathBuf;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CursorThemeItem {
    pub name: String,
    pub path: PathBuf,
    pub is_hyprcursor: bool,
    pub available_sizes: Vec<u32>,
}
