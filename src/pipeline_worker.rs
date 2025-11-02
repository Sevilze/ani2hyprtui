// Pipeline worker for processing Windows cursors in a separate thread

use anyhow::Result;
use crossbeam_channel::Sender;
use std::fs;
use std::path::{Path, PathBuf};
use std::thread;
use walkdir::WalkDir;

use crate::event::AppMsg;
use crate::pipeline::win2xcur::converter::{ConversionOptions, convert_windows_cursor};
use crate::pipeline::xcur2png::{ExtractOptions, extract_to_pngs};

pub struct PipelineWorker {
    tx: Sender<AppMsg>,
}

impl PipelineWorker {
    pub fn new(tx: Sender<AppMsg>) -> Self {
        Self { tx }
    }

    pub fn start_ani_to_png_conversion(
        &self,
        input_dir: PathBuf,
        output_dir: PathBuf,
    ) {
        let tx = self.tx.clone();
        
        thread::spawn(move || {
            if let Err(e) = Self::run_ani_to_png_pipeline(&input_dir, &output_dir, &tx) {
                let _ = tx.send(AppMsg::PipelineFailed(format!("{}", e)));
            }
        });
    }

    fn run_ani_to_png_pipeline(
        input_dir: &Path,
        output_dir: &Path,
        tx: &Sender<AppMsg>,
    ) -> Result<()> {
        fs::create_dir_all(output_dir)?;
        let _ = tx.send(AppMsg::LogMessage(format!(
            "Created output directory: {}",
            output_dir.display()
        )));

        let mut cursor_files = Vec::new();
        for entry in WalkDir::new(input_dir)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            if let Some(ext) = path.extension() {
                let ext_str = ext.to_string_lossy().to_lowercase();
                if ext_str == "ani" || ext_str == "cur" {
                    cursor_files.push(path.to_path_buf());
                }
            }
        }

        let total_files = cursor_files.len();
        if total_files == 0 {
            let _ = tx.send(AppMsg::PipelineFailed(
                "No .ani or .cur files found in input directory".to_string()
            ));
            return Ok(());
        }

        let _ = tx.send(AppMsg::LogMessage(format!(
            "Found {} cursor files to process",
            total_files
        )));

        let xcur_dir = output_dir.join("_xcur_intermediate");
        fs::create_dir_all(&xcur_dir)?;

        let conversion_options = ConversionOptions::new();
        let mut processed = 0;
        let mut failed = 0;

        for (idx, cursor_file) in cursor_files.iter().enumerate() {
            let file_name = cursor_file
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("cursor");

            let _ = tx.send(AppMsg::LogMessage(format!(
                "Processing {}/{}: {}",
                idx + 1,
                total_files,
                file_name
            )));

            let xcur_output = xcur_dir.join(file_name);
            match convert_windows_cursor(cursor_file, &xcur_output, &conversion_options) {
                Ok(_) => {
                    let _ = tx.send(AppMsg::LogMessage(format!(
                        "  ✓ Converted {} to X11 format",
                        file_name
                    )));

                    let png_output_dir = output_dir.join(file_name);
                    fs::create_dir_all(&png_output_dir)?;

                    let extract_options = ExtractOptions::new()
                        .with_prefix(file_name)
                        .with_config(true);

                    match extract_to_pngs(&xcur_output, &png_output_dir, &extract_options) {
                        Ok(files) => {
                            let _ = tx.send(AppMsg::LogMessage(format!(
                                "  ✓ Extracted {} PNG files",
                                files.len()
                            )));
                            processed += 1;
                        }
                        Err(e) => {
                            let _ = tx.send(AppMsg::LogMessage(format!(
                                "  ✗ Failed to extract PNGs: {}",
                                e
                            )));
                            failed += 1;
                        }
                    }
                }
                Err(e) => {
                    let _ = tx.send(AppMsg::LogMessage(format!(
                        "  ✗ Failed to convert: {}",
                        e
                    )));
                    failed += 1;
                }
            }

            let _ = tx.send(AppMsg::PipelineProgress(processed + failed, total_files));
        }

        let _ = fs::remove_dir_all(&xcur_dir);

        if failed > 0 {
            let _ = tx.send(AppMsg::LogMessage(format!(
                "Completed with {} successes and {} failures",
                processed, failed
            )));
        }

        let _ = tx.send(AppMsg::PipelineCompleted(processed));
        Ok(())
    }

    pub fn start_ani_to_xcur_conversion(
        &self,
        input_dir: PathBuf,
        output_dir: PathBuf,
    ) {
        let tx = self.tx.clone();
        
        thread::spawn(move || {
            if let Err(e) = Self::run_ani_to_xcur_pipeline(&input_dir, &output_dir, &tx) {
                let _ = tx.send(AppMsg::PipelineFailed(format!("{}", e)));
            }
        });
    }

    fn run_ani_to_xcur_pipeline(
        input_dir: &Path,
        output_dir: &Path,
        tx: &Sender<AppMsg>,
    ) -> Result<()> {
        fs::create_dir_all(output_dir)?;

        let mut cursor_files = Vec::new();
        for entry in WalkDir::new(input_dir)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            if let Some(ext) = path.extension() {
                let ext_str = ext.to_string_lossy().to_lowercase();
                if ext_str == "ani" || ext_str == "cur" {
                    cursor_files.push(path.to_path_buf());
                }
            }
        }

        let total_files = cursor_files.len();
        if total_files == 0 {
            let _ = tx.send(AppMsg::PipelineFailed(
                "No .ani or .cur files found".to_string()
            ));
            return Ok(());
        }

        let _ = tx.send(AppMsg::LogMessage(format!(
            "Found {} cursor files",
            total_files
        )));

        let conversion_options = ConversionOptions::new();
        let mut processed = 0;

        for (idx, cursor_file) in cursor_files.iter().enumerate() {
            let file_name = cursor_file
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("cursor");

            let xcur_output = output_dir.join(file_name);
            
            if let Err(e) = convert_windows_cursor(cursor_file, &xcur_output, &conversion_options) {
                let _ = tx.send(AppMsg::LogMessage(format!(
                    "Failed to convert {}: {}",
                    file_name, e
                )));
            } else {
                processed += 1;
            }

            let _ = tx.send(AppMsg::PipelineProgress(idx + 1, total_files));
        }

        let _ = tx.send(AppMsg::PipelineCompleted(processed));
        Ok(())
    }
}
