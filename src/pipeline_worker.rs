// Pipeline worker for processing Windows cursors in a separate thread

use anyhow::Result;
use crossbeam_channel::Sender;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::thread;
use walkdir::WalkDir;

use crate::event::AppMsg;
use crate::model::mapping::CursorMapping;
use crate::pipeline::hyprcursor;
use crate::pipeline::win2xcur::converter::{ConversionOptions, convert_windows_cursor};
use crate::pipeline::xcur2png::{ExtractOptions, extract_to_pngs};
use crate::pipeline::xcursor_gen::XCursorThemeBuilder;

pub struct PipelineWorker {
    tx: Sender<AppMsg>,
}

impl PipelineWorker {
    pub fn new(tx: Sender<AppMsg>) -> Self {
        Self { tx }
    }

    pub fn start_ani_to_png_conversion(&self, input_dir: PathBuf, output_dir: PathBuf) {
        let tx = self.tx.clone();

        thread::spawn(move || {
            if let Err(e) = Self::run_ani_to_png_pipeline(&input_dir, &output_dir, &tx) {
                let _ = tx.send(AppMsg::PipelineFailed(format!("{}", e)));
            }
        });
    }

    fn find_cursor_files(input_dir: &Path) -> Vec<PathBuf> {
        WalkDir::new(input_dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .map(|e| e.path().to_path_buf())
            .filter(|path| {
                path.extension()
                    .and_then(|ext| ext.to_str())
                    .map(|s| {
                        let s = s.to_lowercase();
                        s == "ani" || s == "cur"
                    })
                    .unwrap_or(false)
            })
            .collect()
    }

    fn convert_batch(
        cursor_files: &[PathBuf],
        xcur_dir: &Path,
        png_dir: Option<&Path>,
        target_sizes: Vec<u32>,
        tx: &Sender<AppMsg>,
    ) -> Result<(usize, usize)> {
        // (processed, failed)
        let total_files = cursor_files.len();
        let conversion_options = ConversionOptions::new().with_target_sizes(target_sizes);
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
            match convert_windows_cursor(cursor_file, &xcur_output, &conversion_options, |msg| {
                let _ = tx.send(AppMsg::LogMessage(msg));
            }) {
                Ok(_) => {
                    if let Some(png_out) = png_dir {
                        let png_output_dir = png_out.join(file_name);
                        fs::create_dir_all(&png_output_dir)?;

                        let extract_options = ExtractOptions::new()
                            .with_prefix(file_name)
                            .with_config(true);

                        match extract_to_pngs(&xcur_output, &png_output_dir, &extract_options) {
                            Ok(_) => {
                                processed += 1;
                            }
                            Err(e) => {
                                let _ = tx.send(AppMsg::LogMessage(format!(
                                    "Failed to extract PNGs: {}",
                                    e
                                )));
                                failed += 1;
                            }
                        }
                    } else {
                        processed += 1;
                    }
                }
                Err(e) => {
                    let _ = tx.send(AppMsg::LogMessage(format!("  ✗ Failed to convert: {}", e)));
                    failed += 1;
                }
            }

            let _ = tx.send(AppMsg::PipelineProgress(processed + failed, total_files));
        }

        Ok((processed, failed))
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

        let cursor_files = Self::find_cursor_files(input_dir);
        let total_files = cursor_files.len();

        if total_files == 0 {
            let _ = tx.send(AppMsg::PipelineFailed(
                "No .ani or .cur files found in input directory".to_string(),
            ));
            return Ok(());
        }

        let _ = tx.send(AppMsg::LogMessage(format!(
            "Found {} cursor files to process",
            total_files
        )));

        let xcur_dir = output_dir.join("_xcur_intermediate");
        fs::create_dir_all(&xcur_dir)?;

        let (processed, failed) =
            Self::convert_batch(&cursor_files, &xcur_dir, Some(output_dir), Vec::new(), tx)?;

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

    pub fn start_ani_to_xcur_conversion(&self, input_dir: PathBuf, output_dir: PathBuf) {
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

        let cursor_files = Self::find_cursor_files(input_dir);
        let total_files = cursor_files.len();

        if total_files == 0 {
            let _ = tx.send(AppMsg::PipelineFailed(
                "No .ani or .cur files found".to_string(),
            ));
            return Ok(());
        }

        let _ = tx.send(AppMsg::LogMessage(format!(
            "Found {} cursor files",
            total_files
        )));

        let (processed, _) = Self::convert_batch(&cursor_files, output_dir, None, Vec::new(), tx)?;

        let _ = tx.send(AppMsg::PipelineCompleted(processed));
        Ok(())
    }

    pub fn start_full_theme_conversion(
        &self,
        input_dir: PathBuf,
        output_dir: PathBuf,
        theme_name: String,
        mapping: CursorMapping,
        target_sizes: Vec<u32>,
    ) {
        let tx = self.tx.clone();

        thread::spawn(move || {
            if let Err(e) = Self::run_full_theme_pipeline(
                &input_dir,
                &output_dir,
                &theme_name,
                mapping,
                target_sizes,
                &tx,
            ) {
                let _ = tx.send(AppMsg::PipelineFailed(format!("{}", e)));
            }
        });
    }

    pub fn start_incremental_theme_update(
        &self,
        input_dir: PathBuf,
        output_dir: PathBuf,
        theme_name: String,
        mapping: CursorMapping,
        modified_cursors: Vec<String>,
        hotspot_overrides: HashMap<String, HashMap<u32, (u32, u32)>>,
    ) {
        let tx = self.tx.clone();

        thread::spawn(move || {
            if let Err(e) = Self::run_incremental_theme_update(
                &input_dir,
                &output_dir,
                &theme_name,
                mapping,
                modified_cursors,
                hotspot_overrides,
                &tx,
            ) {
                let _ = tx.send(AppMsg::PipelineFailed(format!("{}", e)));
            }
        });
    }

    fn run_incremental_theme_update(
        input_dir: &Path,
        output_dir: &Path,
        theme_name: &str,
        mapping: CursorMapping,
        modified_cursors: Vec<String>,
        hotspot_overrides: HashMap<String, HashMap<u32, (u32, u32)>>,
        tx: &Sender<AppMsg>,
    ) -> Result<()> {
        let count = modified_cursors.len();
        let _ = tx.send(AppMsg::LogMessage(format!(
            "Starting incremental update for {} cursors...",
            count
        )));

        let theme_output = output_dir.join(theme_name);
        let cursors_dir = theme_output.join("cursors");
        let hyprcursors_dir = theme_output.join("hyprcursors");
        let png_dir = output_dir.join("png_intermediate");

        fs::create_dir_all(&cursors_dir)?;
        fs::create_dir_all(&hyprcursors_dir)?;
        fs::create_dir_all(&png_dir)?;

        let default_options = ConversionOptions::new();

        for x11_name in modified_cursors {
            if let Some(win_name) = mapping.get_win_name(&x11_name) {
                let _ = tx.send(AppMsg::LogMessage(format!(
                    "Updating {} -> {}",
                    x11_name, win_name
                )));

                // Find source file
                let mut source_file = None;
                // Try .ani then .cur
                let ani_path = input_dir.join(format!("{}.ani", win_name));
                let cur_path = input_dir.join(format!("{}.cur", win_name));

                if ani_path.exists() {
                    source_file = Some(ani_path);
                } else if cur_path.exists() {
                    source_file = Some(cur_path);
                } else if win_name == "Normal" {
                    // Fallback logic if needed, but usually Normal should exist
                }

                if let Some(source_path) = source_file {
                    // Convert to XCursor
                    let xcur_output = cursors_dir.join(&x11_name);

                    let mut options = default_options.clone();
                    if let Some(overrides) = hotspot_overrides.get(&x11_name) {
                        for (size, (x, y)) in overrides {
                            options = options.with_hotspot_override(*size, *x, *y);
                        }
                    }

                    if let Err(e) =
                        convert_windows_cursor(&source_path, &xcur_output, &options, |msg| {
                            let _ = tx.send(AppMsg::LogMessage(msg));
                        })
                    {
                        let _ = tx.send(AppMsg::LogMessage(format!(
                            "Failed to convert XCursor: {}",
                            e
                        )));
                        continue;
                    }

                    // Update symlinks for this cursor
                    let symlinks = mapping.get_symlinks(&x11_name);
                    for link in &symlinks {
                        let link_path = cursors_dir.join(link);
                        if link_path.exists() {
                            let _ = fs::remove_file(&link_path);
                        }
                        // Create relative symlink
                        #[cfg(unix)]
                        {
                            use std::os::unix::fs::symlink;
                            if let Err(e) = symlink(&x11_name, &link_path) {
                                let _ = tx.send(AppMsg::LogMessage(format!(
                                    "Failed to symlink {}: {}",
                                    link, e
                                )));
                            }
                        }
                    }

                    // Update Hyprcursor
                    // Extract XCursor to temp dir
                    let temp_dir = tempfile::tempdir()?;
                    let working_state_dir = temp_dir.path();

                    // Pass overrides (symlinks) to the extractor
                    if let Err(e) = hyprcursor::extract_xcursor_to_hypr_source(
                        &xcur_output,
                        working_state_dir,
                        None,
                        symlinks.clone(),
                    ) {
                        let _ = tx.send(AppMsg::LogMessage(format!(
                            "Failed to extract for Hyprcursor: {}",
                            e
                        )));
                        continue;
                    }

                    // Compile to .hlc
                    let shape_dir = working_state_dir.join(&x11_name);

                    if let Err(e) =
                        hyprcursor::process_shape(&shape_dir, &hyprcursors_dir, &x11_name, |msg| {
                            let _ = tx.send(AppMsg::LogMessage(msg));
                        })
                    {
                        let _ = tx.send(AppMsg::LogMessage(format!(
                            "Failed to compile Hyprcursor: {}",
                            e
                        )));
                    } else {
                        let _ = tx.send(AppMsg::LogMessage(format!("Updated {}", x11_name)));
                    }
                } else {
                    let _ = tx.send(AppMsg::LogMessage(format!(
                        "Source file not found for {}",
                        win_name
                    )));
                }
            }
        }

        let _ = tx.send(AppMsg::LogMessage(
            "Incremental update completed.".to_string(),
        ));
        Ok(())
    }

    fn run_full_theme_pipeline(
        input_dir: &Path,
        output_dir: &Path,
        theme_name: &str,
        mapping: CursorMapping,
        target_sizes: Vec<u32>,
        tx: &Sender<AppMsg>,
    ) -> Result<()> {
        // ANI to XCursor binaries
        let _ = tx.send(AppMsg::LogMessage(
            "Converting ANI files to X11 cursor format...".to_string(),
        ));

        let xcur_dir = output_dir.join("xcur_intermediate");
        fs::create_dir_all(&xcur_dir)?;

        let png_dir = output_dir.join("png_intermediate");
        fs::create_dir_all(&png_dir)?;

        let cursor_files = Self::find_cursor_files(input_dir);
        let total_files = cursor_files.len();

        if total_files == 0 {
            let _ = tx.send(AppMsg::PipelineFailed(
                "No .ani or .cur files found".to_string(),
            ));
            return Ok(());
        }

        let (processed, _) =
            Self::convert_batch(&cursor_files, &xcur_dir, Some(&png_dir), target_sizes, tx)?;

        if processed == 0 {
            let _ = tx.send(AppMsg::PipelineFailed(
                "Failed to convert any cursor files".to_string(),
            ));
            return Ok(());
        }

        let _ = tx.send(AppMsg::LogMessage(format!(
            "Converted {}/{} cursor files",
            processed, total_files
        )));

        // Organize into theme with mapping
        let _ = tx.send(AppMsg::LogMessage(
            "Building XCursor theme with mapping...".to_string(),
        ));

        let theme_output = output_dir.join(theme_name);
        let builder =
            XCursorThemeBuilder::new(theme_output.clone(), theme_name.to_string(), mapping);

        let theme_count = builder.build_from_xcur_files(&xcur_dir)?;

        let _ = tx.send(AppMsg::LogMessage(format!(
            "Created theme with {} cursors and symlinks",
            theme_count
        )));

        let _ = fs::remove_dir_all(&xcur_dir);

        // Generate Hyprcursor theme
        let _ = tx.send(AppMsg::LogMessage(
            "Generating Hyprcursor theme...".to_string(),
        ));

        let temp_dir = tempfile::tempdir()?;
        let working_state_dir = temp_dir.path();

        // Extract XCursor theme to working state
        let _ = tx.send(AppMsg::LogMessage(
            "Extracting XCursor theme to working state...".to_string(),
        ));

        hyprcursor::extract_xcursor_theme(
            &theme_output,
            Some(working_state_dir),
            None,
            true,
            |msg| {
                let _ = tx.send(AppMsg::LogMessage(msg));
            },
        )?;

        // Compile Hyprcursor theme back into the theme directory
        let _ = tx.send(AppMsg::LogMessage(
            "Compiling Hyprcursor theme...".to_string(),
        ));

        hyprcursor::create_cursor_theme(working_state_dir, Some(&theme_output), true, |msg| {
            let _ = tx.send(AppMsg::LogMessage(msg));
        })?;

        let _ = tx.send(AppMsg::LogMessage(format!(
            "Generated Hyprcursor files in {}",
            theme_output.display()
        )));

        let _ = tx.send(AppMsg::XCursorGenerated(theme_output.display().to_string()));
        let _ = tx.send(AppMsg::PipelineCompleted(processed));
        Ok(())
    }
}
