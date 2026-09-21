// Cursor file loading and parsing

use anyhow::{Context, Result};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;
use xcursor::parser::{Image, parse_xcursor};

use super::cursor_types::{CursorMeta, Frame, SizeVariant};
use super::win2xcur::{AniParser, CurParser, CursorFormat, cur::CursorFrame};

type PngFrameData = (PathBuf, u32, (u16, u16));

fn scan_cursor_dir(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut cursor_files = Vec::new();
    let cursors_dir = dir.join("cursors");

    // Always check the main directory
    for entry in WalkDir::new(dir).max_depth(1) {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() && (is_likely_cursor_file(path) || is_windows_cursor_file(path)) {
            cursor_files.push(path.to_path_buf());
        }
    }

    // Also check cursors subdirectory if it exists
    if cursors_dir.exists() {
        for entry in WalkDir::new(&cursors_dir).max_depth(1) {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() && (is_likely_cursor_file(path) || is_windows_cursor_file(path)) {
                cursor_files.push(path.to_path_buf());
            }
        }
    }

    Ok(cursor_files)
}

fn is_likely_cursor_file(path: &Path) -> bool {
    // skip files with common non-cursor extensions
    if let Some(ext) = path.extension() {
        let ext_str = ext.to_string_lossy().to_lowercase();
        if matches!(
            ext_str.as_str(),
            "txt" | "md" | "conf" | "theme" | "png" | "svg"
        ) {
            return false;
        }
    }

    // Try to read first 4 bytes to check for Xcur magic
    if let Ok(bytes) = fs::read(path)
        && bytes.len() >= 4
        && &bytes[0..4] == b"Xcur"
    {
        return true;
    }

    false
}

fn is_windows_cursor_file(path: &Path) -> bool {
    if let Some(ext) = path.extension() {
        let ext_str = ext.to_string_lossy().to_lowercase();
        if matches!(ext_str.as_str(), "cur" | "ani") {
            return true;
        }
    }

    // check by content
    if let Ok(bytes) = fs::read(path) {
        return CursorFormat::detect(&bytes).is_some();
    }

    false
}

fn parse_cursor_file(path: &Path) -> Result<Vec<Image>> {
    let data = fs::read(path).context("Failed to read cursor file")?;
    parse_xcursor(&data).context("Failed to parse X11 cursor file")
}

fn parse_windows_cursor_file(path: &Path) -> Result<Vec<CursorFrame>> {
    let data = fs::read(path).context("Failed to read Windows cursor file")?;

    let format =
        CursorFormat::detect(&data).ok_or_else(|| anyhow::anyhow!("Unsupported cursor format"))?;

    match format {
        CursorFormat::Cur => CurParser::parse(&data, |msg| {
            eprintln!("{}", msg);
        }),
        CursorFormat::Ani => AniParser::parse(&data, |msg| {
            eprintln!("{}", msg);
        }),
    }
}

fn convert_windows_cursor_to_meta(
    path: &Path,
    frames: Vec<CursorFrame>,
    cache_dir: &Path,
) -> CursorMeta {
    let x11_name = path
        .file_stem()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();

    // group images by nominal size across all frames
    let mut size_map: HashMap<u32, Vec<(usize, usize)>> = HashMap::new(); // size -> [(frame_idx, img_idx)]

    for (frame_idx, frame) in frames.iter().enumerate() {
        for (img_idx, img) in frame.images.iter().enumerate() {
            size_map
                .entry(img.nominal_size)
                .or_default()
                .push((frame_idx, img_idx));
        }
    }

    // save frame images to cache dir and build SizeVariants
    let stem = x11_name.clone();
    let cursor_dir = cache_dir.join(&stem);
    let _ = fs::create_dir_all(&cursor_dir);

    let mut conf_lines = Vec::new();
    let mut variants: Vec<SizeVariant> = size_map
        .into_iter()
        .map(|(size, indices)| {
            // get hotspot from first image of this size
            let (first_frame_idx, first_img_idx) = indices[0];
            let first_img = &frames[first_frame_idx].images[first_img_idx];
            let hotspot = first_img.hotspot;

            // create frames for this size variant, saving each image as PNG
            let frame_list: Vec<Frame> = frames
                .iter()
                .enumerate()
                .map(|(frame_idx, frame)| {
                    let delay = frame.delay;
                    // pick the image matching this size from the frame, or fall back to first
                    let img = frame
                        .images
                        .iter()
                        .find(|i| i.nominal_size == size)
                        .or(frame.images.first());

                    match img {
                        Some(img_data) => {
                            let filename = format!("{}_{}_{}.png", stem, size, frame_idx);
                            let png_path = cursor_dir.join(&filename);
                            // save the image to disk
                            let _ = img_data.image.save(&png_path);
                            conf_lines.push(format!(
                                "{} {} {} {} {}\n",
                                size, hotspot.0, hotspot.1, filename, delay
                            ));
                            Frame {
                                png_path,
                                delay_ms: delay,
                            }
                        }
                        None => Frame {
                            png_path: PathBuf::new(),
                            delay_ms: delay,
                        },
                    }
                })
                .collect();

            SizeVariant {
                size,
                frames: frame_list,
                hotspot: (hotspot.0 as u32, hotspot.1 as u32),
            }
        })
        .collect();

    let conf_path = cursor_dir.join(format!("{}.conf", stem));
    if let Ok(mut file) = fs::File::create(&conf_path) {
        use std::io::Write;
        for line in &conf_lines {
            let _ = file.write_all(line.as_bytes());
        }
    }

    // sort variants by size
    variants.sort_by_key(|v| v.size);

    CursorMeta {
        x11_name,
        win_names: Vec::new(),
        variants,
        src_cursor_path: Some(path.to_path_buf()),
    }
}

/// convert xcursor Images to our CursorMeta structure
fn convert_to_cursor_meta(path: &Path, images: Vec<Image>) -> CursorMeta {
    let x11_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();

    // group images by size
    let mut size_map: HashMap<u32, Vec<Image>> = HashMap::new();
    for img in images {
        size_map.entry(img.size).or_default().push(img);
    }

    // convert to SizeVariants
    let mut variants: Vec<SizeVariant> = size_map
        .into_iter()
        .map(|(size, imgs)| {
            let hotspot = if let Some(first) = imgs.first() {
                (first.xhot, first.yhot)
            } else {
                (0, 0)
            };

            let frames: Vec<Frame> = imgs
                .iter()
                .map(|img| Frame {
                    png_path: PathBuf::new(), // will be populated when we extract frames
                    delay_ms: img.delay,
                })
                .collect();

            SizeVariant {
                size,
                frames,
                hotspot,
            }
        })
        .collect();

    // Sort variants by size
    variants.sort_by_key(|v| v.size);

    CursorMeta {
        x11_name,
        win_names: Vec::new(), // will be populated from mapping config
        variants,
        src_cursor_path: Some(path.to_path_buf()),
    }
}

/// load all cursor files from a directory
pub fn load_cursor_folder(dir: &Path, cache_dir: &Path) -> Result<Vec<CursorMeta>> {
    let cursor_files = scan_cursor_dir(dir)?;
    let mut cursors = Vec::new();

    let _ = fs::create_dir_all(cache_dir);

    for path in cursor_files {
        if is_windows_cursor_file(&path) {
            match parse_windows_cursor_file(&path) {
                Ok(frames) => {
                    let meta = convert_windows_cursor_to_meta(&path, frames, cache_dir);
                    cursors.push(meta);
                }
                Err(e) => {
                    eprintln!(
                        "Warning: Failed to parse Windows cursor {}: {}",
                        path.display(),
                        e
                    );
                }
            }
        } else if is_likely_cursor_file(&path) {
            match parse_cursor_file(&path) {
                Ok(images) => {
                    if !images.is_empty() {
                        let meta = convert_to_cursor_meta(&path, images);
                        cursors.push(meta);
                    }
                }
                Err(e) => {
                    eprintln!(
                        "Warning: Failed to parse X11 cursor {}: {}",
                        path.display(),
                        e
                    );
                }
            }
        }
    }

    Ok(cursors)
}

/// load cursors from a PNG extraction directory (for preview)
pub fn load_cursor_folder_from_pngs(dir: &Path) -> Result<Vec<CursorMeta>> {
    let mut cursors = Vec::new();

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let cursor_dir = entry.path();

        if !cursor_dir.is_dir() {
            continue;
        }

        let cursor_name = cursor_dir
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        let conf_file = cursor_dir.join(format!("{}.conf", cursor_name));
        if !conf_file.exists() {
            continue;
        }

        // parse .conf file
        let conf_content = fs::read_to_string(&conf_file)?;
        let mut variants_map: HashMap<u32, Vec<PngFrameData>> = HashMap::new();

        for line in conf_content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            let all_parts: Vec<&str> = line.split_whitespace().collect();

            if all_parts.len() < 4 {
                continue;
            }

            let size_str = all_parts[0];
            let hotspot_x_str = all_parts[1];
            let hotspot_y_str = all_parts[2];

            let (png_filename, delay_str) =
                if all_parts.len() >= 5 && all_parts.last().unwrap().parse::<u32>().is_ok() {
                    (
                        all_parts[3..all_parts.len() - 1].join(" "),
                        Some(all_parts.last().unwrap()),
                    )
                } else {
                    (all_parts[3..].join(" "), None)
                };

            if let (Ok(size), Ok(hotspot_x), Ok(hotspot_y)) = (
                size_str.parse::<u32>(),
                hotspot_x_str.parse::<u16>(),
                hotspot_y_str.parse::<u16>(),
            ) {
                // resolve PNG path relative to cursor directory
                let png_path = if Path::new(&png_filename).is_absolute() {
                    PathBuf::from(png_filename)
                } else {
                    cursor_dir.join(&png_filename)
                };

                let delay_ms = delay_str.and_then(|s| s.parse::<u32>().ok()).unwrap_or(50);

                variants_map.entry(size).or_default().push((
                    png_path,
                    delay_ms,
                    (hotspot_x, hotspot_y),
                ));
            }
        }

        let mut variants = Vec::new();
        for (size, frames_data) in variants_map {
            let hotspot = frames_data.first().map(|(_, _, h)| *h).unwrap_or((0, 0));
            let frames = frames_data
                .into_iter()
                .map(|(path, delay, _)| Frame {
                    png_path: path,
                    delay_ms: delay,
                })
                .collect();

            variants.push(SizeVariant {
                size,
                frames,
                hotspot: (hotspot.0 as u32, hotspot.1 as u32),
            });
        }

        if !variants.is_empty() {
            cursors.push(CursorMeta {
                x11_name: cursor_name.clone(),
                win_names: vec![cursor_name],
                variants,
                src_cursor_path: Some(cursor_dir),
            });
        }
    }

    Ok(cursors)
}

/// Ensure that a cursor has a variant for `target_size`, generating and saving
/// resized PNG frames and updating the cursor's .conf file if needed.
pub fn ensure_variant_for_size(
    cursor: &mut crate::model::cursor::CursorMeta,
    target_size: u32,
    png_intermediate: Option<&Path>,
) {
    if cursor.variants.iter().any(|v| v.size == target_size) {
        return;
    }

    if let Some(source) = cursor.variants.iter().max_by_key(|v| v.size).cloned() {
        let src_size = source.size;
        if src_size == 0 {
            return;
        }

        let scale = target_size as f32 / src_size as f32;
        let new_hotspot = (
            (source.hotspot.0 as f32 * scale).round() as u32,
            (source.hotspot.1 as f32 * scale).round() as u32,
        );

        // Reuse existing png_intermediate/<cursor> directory if it exists
        let dest_dir = source
            .frames
            .first()
            .and_then(|f| f.png_path.parent())
            .filter(|p| p.exists() && p.is_dir())
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| {
                let base = png_intermediate.unwrap_or_else(|| Path::new("./out/png_intermediate"));
                let p = base.join(&cursor.x11_name);
                let _ = fs::create_dir_all(&p);
                p
            });

        let mut new_frames = Vec::new();
        let mut conf_lines = Vec::new();

        for (frame_idx, frame) in source.frames.iter().enumerate() {
            let filename = format!("{}_{}_{}.png", cursor.x11_name, target_size, frame_idx);
            let png_path = dest_dir.join(&filename);

            if let Ok(img) = image::open(&frame.png_path) {
                let scaled = image::imageops::resize(
                    &img,
                    target_size,
                    target_size,
                    image::imageops::FilterType::Lanczos3,
                );
                let _ = scaled.save(&png_path);
            }

            conf_lines.push(format!(
                "{} {} {} {} {}\n",
                target_size, new_hotspot.0, new_hotspot.1, filename, frame.delay_ms
            ));

            new_frames.push(crate::model::cursor::Frame {
                png_path,
                delay_ms: frame.delay_ms,
            });
        }

        // If .conf file exists in dest_dir (as in png_intermediate), append new size entries
        let conf_path = dest_dir.join(format!("{}.conf", cursor.x11_name));
        if conf_path.exists()
            && let Ok(mut file) = fs::OpenOptions::new().append(true).open(&conf_path)
        {
            use std::io::Write;
            for line in conf_lines {
                let _ = file.write_all(line.as_bytes());
            }
        }

        cursor.variants.push(crate::model::cursor::SizeVariant {
            size: target_size,
            frames: new_frames,
            hotspot: new_hotspot,
        });
        cursor.variants.sort_by_key(|v| v.size);
    }
}
