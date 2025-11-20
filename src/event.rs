use crossterm::event::KeyEvent;
use std::path::PathBuf;

use crate::model::cursor::CursorMeta;

#[derive(Clone, Debug)]
#[allow(dead_code)]
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
    
    // Mapping changes
    MappingChanged(String, String),
    MappingSaved,
    
    // Pipeline control
    PipelineStarted,
    PipelineProgress(usize, usize),
    PipelineCompleted(usize),
    PipelineFailed(String),
    XCursorGenerated(String),
    
    // General
    ErrorOccurred(String),
    LogMessage(String),
}

