use super::Component;
use crate::event::AppMsg;
use std::path::PathBuf;

#[derive(Default)]
pub struct FileBrowserState {
    pub input_dir: PathBuf,
    pub output_dir: PathBuf,
}

impl Component for FileBrowserState {
    fn update(&mut self, _msg: &AppMsg) {}
}
