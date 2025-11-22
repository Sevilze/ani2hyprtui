use super::Component;
use crate::event::AppMsg;
use crate::widgets::common::focused_block;
use crossbeam_channel::Sender;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Paragraph, Widget},
};
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq)]
pub enum PipelineStatus {
    Idle,
    Running,
    Completed(usize),
    Failed(String),
}

pub struct RunnerState {
    pub status: PipelineStatus,
    pub input_dir: Option<PathBuf>,
    pub output_dir: Option<PathBuf>,
    pub files_processed: usize,
    pub total_files: usize,
    pub tx: Option<Sender<AppMsg>>,
}

impl Default for RunnerState {
    fn default() -> Self {
        Self {
            status: PipelineStatus::Idle,
            input_dir: None,
            output_dir: None,
            files_processed: 0,
            total_files: 0,
            tx: None,
        }
    }
}

impl RunnerState {
    pub fn set_sender(&mut self, tx: Sender<AppMsg>) {
        self.tx = Some(tx);
    }
}

impl RunnerState {
    pub fn set_input_dir(&mut self, path: PathBuf) {
        self.input_dir = Some(path);
    }

    pub fn set_output_dir(&mut self, path: PathBuf) {
        self.output_dir = Some(path);
    }

    pub fn update_progress(&mut self, processed: usize, total: usize) {
        self.files_processed = processed;
        self.total_files = total;
        if processed < total {
            self.status = PipelineStatus::Running;
        }
    }

    pub fn complete_pipeline(&mut self, processed: usize) {
        self.status = PipelineStatus::Completed(processed);
    }

    pub fn fail_pipeline(&mut self, error: String) {
        self.status = PipelineStatus::Failed(error.clone());
    }
}

impl Component for RunnerState {
    fn update(&mut self, msg: &AppMsg) -> Option<AppMsg> {
        match msg {
            AppMsg::PipelineStarted => {
                self.status = PipelineStatus::Running;
                self.files_processed = 0;
            }
            AppMsg::PipelineProgress(processed, total) => {
                self.update_progress(*processed, *total);
            }
            AppMsg::PipelineCompleted(count) => {
                self.complete_pipeline(*count);
            }
            AppMsg::PipelineFailed(error) => {
                self.fail_pipeline(error.clone());
            }
            _ => {}
        }
        None
    }

    fn render(&mut self, area: Rect, buf: &mut Buffer, is_focused: bool) {
        let block = focused_block("Pipeline Runner", is_focused);
        let inner = block.inner(area);
        block.render(area, buf);

        let status_text = match &self.status {
            PipelineStatus::Idle => "Status: Idle",
            PipelineStatus::Running => "Status: Running",
            PipelineStatus::Completed(_) => "Status: Completed",
            PipelineStatus::Failed(_) => "Status: Failed",
        };

        let mut status_lines = vec![Line::from(Span::styled(
            status_text,
            Style::default().fg(match &self.status {
                PipelineStatus::Idle => Color::Yellow,
                PipelineStatus::Running => Color::Blue,
                PipelineStatus::Completed(_) => Color::Green,
                PipelineStatus::Failed(_) => Color::Red,
            }),
        ))];

        if let Some(ref input) = self.input_dir {
            status_lines.push(Line::from(format!("Input: {}", input.display())));
        }
        if let Some(ref output) = self.output_dir {
            status_lines.push(Line::from(format!("Output: {}", output.display())));
        }

        if self.status == PipelineStatus::Running {
            status_lines.push(Line::from(format!(
                "Progress: {}/{}",
                self.files_processed, self.total_files
            )));
        }

        let status = Paragraph::new(status_lines).wrap(ratatui::widgets::Wrap { trim: true });
        status.render(inner, buf);
    }
}
