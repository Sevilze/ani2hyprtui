use anyhow::Result;
use image::{ImageFormat, RgbaImage};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct PngWriteConfig {
    pub filename: String,
    pub size: u32,
    pub xhot: u32,
    pub yhot: u32,
    pub delay: u32,
}

pub fn write_png(image: &RgbaImage, path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    image.save_with_format(path, ImageFormat::Png)?;
    Ok(())
}

pub fn format_config_line(config: &PngWriteConfig) -> String {
    format!(
        "{}\t{}\t{}\t{}\t{}",
        config.size, config.xhot, config.yhot, config.filename, config.delay
    )
}

pub fn write_config_file(path: &Path, configs: &[PngWriteConfig]) -> Result<()> {
    use std::io::Write;

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let mut file = fs::File::create(path)?;
    writeln!(file, "#size\txhot\tyhot\tPath to PNG image\tdelay")?;

    for config in configs {
        writeln!(file, "{}", format_config_line(config))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgba;
    use tempfile::tempdir;

    #[test]
    fn test_write_png() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.png");

        let mut image = RgbaImage::new(32, 32);
        for y in 0..32 {
            for x in 0..32 {
                image.put_pixel(x, y, Rgba([255, 0, 0, 255]));
            }
        }

        write_png(&image, &path).unwrap();
        assert!(path.exists());

        let loaded = image::open(&path).unwrap();
        assert_eq!(loaded.width(), 32);
        assert_eq!(loaded.height(), 32);
    }

    #[test]
    fn test_format_config_line() {
        let config = PngWriteConfig {
            filename: "cursor_001.png".to_string(),
            size: 32,
            xhot: 16,
            yhot: 16,
            delay: 50,
        };

        let line = format_config_line(&config);
        assert_eq!(line, "32\t16\t16\tcursor_001.png\t50");
    }

    #[test]
    fn test_write_config_file() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("cursor.conf");

        let configs = vec![
            PngWriteConfig {
                filename: "cursor_001.png".to_string(),
                size: 32,
                xhot: 16,
                yhot: 16,
                delay: 50,
            },
            PngWriteConfig {
                filename: "cursor_002.png".to_string(),
                size: 32,
                xhot: 16,
                yhot: 16,
                delay: 50,
            },
        ];

        write_config_file(&path, &configs).unwrap();
        assert!(path.exists());

        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("#size"));
        assert!(content.contains("cursor_001.png"));
        assert!(content.contains("cursor_002.png"));
    }
}
