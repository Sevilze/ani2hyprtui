use super::Component;
use crate::event::AppMsg;
use crate::pipeline_worker::PipelineWorker;
use crossbeam_channel::Sender;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Widget},
};
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq)]
pub enum PipelineStatus {
    Idle,
    Running,
    Completed(usize), // number of files processed
    Failed(String),
}

pub struct RunnerState {
    pub logs: Vec<String>,
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
            logs: Vec::new(),
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
        self.add_log(format!("Input directory set: {}", self.input_dir.as_ref().unwrap().display()));
    }

    pub fn set_output_dir(&mut self, path: PathBuf) {
        self.output_dir = Some(path);
        self.add_log(format!("Output directory set: {}", self.output_dir.as_ref().unwrap().display()));
    }

    pub fn add_log(&mut self, message: String) {
        self.logs.push(message);
        if self.logs.len() > 100 {
            self.logs.remove(0);
        }
    }

    pub fn start_pipeline(&mut self) {
        if self.input_dir.is_none() || self.output_dir.is_none() {
            self.status = PipelineStatus::Failed("Input or output directory not set".to_string());
            self.add_log("ERROR: Input or output directory not set".to_string());
            return;
        }
        
        self.status = PipelineStatus::Running;
        self.files_processed = 0;
        self.add_log("Starting pipeline...".to_string());

        if let Some(tx) = &self.tx {
            let input = self.input_dir.clone().unwrap();
            let output = self.output_dir.clone().unwrap();
            let worker = PipelineWorker::new(tx.clone());
            worker.start_ani_to_png_conversion(input, output);
        }
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
        self.add_log(format!("Pipeline completed! Processed {} files", processed));
    }

    pub fn fail_pipeline(&mut self, error: String) {
        self.status = PipelineStatus::Failed(error.clone());
        self.add_log(format!("ERROR: {}", error));
    }

    pub fn clear_logs(&mut self) {
        self.logs.clear();
    }
}

impl Component for RunnerState {
    fn update(&mut self, msg: &AppMsg) -> Option<AppMsg> {
        match msg {
            AppMsg::PipelineStarted => {
                self.start_pipeline();
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
            AppMsg::LogMessage(msg) => {
                self.add_log(msg.clone());
            }
            AppMsg::ErrorOccurred(err) => {
                self.add_log(format!("Error: {}", err));
            }
            _ => {}
        }
        None
    }

    fn render(&mut self, area: Rect, buf: &mut Buffer) {
        let block = Block::default()
            .title("Pipeline Runner")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan));

        let inner = block.inner(area);
        block.render(area, buf);

        let chunks = ratatui::layout::Layout::default()
            .direction(ratatui::layout::Direction::Vertical)
            .constraints([
                ratatui::layout::Constraint::Length(6),
                ratatui::layout::Constraint::Min(1),
            ])
            .split(inner);

        let status_text = match &self.status {
            PipelineStatus::Idle => "Status: Idle\nReady to process files",
            PipelineStatus::Running => "Status: Running\nProcessing files...",
            PipelineStatus::Completed(_) => "Status: Completed",
            PipelineStatus::Failed(_) => "Status: Failed",
        };

        let mut status_lines = vec![
            Line::from(Span::styled(
                status_text,
                Style::default().fg(match &self.status {
                    PipelineStatus::Idle => Color::Yellow,
                    PipelineStatus::Running => Color::Blue,
                    PipelineStatus::Completed(_) => Color::Green,
                    PipelineStatus::Failed(_) => Color::Red,
                }),
            )),
        ];

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

        let status = Paragraph::new(status_lines)
            .block(Block::default().borders(Borders::ALL).title("Status"));
        status.render(chunks[0], buf);

        let log_items: Vec<ListItem> = self
            .logs
            .iter()
            .rev()
            .take(chunks[1].height as usize - 2)
            .rev()
            .map(|log| {
                let style = if log.contains("ERROR") {
                    Style::default().fg(Color::Red)
                } else if log.contains("completed") || log.contains("Success") {
                    Style::default().fg(Color::Green)
                } else {
                    Style::default()
                };
                ListItem::new(log.clone()).style(style)
            })
            .collect();

        let logs = List::new(log_items)
            .block(Block::default().borders(Borders::ALL).title("Logs"));
        logs.render(chunks[1], buf);
    }
}
