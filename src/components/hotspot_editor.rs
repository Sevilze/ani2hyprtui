use super::Component;
use crate::event::AppMsg;

#[derive(Default)]
pub struct HotspotEditorState {
    pub size: u32,
    pub hotspot: (u32, u32),
}

impl Component for HotspotEditorState {
    fn update(&mut self, _msg: &AppMsg) {}
}
