// High-level conversion API for Windows to X11 cursor conversion

use anyhow::Result;
use std::collections::HashMap;
use std::path::Path;

use super::{
    cur::CursorFrame,
    utils::{ShadowConfig, apply_shadows, scale_frames},
    xcursor_writer,
};

#[derive(Debug, Clone, Default)]
pub struct ConversionOptions {
    pub scale: Option<f32>,
    pub shadow: Option<ShadowConfig>,
    pub hotspot_overrides: HashMap<u32, (u32, u32)>,
    pub target_sizes: Vec<u32>,
}

impl ConversionOptions {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_scale(mut self, scale: f32) -> Self {
        self.scale = Some(scale);
        self
    }

    pub fn with_shadow(mut self) -> Self {
        self.shadow = Some(ShadowConfig::default());
        self
    }

    pub fn with_shadow_config(mut self, config: ShadowConfig) -> Self {
        self.shadow = Some(config);
        self
    }

    pub fn with_hotspot_override(mut self, size: u32, x: u32, y: u32) -> Self {
        self.hotspot_overrides.insert(size, (x, y));
        self
    }

    pub fn with_target_sizes(mut self, sizes: Vec<u32>) -> Self {
        self.target_sizes = sizes;
        self
    }
}

pub fn convert_to_x11(
    mut frames: Vec<CursorFrame>,
    options: &ConversionOptions,
) -> Result<Vec<u8>> {
    // Apply hotspot overrides
    if !options.hotspot_overrides.is_empty() {
        for frame in &mut frames {
            for image in &mut frame.images {
                if let Some(&hotspot) = options.hotspot_overrides.get(&image.nominal_size) {
                    image.hotspot = (hotspot.0 as u16, hotspot.1 as u16);
                }
            }
        }
    }

    if let Some(scale) = options.scale {
        scale_frames(&mut frames, scale);
    }

    // Handle target sizes resizing
    if !options.target_sizes.is_empty() {
        for frame in &mut frames {
            let mut new_images = Vec::new();

            // We assume the first image in the frame is the "source" to resize from
            // usually ANI/CUR frames have one image per frame index, but can have multiple sizes.
            // We'll take the largest one as source if multiple exist.
            if let Some(source_image) = frame.images.iter().max_by_key(|i| i.nominal_size) {
                for &size in &options.target_sizes {
                    // Check if we already have this size
                    if frame.images.iter().any(|i| i.nominal_size == size) {
                        continue;
                    }

                    let _width = source_image.image.width();
                    let _height = source_image.image.height();

                    // Calculate scale factor
                    let scale = size as f32 / source_image.nominal_size as f32;

                    let new_width = size;
                    let new_height = size; // Force square for cursor sizes usually

                    let scaled_img = image::imageops::resize(
                        &source_image.image,
                        new_width,
                        new_height,
                        image::imageops::FilterType::Lanczos3,
                    );

                    let (new_hotspot_x, new_hotspot_y) =
                        if let Some(&override_hotspot) = options.hotspot_overrides.get(&size) {
                            (
                                override_hotspot.0.min(u16::MAX as u32) as u16,
                                override_hotspot.1.min(u16::MAX as u32) as u16,
                            )
                        } else {
                            (
                                (source_image.hotspot.0 as f32 * scale).round() as u16,
                                (source_image.hotspot.1 as f32 * scale).round() as u16,
                            )
                        };

                    use super::cur::CursorImage;
                    new_images.push(CursorImage {
                        image: scaled_img,
                        hotspot: (new_hotspot_x, new_hotspot_y),
                        nominal_size: size,
                    });
                }
            }

            frame.images.extend(new_images);
        }
    }

    if let Some(ref shadow_config) = options.shadow {
        apply_shadows(&mut frames, shadow_config)?;
    }

    xcursor_writer::to_x11(&frames)
}

pub fn convert_windows_cursor<F>(
    input_path: &Path,
    output_path: &Path,
    options: &ConversionOptions,
    mut log_fn: F,
) -> Result<()>
where
    F: FnMut(String),
{
    use super::{AniParser, CurParser, CursorFormat};

    let data = std::fs::read(input_path)?;

    let format = CursorFormat::detect(&data)
        .ok_or_else(|| anyhow::anyhow!("Unsupported cursor format: {}", input_path.display()))?;

    let frames = match format {
        CursorFormat::Cur => CurParser::parse(&data, &mut log_fn)?,
        CursorFormat::Ani => AniParser::parse(&data, &mut log_fn)?,
    };

    let x11_data = convert_to_x11(frames, options)?;

    std::fs::write(output_path, x11_data)?;

    Ok(())
}

pub fn batch_convert<F>(
    files: &[(&Path, &Path)],
    options: &ConversionOptions,
    mut log_fn: F,
) -> Result<Vec<Result<()>>>
where
    F: FnMut(String),
{
    let results: Vec<Result<()>> = files
        .iter()
        .map(|(input, output)| convert_windows_cursor(input, output, options, &mut log_fn))
        .collect();

    Ok(results)
}

pub fn batch_convert_parallel(
    files: Vec<(std::path::PathBuf, std::path::PathBuf)>,
    options: ConversionOptions,
) -> Vec<Result<()>> {
    use std::sync::Arc;
    use std::thread;

    let options = Arc::new(options);
    let chunk_size = (files.len() / num_cpus() + 1).max(1);

    let mut handles = Vec::new();

    for chunk in files.chunks(chunk_size) {
        let chunk = chunk.to_vec();
        let options = Arc::clone(&options);

        let handle = thread::spawn(move || {
            chunk
                .iter()
                .map(|(input, output)| {
                    convert_windows_cursor(input, output, &options, |msg| {
                        // For parallel execution, we fall back to eprintln since cross-thread logging is complex
                        eprintln!("{}", msg);
                    })
                })
                .collect::<Vec<_>>()
        });

        handles.push(handle);
    }

    let mut results = Vec::new();
    for handle in handles {
        if let Ok(chunk_results) = handle.join() {
            results.extend(chunk_results);
        }
    }

    results
}

fn num_cpus() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_conversion_options() {
        let opts = ConversionOptions::new().with_scale(2.0).with_shadow();

        assert_eq!(opts.scale, Some(2.0));
        assert!(opts.shadow.is_some());
    }

    #[test]
    fn test_num_cpus() {
        let cpus = num_cpus();
        assert!(cpus > 0);
        assert!(cpus <= 128); // Reasonable upper bound
    }
}
