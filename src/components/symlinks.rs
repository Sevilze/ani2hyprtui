use super::Component;
use crate::event::AppMsg;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph, Widget},
};

#[derive(Default)]
pub struct SymlinksState {
    pub planned: Vec<(String, String)>,
}

impl Component for SymlinksState {
    fn update(&mut self, _msg: &AppMsg) -> Option<AppMsg> {
        None
    }
    
    fn render(&mut self, area: Rect, buf: &mut Buffer) {
        let block = Block::default()
            .title("Symlinks")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Magenta));
        
        let text = format!("{} symlinks planned", self.planned.len());
        let paragraph = Paragraph::new(text).block(block);
        paragraph.render(area, buf);
    }
}
