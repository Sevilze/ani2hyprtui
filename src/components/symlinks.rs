use super::Component;
use crate::event::AppMsg;

#[derive(Default)]
pub struct SymlinksState {
    pub planned: Vec<(String, String)>,
}

impl Component for SymlinksState {
    fn update(&mut self, _msg: &AppMsg) {}
}
