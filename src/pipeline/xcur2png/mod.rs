// X11 Xcursor to PNG extraction pipeline

pub mod xcursor_reader;
pub mod png_writer;
pub mod extractor;

use anyhow::Result;
use std::path::{Path, PathBuf};

pub use xcursor_reader::{XcursorFile, XcursorImage};
pub use extractor::{ExtractOptions, extract_to_pngs};

pub fn extract_cursor(
    xcursor_path: &Path,
    output_dir: &Path,
    prefix: Option<&str>,
    initial_suffix: usize,
) -> Result<Vec<PathBuf>> {
    let options = ExtractOptions::new()
        .with_prefix(prefix.unwrap_or_else(|| {
            xcursor_path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("cursor")
        }))
        .with_initial_suffix(initial_suffix);
    
    extract_to_pngs(xcursor_path, output_dir, &options)
}

