use super::Component;
use crate::event::AppMsg;

#[derive(Default)]
pub struct ThemeWriterState {
    pub name: String,
}

impl Component for ThemeWriterState {
    fn update(&mut self, _msg: &AppMsg) {}
}
