use crossterm::event::KeyEvent;
use std::path::PathBuf;

use crate::model::cursor::CursorMeta;

#[derive(Clone, Debug)]
pub enum AppMsg {
    Tick,
    Key(KeyEvent),
    Quit,
    
    // Folder selection
    CursorSelected(PathBuf),
    InputDirSelected(PathBuf),
    OutputDirSelected(PathBuf),
    
    // Cursor loading
    CursorLoaded(Vec<CursorMeta>),
    
    // Pipeline control
    PipelineStarted,
    PipelineProgress(usize, usize),
    PipelineCompleted(usize),
    
    // General
    ErrorOccurred(String),
    LogMessage(String),
}

