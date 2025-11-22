use anyhow::Result;
use std::path::{Path, PathBuf};

use super::png_writer::{PngWriteConfig, write_config_file, write_png};
use super::xcursor_reader::XcursorFile;

#[derive(Debug, Clone)]
pub struct ExtractOptions {
    pub prefix: String,
    pub initial_suffix: usize,
    pub write_config: bool,
    pub config_name: Option<String>,
    pub extract_all_sizes: bool,
}

impl ExtractOptions {
    pub fn new() -> Self {
        Self {
            prefix: "cursor".to_string(),
            initial_suffix: 0,
            write_config: true,
            config_name: None,
            extract_all_sizes: true,
        }
    }

    pub fn with_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.prefix = prefix.into();
        self
    }

    pub fn with_initial_suffix(mut self, suffix: usize) -> Self {
        self.initial_suffix = suffix;
        self
    }

    pub fn with_config(mut self, write: bool) -> Self {
        self.write_config = write;
        self
    }

    pub fn with_config_name(mut self, name: impl Into<String>) -> Self {
        self.config_name = Some(name.into());
        self
    }

    pub fn with_all_sizes(mut self, extract_all: bool) -> Self {
        self.extract_all_sizes = extract_all;
        self
    }
}

impl Default for ExtractOptions {
    fn default() -> Self {
        Self::new()
    }
}

pub fn extract_to_pngs(
    xcursor_path: &Path,
    output_dir: &Path,
    options: &ExtractOptions,
) -> Result<Vec<PathBuf>> {
    let xcursor = XcursorFile::from_file(xcursor_path)?;

    std::fs::create_dir_all(output_dir)?;

    let mut extracted_files = Vec::new();
    let mut config_entries = Vec::new();
    let mut suffix = options.initial_suffix;

    let sizes = if options.extract_all_sizes {
        xcursor.get_sizes()
    } else {
        xcursor.get_sizes().into_iter().max().into_iter().collect()
    };

    for size in sizes {
        let images = xcursor.get_images_for_size(size);
        for image in images.iter() {
            let filename = format!("{}_{:03}.png", options.prefix, suffix);
            let filepath = output_dir.join(&filename);

            write_png(&image.pixels, &filepath)?;
            extracted_files.push(filepath);

            if options.write_config {
                let relative_path = filename.clone();

                config_entries.push(PngWriteConfig {
                    filename: relative_path,
                    size: image.size,
                    xhot: image.xhot,
                    yhot: image.yhot,
                    delay: image.delay,
                });
            }

            suffix += 1;
            if suffix > 999 {
                return Err(anyhow::anyhow!("Suffix exceeded 999"));
            }
        }
    }

    if options.write_config && !config_entries.is_empty() {
        let config_name = options
            .config_name
            .clone()
            .unwrap_or_else(|| format!("{}.conf", options.prefix));

        let config_path = output_dir.join(config_name);
        write_config_file(&config_path, &config_entries)?;
    }

    Ok(extracted_files)
}

pub fn extract_metadata(xcursor_path: &Path) -> Result<CursorMetadata> {
    let xcursor = XcursorFile::from_file(xcursor_path)?;

    let sizes = xcursor.get_sizes();
    let total_images = xcursor.images.len();

    let mut frames_per_size = Vec::new();
    for size in &sizes {
        let count = xcursor.get_images_for_size(*size).len();
        frames_per_size.push((*size, count));
    }

    Ok(CursorMetadata {
        sizes: sizes.clone(),
        total_images,
        frames_per_size,
    })
}

#[derive(Debug, Clone)]
pub struct CursorMetadata {
    pub sizes: Vec<u32>,
    pub total_images: usize,
    pub frames_per_size: Vec<(u32, usize)>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_options() {
        let opts = ExtractOptions::new()
            .with_prefix("test")
            .with_initial_suffix(10)
            .with_config(false);

        assert_eq!(opts.prefix, "test");
        assert_eq!(opts.initial_suffix, 10);
        assert!(!opts.write_config);
    }

    #[test]
    fn test_extract_options_default() {
        let opts = ExtractOptions::default();
        assert_eq!(opts.prefix, "cursor");
        assert_eq!(opts.initial_suffix, 0);
        assert!(opts.write_config);
    }
}
