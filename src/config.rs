use crate::model::mapping::CursorMapping;
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct Config {
    pub input_dir: PathBuf,
    pub output_dir: PathBuf,
    pub mapping: CursorMapping,
    pub thread_count: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            input_dir: PathBuf::from("."),
            output_dir: PathBuf::from("./out"),
            mapping: CursorMapping::default(),
            thread_count: 0,
        }
    }
}
