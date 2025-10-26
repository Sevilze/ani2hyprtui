// High-level conversion API for Windows to X11 cursor conversion

use anyhow::Result;
use std::path::Path;

use super::{
    cur::CursorFrame,
    utils::{scale_frames, apply_shadows, ShadowConfig},
    xcursor_writer,
};

#[derive(Debug, Clone)]
pub struct ConversionOptions {
    pub scale: Option<f32>,  
    pub shadow: Option<ShadowConfig>,
}

impl Default for ConversionOptions {
    fn default() -> Self {
        Self {
            scale: None,
            shadow: None,
        }
    }
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
}

pub fn convert_to_x11(mut frames: Vec<CursorFrame>, options: &ConversionOptions) -> Result<Vec<u8>> {
    if let Some(scale) = options.scale {
        scale_frames(&mut frames, scale);
    }
    
    if let Some(ref shadow_config) = options.shadow {
        apply_shadows(&mut frames, shadow_config)?;
    }

    xcursor_writer::to_x11(&frames)
}

pub fn convert_windows_cursor(
    input_path: &Path,
    output_path: &Path,
    options: &ConversionOptions,
) -> Result<()> {
    use super::{CursorFormat, AniParser, CurParser};
    
    let data = std::fs::read(input_path)?;
    
    let format = CursorFormat::detect(&data)
        .ok_or_else(|| anyhow::anyhow!("Unsupported cursor format: {}", input_path.display()))?;
    
    let frames = match format {
        CursorFormat::Cur => CurParser::parse(&data)?,
        CursorFormat::Ani => AniParser::parse(&data)?,
    };
    
    let x11_data = convert_to_x11(frames, options)?;
    
    std::fs::write(output_path, x11_data)?;
    
    Ok(())
}

pub fn batch_convert(
    files: &[(&Path, &Path)],
    options: &ConversionOptions,
) -> Result<Vec<Result<()>>> {
    let results: Vec<Result<()>> = files
        .iter()
        .map(|(input, output)| convert_windows_cursor(input, output, options))
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
                .map(|(input, output)| convert_windows_cursor(input, output, &options))
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
        let opts = ConversionOptions::new()
            .with_scale(2.0)
            .with_shadow();
        
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
