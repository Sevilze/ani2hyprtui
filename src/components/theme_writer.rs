use super::Component;
use crate::event::AppMsg;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph, Widget},
};

#[derive(Default)]
pub struct ThemeWriterState {
    pub name: String,
}

impl Component for ThemeWriterState {
    fn update(&mut self, _msg: &AppMsg) -> Option<AppMsg> {
        None
    }
    
    fn render(&mut self, area: Rect, buf: &mut Buffer) {
        let block = Block::default()
            .title("Theme Writer")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Green));
        
        let text = if self.name.is_empty() {
            "No theme name set"
        } else {
            &format!("Theme: {}", self.name)
        };
        
        let paragraph = Paragraph::new(text).block(block);
        paragraph.render(area, buf);
    }
}
