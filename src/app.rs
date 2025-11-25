use anyhow::Result;
use crossbeam_channel::{Receiver, Sender, unbounded};
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Style},
    widgets::Paragraph,
};
use std::collections::{HashMap, HashSet};
use std::{io, thread, time::Duration};

use crate::components::{
    Component, file_browser::FileBrowserState, hotspot_editor::HotspotEditorState, logs::LogsState,
    mapping_editor::MappingEditorState, runner::RunnerState, theme_overrides::ThemeOverridesState,
};
use crate::config::Config;
use crate::event::AppMsg;
use crate::model::cursor;
use crate::pipeline::cursor_io::{load_cursor_folder, load_cursor_folder_from_pngs};
use crate::pipeline_worker::PipelineWorker;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    FileBrowser,
    Runner,
    Overrides,
    Editor,
    Logs,
    Mapping,
}

impl Focus {
    fn next(&self, show_mapping: bool) -> Self {
        match self {
            Focus::FileBrowser => Focus::Runner,
            Focus::Runner => Focus::Overrides,
            Focus::Overrides => Focus::Editor,
            Focus::Editor => Focus::Logs,
            Focus::Logs => {
                if show_mapping {
                    Focus::Mapping
                } else {
                    Focus::FileBrowser
                }
            }
            Focus::Mapping => Focus::FileBrowser,
        }
    }

    fn prev(&self, show_mapping: bool) -> Self {
        match self {
            Focus::FileBrowser => {
                if show_mapping {
                    Focus::Mapping
                } else {
                    Focus::Logs
                }
            }
            Focus::Runner => Focus::FileBrowser,
            Focus::Overrides => Focus::Runner,
            Focus::Editor => Focus::Overrides,
            Focus::Logs => Focus::Editor,
            Focus::Mapping => Focus::Logs,
        }
    }

    fn left(&self) -> Option<Self> {
        match self {
            Focus::Editor => Some(Focus::FileBrowser),
            Focus::Logs => Some(Focus::Overrides),
            Focus::Mapping => Some(Focus::Editor),
            _ => None,
        }
    }

    fn right(&self, show_mapping: bool) -> Option<Self> {
        match self {
            Focus::FileBrowser => Some(Focus::Editor),
            Focus::Runner => Some(Focus::Editor),
            Focus::Overrides => Some(Focus::Logs),
            Focus::Editor | Focus::Logs => {
                if show_mapping {
                    Some(Focus::Mapping)
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    fn up(&self) -> Option<Self> {
        match self {
            Focus::Runner => Some(Focus::FileBrowser),
            Focus::Overrides => Some(Focus::Runner),
            Focus::Logs => Some(Focus::Editor),
            _ => None,
        }
    }

    fn down(&self) -> Option<Self> {
        match self {
            Focus::FileBrowser => Some(Focus::Runner),
            Focus::Runner => Some(Focus::Overrides),
            Focus::Editor => Some(Focus::Logs),
            _ => None,
        }
    }
}

pub struct App {
    pub file_browser: FileBrowserState,
    pub cursor_editor: HotspotEditorState,
    pub mapping_editor: MappingEditorState,
    pub runner: RunnerState,
    pub logs: LogsState,
    pub theme_overrides: ThemeOverridesState,
    pub pipeline_worker: PipelineWorker,
    pub tx: Sender<AppMsg>,
    pub rx: Receiver<AppMsg>,
    pub focus: Focus,
    pub modified_cursors: HashSet<String>,
}

impl App {
    pub fn new_with_picker(picker: ratatui_image::picker::Picker) -> Self {
        let (tx, rx) = unbounded();
        let config = Config::default();

        let mut file_browser = FileBrowserState::default();
        file_browser.set_sender(tx.clone());

        let mut runner = RunnerState::default();
        runner.set_sender(tx.clone());

        // Only set input dir if it's not the default ".", so mapping editor starts hidden
        if config.input_dir != std::path::PathBuf::from(".") {
            runner.set_input_dir(config.input_dir.clone());
        }
        runner.set_output_dir(config.output_dir.clone());

        let mapping_editor = MappingEditorState::new(config.mapping.clone());

        let pipeline_worker = PipelineWorker::new(tx.clone());

        Self {
            file_browser,
            cursor_editor: HotspotEditorState::new_with_picker(picker),
            mapping_editor,
            runner,
            logs: LogsState::default(),
            theme_overrides: ThemeOverridesState::default(),
            pipeline_worker,
            tx,
            rx,
            focus: Focus::FileBrowser,
            modified_cursors: HashSet::new(),
        }
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
        let mut res: Result<()> = Ok(());

        'outer: loop {
            terminal.draw(|f| {
                let area = f.area();

                // Main layout: vertical split into content and status bar
                let main_chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Min(1), Constraint::Length(1)])
                    .split(area);

                let show_mapping = self.runner.input_dir.is_some();

                if self.cursor_editor.maximized {
                    self.cursor_editor
                        .render(main_chunks[0], f.buffer_mut(), true);
                } else {
                    let constraints = if show_mapping {
                        vec![
                            Constraint::Percentage(25), // Left: File Browser, Runner, Overrides
                            Constraint::Percentage(50), // Middle: Cursor Editor, Logs
                            Constraint::Percentage(25), // Right: Mapping Editor
                        ]
                    } else {
                        vec![
                            Constraint::Percentage(30), // Left
                            Constraint::Percentage(70), // Middle
                        ]
                    };

                    let columns = Layout::default()
                        .direction(Direction::Horizontal)
                        .constraints(constraints)
                        .split(main_chunks[0]);

                    // Left Column
                    let left_chunks = Layout::default()
                        .direction(Direction::Vertical)
                        .constraints([
                            Constraint::Percentage(40), // File Browser
                            Constraint::Percentage(20), // Runner
                            Constraint::Percentage(40), // Overrides
                        ])
                        .split(columns[0]);

                    // Middle Column
                    let middle_chunks = Layout::default()
                        .direction(Direction::Vertical)
                        .constraints([
                            Constraint::Percentage(70), // Cursor Editor
                            Constraint::Percentage(30), // Logs
                        ])
                        .split(columns[1]);

                    // Render components
                    self.file_browser.render(
                        left_chunks[0],
                        f.buffer_mut(),
                        self.focus == Focus::FileBrowser,
                    );
                    self.runner
                        .render(left_chunks[1], f.buffer_mut(), self.focus == Focus::Runner);
                    self.theme_overrides.render(
                        left_chunks[2],
                        f.buffer_mut(),
                        self.focus == Focus::Overrides,
                    );

                    self.cursor_editor.render(
                        middle_chunks[0],
                        f.buffer_mut(),
                        self.focus == Focus::Editor,
                    );
                    self.logs
                        .render(middle_chunks[1], f.buffer_mut(), self.focus == Focus::Logs);

                    if show_mapping {
                        self.mapping_editor.render(
                            columns[2],
                            f.buffer_mut(),
                            self.focus == Focus::Mapping,
                        );
                    }
                }

                // Status bar
                let focus_str = format!("{:?}", self.focus);
                let status_text = format!(
                    "q: Quit | Ctrl+hjkl: Navigate | Focus: {} | {}",
                    focus_str,
                    match self.focus {
                        Focus::FileBrowser => "i/o: Set In/Out | Enter: Select | l: Load",
                        Focus::Runner => "c: Full Convert | x: XCur | p: PNG",
                        Focus::Overrides => "Tab: Switch Field | Type to edit",
                        Focus::Editor => "Space: Play | ,/.: Frame | Arrows: Hotspot | S: Save",
                        Focus::Logs => "Logs View",
                        Focus::Mapping => "Enter: Edit | s: Save",
                    }
                );

                let status = Paragraph::new(status_text)
                    .style(Style::default().fg(Color::Gray))
                    .alignment(Alignment::Center);
                f.render_widget(status, main_chunks[1]);
            })?;

            // Check for messages from tick thread or other sources
            while let Ok(msg) = self.rx.try_recv() {
                if self.handle_message(msg) {
                    break 'outer;
                }
            }

            // Poll for keyboard events
            if event::poll(tick_rate)? {
                match event::read()? {
                    Event::Key(key) => {
                        if self.handle_key(key) {
                            break 'outer;
                        }
                    }
                    Event::Resize(_, _) => {}
                    _ => {}
                }
            }
        }

        // Restore terminal
        if let Err(e) = restore_terminal(&mut terminal) {
            res = Err(e);
        }
        res
    }

    fn start_tick_thread(&self) {
        let tx = self.tx.clone();
        thread::spawn(move || {
            loop {
                thread::sleep(Duration::from_millis(16));
                if tx.send(AppMsg::Tick).is_err() {
                    break;
                }
            }
        });
    }

    fn handle_message(&mut self, msg: AppMsg) -> bool {
        match &msg {
            AppMsg::Tick => {
                // Tick is handled by Editor component for animation
            }
            AppMsg::MappingChanged(x11_name, _win_name) => {
                self.modified_cursors.insert(x11_name.clone());
            }
            AppMsg::InputDirSelected(_) | AppMsg::OutputDirSelected(_) => {
                self.handle_dir_selection(&msg);
            }
            AppMsg::PipelineStarted
            | AppMsg::ConvertXCursorOnly
            | AppMsg::ConvertPNGOnly
            | AppMsg::PipelineCompleted(_)
            | AppMsg::XCursorGenerated(_) => {
                self.handle_pipeline_msg(&msg);
            }
            AppMsg::HotspotsSaved(_) | AppMsg::MappingSaved => {
                self.handle_save_msg(&msg);
            }
            AppMsg::CursorSelected(_) | AppMsg::CursorLoaded(_) => {
                self.handle_cursor_msg(&msg);
            }
            AppMsg::ErrorOccurred(err) => {
                eprintln!("Error: {}", err);
            }
            _ => {}
        }

        self.update_components(&msg);
        false
    }

    fn handle_dir_selection(&mut self, msg: &AppMsg) {
        match msg {
            AppMsg::InputDirSelected(path) => {
                self.runner.set_input_dir(path.clone());
                // Scan directory for available sources (.ani/.cur files)
                let mut sources = Vec::new();
                if let Ok(entries) = std::fs::read_dir(path) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if let Some(ext) = path.extension() {
                            let ext_str = ext.to_string_lossy().to_lowercase();
                            if (ext_str == "ani" || ext_str == "cur")
                                && let Some(stem) = path.file_stem()
                            {
                                sources.push(stem.to_string_lossy().to_string());
                            }
                        }
                    }
                }
                self.mapping_editor.set_available_sources(sources);
            }
            AppMsg::OutputDirSelected(path) => {
                self.runner.set_output_dir(path.clone());
            }
            _ => {}
        }
    }

    fn handle_pipeline_msg(&mut self, msg: &AppMsg) {
        match msg {
            AppMsg::PipelineStarted => {
                if let (Some(input_dir), Some(output_dir)) = (
                    self.runner.input_dir.clone(),
                    self.runner.output_dir.clone(),
                ) {
                    let theme_name = input_dir
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("ConvertedCursors")
                        .to_string();
                    let mapping = self.mapping_editor.mapping.clone();
                    let selected_sizes: Vec<u32> = self
                        .theme_overrides
                        .selected_sizes
                        .iter()
                        .cloned()
                        .collect();

                    self.pipeline_worker.start_full_theme_conversion(
                        input_dir.clone(),
                        output_dir.clone(),
                        theme_name,
                        mapping,
                        selected_sizes,
                    );
                }
            }
            AppMsg::ConvertXCursorOnly => {
                if let (Some(input_dir), Some(output_dir)) = (
                    self.runner.input_dir.clone(),
                    self.runner.output_dir.clone(),
                ) {
                    self.pipeline_worker
                        .start_ani_to_xcur_conversion(input_dir, output_dir);
                }
            }
            AppMsg::ConvertPNGOnly => {
                if let (Some(input_dir), Some(output_dir)) = (
                    self.runner.input_dir.clone(),
                    self.runner.output_dir.clone(),
                ) {
                    self.pipeline_worker
                        .start_ani_to_png_conversion(input_dir, output_dir);
                }
            }
            AppMsg::PipelineCompleted(_count) => {
                if let Some(output_dir) = &self.runner.output_dir {
                    let png_dir = output_dir.join("png_intermediate");
                    if png_dir.exists() {
                        let _ = self.tx.send(AppMsg::CursorSelected(png_dir));
                    }
                }
            }
            AppMsg::XCursorGenerated(path) => {
                let _ = self.tx.send(AppMsg::LogMessage(format!(
                    "XCursor theme generated at: {}",
                    path
                )));
            }
            _ => {}
        }
    }

    fn handle_save_msg(&mut self, msg: &AppMsg) {
        match msg {
            AppMsg::HotspotsSaved(modified_cursors) => {
                for c in modified_cursors {
                    self.modified_cursors.insert(c.clone());
                }
                let _ = self.tx.send(AppMsg::MappingSaved);
            }
            AppMsg::MappingSaved => {
                let _ = self.tx.send(AppMsg::LogMessage(
                    "Saving changes. Triggering incremental update...".to_string(),
                ));

                if let (Some(input_dir), Some(output_dir)) = (
                    self.runner.input_dir.clone(),
                    self.runner.output_dir.clone(),
                ) {
                    let theme_name = input_dir
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("ConvertedCursors")
                        .to_string();
                    let mapping = self.mapping_editor.mapping.clone();

                    if self.modified_cursors.is_empty() {
                        let _ = self.tx.send(AppMsg::LogMessage(
                            "No changes detected since last save.".to_string(),
                        ));
                    } else {
                        let modified: Vec<String> = self.modified_cursors.drain().collect();
                        let _ = self.tx.send(AppMsg::LogMessage(format!(
                            "Updating {} modified cursors...",
                            modified.len()
                        )));

                        let mut hotspot_overrides = HashMap::new();
                        for cursor_name in &modified {
                            if let Some(cursor) = self
                                .cursor_editor
                                .cursors
                                .iter()
                                .find(|c| c.x11_name == *cursor_name)
                            {
                                let mut variants_map = HashMap::new();
                                for variant in &cursor.variants {
                                    variants_map.insert(variant.size, variant.hotspot);
                                }
                                hotspot_overrides.insert(cursor_name.clone(), variants_map);
                            }
                        }

                        self.pipeline_worker.start_incremental_theme_update(
                            input_dir,
                            output_dir,
                            theme_name,
                            mapping,
                            modified,
                            hotspot_overrides,
                        );
                    }
                } else {
                    let _ = self.tx.send(AppMsg::LogMessage(
                        "Cannot update theme: Input or Output directory not set.".to_string(),
                    ));
                }
            }
            _ => {}
        }
    }

    fn handle_cursor_msg(&mut self, msg: &AppMsg) {
        match msg {
            AppMsg::CursorSelected(path) => {
                let _ = self.tx.send(AppMsg::LogMessage(format!(
                    "Loading cursors from: {}",
                    path.display()
                )));

                let cursors = load_cursor_folder_from_pngs(path).or_else(|e| {
                    let _ = self.tx.send(AppMsg::LogMessage(format!(
                        "PNG load failed: {}, trying binary...",
                        e
                    )));
                    load_cursor_folder(path)
                });

                match cursors {
                    Ok(cursors) => {
                        let _ = self.tx.send(AppMsg::LogMessage(format!(
                            "Loaded {} cursors",
                            cursors.len()
                        )));

                        let mut converted_cursors: Vec<cursor::CursorMeta> = cursors
                            .into_iter()
                            .map(|c| {
                                let mut variants: Vec<cursor::SizeVariant> = c
                                    .variants
                                    .into_iter()
                                    .map(|v| cursor::SizeVariant {
                                        size: v.size,
                                        frames: v
                                            .frames
                                            .into_iter()
                                            .map(|f| cursor::Frame {
                                                png_path: f.png_path,
                                                delay_ms: f.delay_ms,
                                            })
                                            .collect(),
                                        hotspot: v.hotspot,
                                    })
                                    .collect();
                                variants.sort_by_key(|v| v.size);

                                cursor::CursorMeta {
                                    x11_name: c.x11_name,
                                    variants,
                                }
                            })
                            .collect();

                        converted_cursors.sort_by(|a, b| a.x11_name.cmp(&b.x11_name));

                        if !converted_cursors.is_empty() {
                            let _ = self.tx.send(AppMsg::LogMessage(format!(
                                "Sending {} cursors to editor",
                                converted_cursors.len()
                            )));
                            let _ = self.tx.send(AppMsg::CursorLoaded(converted_cursors));
                        } else {
                            let _ = self.tx.send(AppMsg::LogMessage(
                                "No cursors found in selected directory".to_string(),
                            ));
                        }
                    }
                    Err(e) => {
                        let _ = self.tx.send(AppMsg::ErrorOccurred(format!(
                            "Failed to load cursors: {}",
                            e
                        )));
                    }
                }
            }
            AppMsg::CursorLoaded(_) => {
                // Handled by Editor component
            }
            _ => {}
        }
    }

    fn update_components(&mut self, msg: &AppMsg) {
        match msg {
            AppMsg::Key(_) => {}
            _ => {
                self.file_browser.update(msg);
                self.cursor_editor.update(msg);
                self.runner.update(msg);
                self.logs.update(msg);
                self.theme_overrides.update(msg);
                self.mapping_editor.update(msg);
            }
        }
    }

    fn handle_key(&mut self, key: KeyEvent) -> bool {
        match (key.code, key.modifiers) {
            (KeyCode::Char('q'), _) | (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
                if self.focus == Focus::Mapping && self.mapping_editor.show_popup {
                    if let Some(msg) = self.mapping_editor.update(&AppMsg::Key(key)) {
                        let _ = self.tx.send(msg);
                    }
                    return false;
                }
                return true;
            }
            // Window Navigation (Ctrl+hjkl or Ctrl+Arrows)
            (KeyCode::Left, KeyModifiers::CONTROL)
            | (KeyCode::Char('h'), KeyModifiers::CONTROL) => {
                if let Some(focus) = self.focus.left() {
                    self.focus = focus;
                }
            }
            (KeyCode::Right, KeyModifiers::CONTROL)
            | (KeyCode::Char('l'), KeyModifiers::CONTROL) => {
                let show_mapping = self.runner.input_dir.is_some();
                if let Some(focus) = self.focus.right(show_mapping) {
                    self.focus = focus;
                }
            }
            (KeyCode::Up, KeyModifiers::CONTROL) | (KeyCode::Char('k'), KeyModifiers::CONTROL) => {
                if let Some(focus) = self.focus.up() {
                    self.focus = focus;
                }
            }
            (KeyCode::Down, KeyModifiers::CONTROL)
            | (KeyCode::Char('j'), KeyModifiers::CONTROL) => {
                if let Some(focus) = self.focus.down() {
                    self.focus = focus;
                }
            }
            (KeyCode::Tab, _) => {
                let show_mapping = self.runner.input_dir.is_some();
                self.focus = self.focus.next(show_mapping);
            }
            (KeyCode::BackTab, _) => {
                let show_mapping = self.runner.input_dir.is_some();
                self.focus = self.focus.prev(show_mapping);
            }
            _ => {
                let msg = AppMsg::Key(key);
                match self.focus {
                    Focus::FileBrowser => match key.code {
                        KeyCode::Char('i') => {
                            let current_dir = self.file_browser.current_dir.clone();
                            let _ = self.tx.send(AppMsg::InputDirSelected(current_dir));
                        }
                        KeyCode::Char('o') => {
                            let current_dir = self.file_browser.current_dir.clone();
                            let _ = self.tx.send(AppMsg::OutputDirSelected(current_dir));
                        }
                        _ => {
                            self.file_browser.update(&msg);
                        }
                    },
                    Focus::Runner => match key.code {
                        KeyCode::Char('c') => {
                            let _ = self.tx.send(AppMsg::PipelineStarted);
                        }
                        KeyCode::Char('x') => {
                            let _ = self.tx.send(AppMsg::ConvertXCursorOnly);
                        }
                        KeyCode::Char('p') => {
                            let _ = self.tx.send(AppMsg::ConvertPNGOnly);
                        }
                        _ => {
                            self.runner.update(&msg);
                        }
                    },
                    Focus::Overrides => {
                        self.theme_overrides.update(&msg);
                    }
                    Focus::Editor => {
                        if let Some(response) = self.cursor_editor.update(&msg) {
                            let _ = self.tx.send(response);
                        }
                    }
                    Focus::Logs => {
                        self.logs.update(&msg);
                    }
                    Focus::Mapping => {
                        if let Some(response) = self.mapping_editor.update(&msg) {
                            let _ = self.tx.send(response);
                        }
                    }
                }
            }
        }
        false
    }
}

fn restore_terminal(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> anyhow::Result<()> {
    terminal.show_cursor().ok();
    disable_raw_mode().ok();
    let mut out = io::stdout();
    execute!(out, LeaveAlternateScreen)?;
    Ok(())
}
