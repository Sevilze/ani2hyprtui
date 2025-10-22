pub mod cur;
pub mod ani;
pub mod xcursor_writer;
pub mod utils;
pub mod converter;

pub use converter::ConversionOptions;
pub use cur::CurParser;
pub use ani::AniParser;

use anyhow::Result;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CursorFormat {
    Cur,
    Ani,
}

impl CursorFormat {
    pub fn detect(data: &[u8]) -> Option<Self> {
        if data.len() < 4 {
            return None;
        }
        
        if &data[0..4] == b"\x00\x00\x02\x00" {
            Some(CursorFormat::Cur)
        } else if data.len() >= 12 && &data[0..4] == b"RIFF" && &data[8..12] == b"ACON" {
            Some(CursorFormat::Ani)
        } else {
            None
        }
    }
}

pub fn parse_and_convert(path: &Path, options: &ConversionOptions) -> Result<Vec<u8>> {
    let data = std::fs::read(path)?;
    
    let format = CursorFormat::detect(&data)
        .ok_or_else(|| anyhow::anyhow!("Unsupported cursor format"))?;
    
    match format {
        CursorFormat::Cur => {
            let cursor = CurParser::parse(&data)?;
            converter::convert_to_x11(cursor, options)
        }
        CursorFormat::Ani => {
            let cursor = AniParser::parse(&data)?;
            converter::convert_to_x11(cursor, options)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cursor_format_detection() {
        // CUR format
        let cur_data = vec![0x00, 0x00, 0x02, 0x00, 0x01, 0x00];
        assert_eq!(CursorFormat::detect(&cur_data), Some(CursorFormat::Cur));

        // ANI format
        let ani_data = b"RIFF\x00\x00\x00\x00ACON";
        assert_eq!(CursorFormat::detect(ani_data), Some(CursorFormat::Ani));

        // Invalid
        let invalid = vec![0xFF, 0xFF, 0xFF, 0xFF];
        assert_eq!(CursorFormat::detect(&invalid), None);
    }

    #[test]
    #[ignore] // Requires sample file
    fn test_sample_crosshair_conversion() {
        use std::path::Path;
        
        let sample_path = Path::new("win2xcur/sample/crosshair.cur");
        if !sample_path.exists() {
            return;
        }

        let options = ConversionOptions::default();
        let result = parse_and_convert(sample_path, &options);
        
        match result {
            Ok(x11_data) => {
                assert!(x11_data.len() > 0, "Empty output");
                assert_eq!(&x11_data[0..4], b"Xcur", "Missing X11 magic bytes");
                println!("✓ Successfully converted: {} bytes", x11_data.len());
            }
            Err(e) => {
                panic!("Failed to convert sample cursor: {:?}", e);
            }
        }
    }
}
