use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::App;
use super::focus::Focus;
use crate::components::Component;
use crate::event::AppMsg;

pub fn handle_key(app: &mut App, key: KeyEvent) -> bool {
    match (key.code, key.modifiers) {
        (KeyCode::Char('q'), _) | (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
            if app.focus == Focus::Mapping && app.mapping_editor.show_popup {
                if let Some(msg) = app.mapping_editor.update(&AppMsg::Key(key)) {
                    let _ = app.tx.send(msg);
                }
                return false;
            }
            true
        }
        // Window Navigation (Ctrl+hjkl or Ctrl+Arrows)
        (KeyCode::Left, KeyModifiers::CONTROL) | (KeyCode::Char('h'), KeyModifiers::CONTROL) => {
            if let Some(focus) = app.focus.left() {
                app.focus = focus;
            }
            false
        }
        (KeyCode::Right, KeyModifiers::CONTROL) | (KeyCode::Char('l'), KeyModifiers::CONTROL) => {
            if let Some(focus) = app.focus.right() {
                app.focus = focus;
            }
            false
        }
        (KeyCode::Up, KeyModifiers::CONTROL) | (KeyCode::Char('k'), KeyModifiers::CONTROL) => {
            if let Some(focus) = app.focus.up() {
                app.focus = focus;
            }
            false
        }
        (KeyCode::Down, KeyModifiers::CONTROL) | (KeyCode::Char('j'), KeyModifiers::CONTROL) => {
            if let Some(focus) = app.focus.down() {
                app.focus = focus;
            }
            false
        }
        (KeyCode::Tab, _) => {
            app.focus = app.focus.next();
            false
        }
        (KeyCode::BackTab, _) => {
            app.focus = app.focus.prev();
            false
        }
        _ => {
            let msg = AppMsg::Key(key);
            match app.focus {
                Focus::FileBrowser => match key.code {
                    KeyCode::Char('i') => {
                        let current_dir = app.file_browser.current_dir.clone();
                        let _ = app.tx.send(AppMsg::InputDirSelected(current_dir));
                    }
                    KeyCode::Char('o') => {
                        let current_dir = app.file_browser.current_dir.clone();
                        let _ = app.tx.send(AppMsg::OutputDirSelected(current_dir));
                    }
                    KeyCode::Char('l') => {
                        let target_dir = app
                            .runner
                            .input_dir
                            .clone()
                            .unwrap_or_else(|| app.file_browser.current_dir.clone());
                        let _ = app.tx.send(AppMsg::CursorSelected(target_dir));
                    }
                    _ => {
                        app.file_browser.update(&msg);
                    }
                },
                Focus::Runner => match key.code {
                    KeyCode::Char('c') => {
                        let _ = app.tx.send(AppMsg::PipelineStarted);
                    }
                    KeyCode::Char('x') => {
                        let _ = app.tx.send(AppMsg::ConvertXCursorOnly);
                    }
                    KeyCode::Char('p') => {
                        let _ = app.tx.send(AppMsg::ConvertPNGOnly);
                    }
                    _ => {
                        app.runner.update(&msg);
                    }
                },
                Focus::Overrides => {
                    if let Some(response) = app.theme_overrides.update(&msg) {
                        let _ = app.tx.send(response);
                    }
                }
                Focus::Editor => {
                    if let Some(response) = app.cursor_editor.update(&msg) {
                        let _ = app.tx.send(response);
                    }
                }
                Focus::Logs => {
                    app.logs.update(&msg);
                }
                Focus::Settings => {
                    if let Some(response) = app.settings.update(&msg) {
                        let _ = app.tx.send(response);
                    }
                }
                Focus::Hyprctl => {
                    if let Some(response) = app.hyprctl.update(&msg) {
                        let _ = app.tx.send(response);
                    }
                }
                Focus::Mapping => {
                    if let Some(response) = app.mapping_editor.update(&msg) {
                        let _ = app.tx.send(response);
                    }
                }
            }
            false
        }
    }
}
