use crossterm::event::KeyEvent;
use std::path::PathBuf;

use crate::model::cursor::CursorMeta;

#[derive(Clone, Debug)]
pub enum AppMsg {
    Tick,
    Key(KeyEvent),

    // Folder selection
    CursorSelected(PathBuf),
    InputDirSelected(PathBuf),
    OutputDirSelected(PathBuf),

    // Cursor loading
    CursorLoaded(Vec<CursorMeta>),

    // Mapping changes
    MappingChanged(String, String),
    MappingSaved,
    HotspotsSaved(Vec<String>),

    // Pipeline control
    PipelineStarted,
    ConvertXCursorOnly,
    ConvertPNGOnly,
    PipelineProgress(usize, usize),
    PipelineCompleted(usize),
    PipelineFailed(String),
    XCursorGenerated(String),

    // General
    ErrorOccurred(String),
    LogMessage(String),
}
