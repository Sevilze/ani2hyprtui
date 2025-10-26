use anyhow::{Context, Result, bail};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use image::RgbaImage;
use std::io::{Cursor, Write};

const ICO_TYPE_CUR: u16 = 2;
const MAGIC: &[u8] = &[0x00, 0x00, 0x02, 0x00];

#[derive(Debug, Clone)]
pub struct CursorImage {
    pub image: RgbaImage,
    pub hotspot: (u16, u16),
    pub nominal_size: u32,
}

#[derive(Debug, Clone)]
pub struct CursorFrame {
    pub images: Vec<CursorImage>,
    pub delay: u32,
}

pub struct CurParser;

#[derive(Debug)]
struct IconDirEntry {
    width: u8,
    height: u8,
    color_count: u8,
    reserved: u8,
    hotspot_x: u16,
    hotspot_y: u16,
    size_bytes: u32,
    offset: u32,
}

impl CurParser {
    pub fn can_parse(data: &[u8]) -> bool {
        data.len() >= 4 && &data[0..4] == MAGIC
    }

    pub fn parse(data: &[u8]) -> Result<Vec<CursorFrame>> {
        if !Self::can_parse(data) {
            bail!("Not a valid .CUR file");
        }

        let mut cursor = Cursor::new(data);
        
        // Read ICONDIR header
        let reserved = cursor.read_u16::<LittleEndian>()?;
        let ico_type = cursor.read_u16::<LittleEndian>()?;
        let image_count = cursor.read_u16::<LittleEndian>()?;

        if reserved != 0 {
            bail!("Invalid reserved field in CUR header");
        }
        if ico_type != ICO_TYPE_CUR {
            bail!("Not a cursor file (type must be 2)");
        }

        // Read directory entries
        let mut entries = Vec::new();
        for _ in 0..image_count {
            let entry = Self::read_dir_entry(&mut cursor)?;
            entries.push(entry);
        }

        let mut cursor_images = Vec::new();
        for entry in entries {
            let image = Self::parse_image(data, &entry)?;
            cursor_images.push(image);
        }

        Ok(vec![CursorFrame {
            images: cursor_images,
            delay: 0,
        }])
    }

    fn read_dir_entry(cursor: &mut Cursor<&[u8]>) -> Result<IconDirEntry> {
        Ok(IconDirEntry {
            width: cursor.read_u8()?,
            height: cursor.read_u8()?,
            color_count: cursor.read_u8()?,
            reserved: cursor.read_u8()?,
            hotspot_x: cursor.read_u16::<LittleEndian>()?,
            hotspot_y: cursor.read_u16::<LittleEndian>()?,
            size_bytes: cursor.read_u32::<LittleEndian>()?,
            offset: cursor.read_u32::<LittleEndian>()?,
        })
    }

    fn parse_image(data: &[u8], entry: &IconDirEntry) -> Result<CursorImage> {
        let offset = entry.offset as usize;
        let size = entry.size_bytes as usize;
        
        if offset + size > data.len() {
            bail!("Image data extends beyond file bounds");
        }

        let image_data = &data[offset..offset + size];
        
        let img = if image_data.len() >= 8 && &image_data[0..8] == b"\x89PNG\r\n\x1a\n" {
            image::load_from_memory_with_format(image_data, image::ImageFormat::Png)
                .context("Failed to decode PNG cursor image")?
        } else {
            let bmp_data = create_bmp_from_dib(image_data)?;
            image::load_from_memory_with_format(&bmp_data, image::ImageFormat::Bmp)
                .context("Failed to decode DIB cursor image")?
        };

        let rgba = img.to_rgba8();
        let _width = rgba.width();
        let _height = rgba.height();

        let actual_width = if entry.width == 0 { 256 } else { entry.width as u32 };
        let actual_height = if entry.height == 0 { 256 } else { entry.height as u32 };

        let nominal_size = actual_width.max(actual_height);

        Ok(CursorImage {
            image: rgba,
            hotspot: (entry.hotspot_x, entry.hotspot_y),
            nominal_size,
        })
    }
}

/// Create a complete BMP file from DIB data
fn create_bmp_from_dib(dib_data: &[u8]) -> Result<Vec<u8>> {
    if dib_data.len() < 40 {
        bail!("DIB data too small");
    }
    
    let header_size = u32::from_le_bytes([dib_data[0], dib_data[1], dib_data[2], dib_data[3]]);
    let file_size = 14 + dib_data.len() as u32;
    let pixel_data_offset = 14 + header_size + calculate_palette_size(dib_data)?;
    
    let mut bmp_data = Vec::new();
    
    // BMP file header
    bmp_data.write_all(b"BM")?; // Signature
    bmp_data.write_u32::<LittleEndian>(file_size)?;
    bmp_data.write_u16::<LittleEndian>(0)?;  // Reserved1
    bmp_data.write_u16::<LittleEndian>(0)?;  // Reserved2
    bmp_data.write_u32::<LittleEndian>(pixel_data_offset)?;
    
    bmp_data.write_all(dib_data)?;
    
    Ok(bmp_data)
}

fn calculate_palette_size(dib_data: &[u8]) -> Result<u32> {
    if dib_data.len() < 40 {
        return Ok(0);
    }
    
    // Read bit depth from DIB header (offset 14 in DIB header)
    let bits_per_pixel = u16::from_le_bytes([dib_data[14], dib_data[15]]);
    
    // Read colors used from DIB header (offset 32 in DIB header)
    let colors_used = u32::from_le_bytes([dib_data[32], dib_data[33], dib_data[34], dib_data[35]]);
    
    let palette_entries = if colors_used > 0 {
        colors_used
    } else if bits_per_pixel <= 8 {
        1 << bits_per_pixel
    } else {
        0
    };
    
    // Each palette entry is 4 bytes (RGBQUAD)
    Ok(palette_entries * 4)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_magic_detection() {
        let valid = vec![0x00, 0x00, 0x02, 0x00, 0x01, 0x00];
        assert!(CurParser::can_parse(&valid));

        let invalid = vec![0x00, 0x00, 0x01, 0x00];
        assert!(!CurParser::can_parse(&invalid));
    }
}
