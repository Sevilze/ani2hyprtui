use anyhow::Result;
use crossbeam_channel::{Receiver, Sender, unbounded};
use crossterm::{
    event::{self, Event, KeyEvent},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};
use std::collections::HashSet;
use std::path::Path;
use std::{io, thread, time::Duration};

pub mod focus;
pub mod handlers;
pub mod keymap;
pub mod ui;

pub use focus::{ActivePipeline, Focus};

use crate::components::{
    Component, file_browser::FileBrowserState, hotspot_editor::HotspotEditorState,
    hyprctl::HyprctlState, logs::LogsState, mapping_editor::MappingEditorState,
    runner::RunnerState, settings::SettingsState, theme_overrides::ThemeOverridesState,
};
use crate::config::{AppConfig, Config};
use crate::event::AppMsg;
use crate::pipeline::worker::PipelineWorker;
use crate::widgets::theme::ThemeType;

pub struct App {
    pub file_browser: FileBrowserState,
    pub cursor_editor: HotspotEditorState,
    pub mapping_editor: MappingEditorState,
    pub runner: RunnerState,
    pub logs: LogsState,
    pub settings: SettingsState,
    pub hyprctl: HyprctlState,
    pub theme_overrides: ThemeOverridesState,
    pub pipeline_worker: PipelineWorker,
    pub tx: Sender<AppMsg>,
    pub rx: Receiver<AppMsg>,
    pub focus: Focus,
    pub modified_cursors: HashSet<String>,
    pub active_pipeline: Option<ActivePipeline>,
}

impl App {
    pub fn new_with_picker(picker: ratatui_image::picker::Picker) -> Self {
        let (tx, rx) = unbounded();
        let config = Config::default();

        let mut file_browser = FileBrowserState::default();
        file_browser.set_sender(tx.clone());

        let mut runner = RunnerState::default();
        runner.set_sender(tx.clone());

        if config.input_dir.as_path() != Path::new(".") {
            runner.set_input_dir(config.input_dir.clone());
        }
        runner.set_output_dir(config.output_dir.clone());

        let mapping_editor = MappingEditorState::new(config.mapping.clone());
        let pipeline_worker = PipelineWorker::new(tx.clone(), config.thread_count);

        let app_config = AppConfig::load();
        let saved_theme = app_config.theme.as_deref().and_then(ThemeType::from_name);

        if let Some(theme_type) = saved_theme {
            crate::widgets::theme::set_theme(theme_type);
        }

        let mut settings = SettingsState::default();
        if let Some(theme_type) = saved_theme {
            settings.select_theme(theme_type);
        }
        let mut hyprctl = HyprctlState::default();
        hyprctl.set_sender(tx.clone());

        let mut app = Self {
            file_browser,
            cursor_editor: HotspotEditorState::new_with_picker(picker),
            mapping_editor,
            runner,
            logs: LogsState::default(),
            settings,
            hyprctl,
            theme_overrides: ThemeOverridesState::default(),
            pipeline_worker,
            tx,
            rx,
            focus: Focus::FileBrowser,
            modified_cursors: HashSet::new(),
            active_pipeline: None,
        };
        app.sync_hyprctl_dirs();
        app
    }

    pub fn run(&mut self) -> Result<()> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;
        terminal.hide_cursor()?;

        self.start_tick_thread();

        let tick_rate = Duration::from_millis(16);
        let res: Result<()> = Ok(());

        'outer: loop {
            terminal.draw(|f| ui::draw_ui(self, f))?;

            while let Ok(msg) = self.rx.try_recv() {
                if self.update(msg) {
                    break 'outer;
                }
            }

            if event::poll(tick_rate)? {
                match event::read()? {
                    Event::Key(key) => {
                        if key.kind != crossterm::event::KeyEventKind::Release
                            && self.handle_key(key)
                        {
                            break 'outer;
                        }
                    }
                    Event::Resize(_, _) => {}
                    _ => {}
                }
            }
        }

        restore_terminal(&mut terminal)?;
        res
    }

    fn start_tick_thread(&self) {
        let tx = self.tx.clone();
        thread::spawn(move || {
            let tick_rate = Duration::from_millis(16);
            loop {
                thread::sleep(tick_rate);
                if tx.send(AppMsg::Tick).is_err() {
                    break;
                }
            }
        });
    }

    pub fn update(&mut self, msg: AppMsg) -> bool {
        match &msg {
            AppMsg::InputDirSelected(_) | AppMsg::OutputDirSelected(_) => {
                self.handle_dir_selection(&msg);
            }
            AppMsg::PipelineStarted
            | AppMsg::ConvertXCursorOnly
            | AppMsg::ConvertPNGOnly
            | AppMsg::PipelineCompleted(_)
            | AppMsg::PipelineFailed(_)
            | AppMsg::XCursorGenerated(_) => {
                self.handle_pipeline_msg(&msg);
            }
            AppMsg::HotspotsSaved(_) | AppMsg::MappingSaved => {
                self.handle_save_msg(&msg);
            }
            AppMsg::CursorSelected(_) | AppMsg::CursorLoaded(_) => {
                self.handle_cursor_msg(&msg);
            }
            AppMsg::ThreadCountChanged(count) => {
                self.pipeline_worker.set_thread_count(*count);
                let _ = self.tx.send(AppMsg::LogMessage(format!(
                    "Thread count set to {}",
                    if *count == 0 {
                        "Auto".to_string()
                    } else {
                        count.to_string()
                    }
                )));
            }
            AppMsg::ThemeSizeToggled { size, added } => {
                if *added {
                    self.cursor_editor.add_size_variant(*size);
                } else {
                    self.cursor_editor.remove_size_variant(*size);
                }
                let _ = self.tx.send(AppMsg::LogMessage(format!(
                    "Theme size {} {}",
                    size,
                    if *added { "enabled" } else { "disabled" }
                )));
            }
            AppMsg::ErrorOccurred(err) => {
                eprintln!("Error: {}", err);
            }
            _ => {}
        }

        self.update_components(&msg);
        false
    }

    pub fn update_components(&mut self, msg: &AppMsg) {
        match msg {
            AppMsg::Key(_) | AppMsg::ThemeSizeToggled { .. } => {}
            _ => {
                self.sync_hyprctl_dirs();
                self.file_browser.update(msg);
                self.cursor_editor.update(msg);
                self.runner.update(msg);
                self.logs.update(msg);
                self.settings.update(msg);
                if let Some(resp) = self.hyprctl.update(msg) {
                    let _ = self.tx.send(resp);
                }
                self.theme_overrides.update(msg);
                self.mapping_editor.update(msg);
            }
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> bool {
        keymap::handle_key(self, key)
    }
}

fn restore_terminal(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<()> {
    terminal.show_cursor().ok();
    disable_raw_mode().ok();
    let mut out = io::stdout();
    execute!(out, LeaveAlternateScreen)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use std::path::PathBuf;

    #[test]
    fn test_load_in_file_browser_uses_selected_input_dir() {
        let picker = ratatui_image::picker::Picker::halfblocks();
        let mut app = App::new_with_picker(picker);

        // Simulate user already having an input directory set
        let custom_dir = PathBuf::from("/custom/input/dir");
        app.runner.set_input_dir(custom_dir.clone());

        // File browser is in some other directory
        let other_dir = PathBuf::from("/different/browser/dir");
        app.file_browser.current_dir = other_dir;
        app.focus = Focus::FileBrowser;

        // Press 'l' in FileBrowser
        let key = KeyEvent::new(KeyCode::Char('l'), KeyModifiers::NONE);
        app.handle_key(key);

        // Check the message sent on channel: must be CursorSelected with custom_dir
        let mut found_cursor_selected = None;
        while let Ok(msg) = app.rx.try_recv() {
            if let AppMsg::CursorSelected(dir) = msg {
                found_cursor_selected = Some(dir);
                break;
            }
        }
        assert_eq!(found_cursor_selected, Some(custom_dir));
    }

    #[test]
    fn test_load_in_file_browser_falls_back_to_browser_dir_when_none_selected() {
        let picker = ratatui_image::picker::Picker::halfblocks();
        let mut app = App::new_with_picker(picker);

        app.runner.input_dir = None;
        let browser_dir = PathBuf::from("/my/browser/dir");
        app.file_browser.current_dir = browser_dir.clone();
        app.focus = Focus::FileBrowser;

        let key = KeyEvent::new(KeyCode::Char('l'), KeyModifiers::NONE);
        app.handle_key(key);

        let mut found_cursor_selected = None;
        while let Ok(msg) = app.rx.try_recv() {
            if let AppMsg::CursorSelected(dir) = msg {
                found_cursor_selected = Some(dir);
                break;
            }
        }
        assert_eq!(found_cursor_selected, Some(browser_dir));
    }
}
