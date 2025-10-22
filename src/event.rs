use crossterm::event::KeyEvent;
use std::path::PathBuf;

use crate::model::cursor::CursorMeta;

#[derive(Clone, Debug)]
pub enum AppMsg {
    Tick,
    Key(KeyEvent),
    Quit,
    CursorSelected(PathBuf),
    CursorLoaded(Vec<CursorMeta>),
    ErrorOccurred(String),
}

