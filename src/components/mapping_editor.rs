use super::Component;
use crate::event::AppMsg;
use std::collections::BTreeMap;

#[derive(Default)]
pub struct MappingEditorState {
    pub win_to_x11: BTreeMap<String, String>,
}

impl Component for MappingEditorState {
    fn update(&mut self, _msg: &AppMsg) {}
}
