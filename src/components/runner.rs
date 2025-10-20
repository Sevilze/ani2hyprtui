use super::Component;
use crate::event::AppMsg;

#[derive(Default)]
pub struct RunnerState {
    pub logs: Vec<String>,
}

impl Component for RunnerState {
    fn update(&mut self, _msg: &AppMsg) {}
}
