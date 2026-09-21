use std::collections::HashMap;
use std::path::{Path, PathBuf};

use super::App;
use super::focus::ActivePipeline;
use crate::event::AppMsg;
use crate::model::cursor;
use crate::pipeline::cursor_io::{
    ensure_variant_for_size, load_cursor_folder, load_cursor_folder_from_pngs,
};

impl App {
    pub fn handle_dir_selection(&mut self, msg: &AppMsg) {
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
                self.mapping_editor.set_available_sources(sources, &self.tx);
                self.sync_hyprctl_dirs();
            }
            AppMsg::OutputDirSelected(path) => {
                self.runner.set_output_dir(path.clone());
                self.sync_hyprctl_dirs();
            }
            _ => {}
        }
    }

    pub fn handle_pipeline_msg(&mut self, msg: &AppMsg) {
        match msg {
            AppMsg::PipelineStarted => {
                if let (Some(input_dir), Some(output_dir)) = (
                    self.runner.input_dir.clone(),
                    self.runner.output_dir.clone(),
                ) {
                    self.active_pipeline = Some(ActivePipeline::Full);
                    let theme_name = self.get_theme_name(&input_dir);
                    let mapping = self.mapping_editor.mapping.clone();
                    let selected_sizes: Vec<u32> = self
                        .theme_overrides
                        .selected_sizes
                        .iter()
                        .copied()
                        .collect();

                    self.pipeline_worker.start_full_theme_conversion(
                        input_dir.clone(),
                        output_dir.clone(),
                        theme_name,
                        mapping,
                        selected_sizes,
                    );
                } else {
                    let _ = self.tx.send(AppMsg::ErrorOccurred(
                        "Cannot start pipeline: input or output directory not set".to_string(),
                    ));
                }
            }
            AppMsg::ConvertXCursorOnly => {
                if let (Some(input_dir), Some(output_dir)) = (
                    self.runner.input_dir.clone(),
                    self.runner.output_dir.clone(),
                ) {
                    self.active_pipeline = Some(ActivePipeline::XCursor);
                    let folder_name = self.get_xcursor_output_name(&input_dir);
                    self.pipeline_worker.start_ani_to_xcur_conversion(
                        input_dir,
                        output_dir,
                        folder_name,
                    );
                } else {
                    let _ = self.tx.send(AppMsg::ErrorOccurred(
                        "Cannot start pipeline: input or output directory not set".to_string(),
                    ));
                }
            }
            AppMsg::ConvertPNGOnly => {
                if let (Some(input_dir), Some(output_dir)) = (
                    self.runner.input_dir.clone(),
                    self.runner.output_dir.clone(),
                ) {
                    self.active_pipeline = Some(ActivePipeline::Png);
                    let folder_name = self.get_png_output_name(&input_dir);
                    self.pipeline_worker.start_ani_to_png_conversion(
                        input_dir,
                        output_dir,
                        folder_name,
                    );
                } else {
                    let _ = self.tx.send(AppMsg::ErrorOccurred(
                        "Cannot start pipeline: input or output directory not set".to_string(),
                    ));
                }
            }
            AppMsg::PipelineCompleted(_count) => {
                if self.active_pipeline == Some(ActivePipeline::Full)
                    && let Some(output_dir) = &self.runner.output_dir
                {
                    let png_dir = output_dir.join("png_intermediate");
                    if png_dir.exists() {
                        self.load_cursors_from_path(&png_dir);
                    }
                }
                self.active_pipeline = None;
            }
            AppMsg::PipelineFailed(_) => {
                self.active_pipeline = None;
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

    pub fn handle_save_msg(&mut self, msg: &AppMsg) {
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
                    let theme_name = self.get_theme_name(&input_dir);
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

    pub fn handle_cursor_msg(&mut self, msg: &AppMsg) {
        match msg {
            AppMsg::CursorSelected(path) => {
                let _ = self.tx.send(AppMsg::LogMessage(format!(
                    "Loading cursors from: {}",
                    path.display()
                )));

                // Populate mapping editor with available cursor files
                let mut sources = Vec::new();
                let mut scan_dirs = vec![path.clone()];
                let nested_cursors = path.join("cursors");
                if nested_cursors.is_dir() {
                    scan_dirs.push(nested_cursors);
                }

                for scan_dir in scan_dirs {
                    if let Ok(entries) = std::fs::read_dir(scan_dir) {
                        for entry in entries.flatten() {
                            let entry_path = entry.path();
                            if let Some(ext) = entry_path.extension() {
                                let ext_str = ext.to_string_lossy().to_lowercase();
                                if (ext_str == "ani" || ext_str == "cur")
                                    && let Some(stem) = entry_path.file_stem()
                                {
                                    sources.push(stem.to_string_lossy().to_string());
                                }
                            }
                        }
                    }
                }
                sources.sort();
                sources.dedup();
                self.mapping_editor.set_available_sources(sources, &self.tx);

                self.load_cursors_from_path(path);
            }
            AppMsg::CursorLoaded(_) => {
                // Handled by Editor component
            }
            _ => {}
        }
    }

    pub fn load_cursors_from_path(&self, path: &Path) {
        let _ = self.tx.send(AppMsg::LogMessage(format!(
            "Loading cursors from: {}",
            path.display()
        )));

        let output_dir = self
            .runner
            .output_dir
            .clone()
            .unwrap_or_else(|| PathBuf::from("./out"));
        let png_intermediate = output_dir.join("png_intermediate");

        let cursors = load_cursor_folder_from_pngs(path)
            .ok()
            .filter(|v| !v.is_empty())
            .map(Ok)
            .unwrap_or_else(|| load_cursor_folder(path, &png_intermediate));

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

                let _ = std::fs::create_dir_all(&png_intermediate);
                let selected_sizes: Vec<u32> = self
                    .theme_overrides
                    .selected_sizes
                    .iter()
                    .copied()
                    .collect();

                for cursor in &mut converted_cursors {
                    for &target_size in &selected_sizes {
                        ensure_variant_for_size(cursor, target_size, Some(&png_intermediate));
                    }
                }

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

    pub fn sync_hyprctl_dirs(&mut self) {
        let mut dirs = Vec::new();
        if let Some(ref out) = self.runner.output_dir {
            dirs.push(out.clone());
        }
        if let Some(ref inp) = self.runner.input_dir
            && !dirs.contains(inp)
        {
            dirs.push(inp.clone());
        }
        if !dirs.contains(&self.file_browser.current_dir) {
            dirs.push(self.file_browser.current_dir.clone());
        }
        self.hyprctl.set_extra_dirs(dirs);
    }

    pub fn get_theme_name(&self, input_dir: &Path) -> String {
        if !self.theme_overrides.output_name.trim().is_empty() {
            self.theme_overrides.output_name.trim().to_string()
        } else {
            input_dir
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("ConvertedCursors")
                .to_string()
        }
    }

    pub fn get_xcursor_output_name(&self, input_dir: &Path) -> String {
        if !self.theme_overrides.output_name.trim().is_empty() {
            self.theme_overrides.output_name.trim().to_string()
        } else {
            let base = input_dir
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("ConvertedCursors");
            format!("{} - X11", base)
        }
    }

    pub fn get_png_output_name(&self, input_dir: &Path) -> String {
        if !self.theme_overrides.output_name.trim().is_empty() {
            self.theme_overrides.output_name.trim().to_string()
        } else {
            let base = input_dir
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("ConvertedCursors");
            format!("{} - PNG", base)
        }
    }
}
