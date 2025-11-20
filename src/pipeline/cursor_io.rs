// Cursor file loading and parsing

use anyhow::{Context, Result};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;
use xcursor::parser::{parse_xcursor, Image};

use super::cursor_types::{CursorMeta, Frame, SizeVariant};
use super::win2xcur::{CursorFormat, CurParser, AniParser};

fn scan_cursor_dir(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut cursor_files = Vec::new();
    let cursors_dir = dir.join("cursors");

    if !cursors_dir.exists() {
        // Try the directory itself if no cursors subdirectory
        for entry in WalkDir::new(dir).max_depth(1) {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() && (is_likely_cursor_file(path) || is_windows_cursor_file(path)) {
                cursor_files.push(path.to_path_buf());
            }
        }
    } else {
        // Scan cursors subdirectory
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
        if matches!(ext_str.as_str(), "txt" | "md" | "conf" | "theme" | "png" | "svg") {
            return false;
        }
    }

    // Try to read first 4 bytes to check for Xcur magic
    if let Ok(bytes) = fs::read(path) {
        if bytes.len() >= 4 && &bytes[0..4] == b"Xcur" {
            return true;
        }
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

fn parse_windows_cursor_file(path: &Path) -> Result<Vec<crate::pipeline::win2xcur::cur::CursorFrame>> {
    let data = fs::read(path).context("Failed to read Windows cursor file")?;
    
    let format = CursorFormat::detect(&data)
        .ok_or_else(|| anyhow::anyhow!("Unsupported cursor format"))?;
    
    match format {
        CursorFormat::Cur => CurParser::parse(&data),
        CursorFormat::Ani => AniParser::parse(&data),
    }
}

fn convert_windows_cursor_to_meta(
    path: &Path,
    frames: Vec<crate::pipeline::win2xcur::cur::CursorFrame>,
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

    // convert to SizeVariants
    let mut variants: Vec<SizeVariant> = size_map
        .into_iter()
        .map(|(size, indices)| {
            // get hotspot from first image of this size
            let (first_frame_idx, first_img_idx) = indices[0];
            let first_img = &frames[first_frame_idx].images[first_img_idx];
            let hotspot = first_img.hotspot;

            // create frames for this size variant
            let frame_list: Vec<Frame> = frames
                .iter()
                .map(|frame| {
                    let delay = frame.delay;
                    Frame {
                        png_path: PathBuf::new(), // will be populated when extracted
                        delay_ms: delay,
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
pub fn load_cursor_folder(dir: &Path) -> Result<Vec<CursorMeta>> {
    let cursor_files = scan_cursor_dir(dir)?;
    let mut cursors = Vec::new();

    for path in cursor_files {
        if is_windows_cursor_file(&path) {
            match parse_windows_cursor_file(&path) {
                Ok(frames) => {
                    let meta = convert_windows_cursor_to_meta(&path, frames);
                    cursors.push(meta);
                }
                Err(e) => {
                    eprintln!("Warning: Failed to parse Windows cursor {}: {}", path.display(), e);
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
                    eprintln!("Warning: Failed to parse X11 cursor {}: {}", path.display(), e);
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
        let mut variants_map: HashMap<u32, Vec<(PathBuf, u32, (u16, u16))>> = HashMap::new();
        
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
            
            let (png_filename, delay_str) = if all_parts.len() > 4 
                && all_parts.last().unwrap().parse::<u32>().is_ok() 
                && all_parts.len() >= 5 {
                (all_parts[3..all_parts.len()-1].join(" "), Some(all_parts.last().unwrap()))
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
                
                let delay_ms = delay_str
                    .and_then(|s| s.parse::<u32>().ok())
                    .unwrap_or(50);
                
                variants_map
                    .entry(size)
                    .or_insert_with(Vec::new)
                    .push((png_path, delay_ms, (hotspot_x, hotspot_y)));
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

