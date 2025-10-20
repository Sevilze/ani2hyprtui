use crate::event::AppMsg;
use super::Component;

#[derive(Default)]
pub struct PreviewState {
    pub frame_ix: usize,
    pub playing: bool,
}

impl Component for PreviewState {
    fn update(&mut self, _msg: &AppMsg) {}
}

