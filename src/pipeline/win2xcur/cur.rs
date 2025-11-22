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

impl IconDirEntry {
    fn validate<F>(&self, mut log_fn: F) -> Result<()>
    where
        F: FnMut(String),
    {
        if self.reserved != 0 {
            bail!(
                "Invalid reserved field in ICONDIR entry: expected 0, got {}",
                self.reserved
            );
        }

        // Log a warning if color count seems suspicious
        if self.color_count != 0 && self.color_count < 2 {
            log_fn(format!(
                "Warning: Suspicious color count {} in cursor entry",
                self.color_count
            ));
        }
        Ok(())
    }
}

impl CurParser {
    pub fn can_parse(data: &[u8]) -> bool {
        data.len() >= 4 && &data[0..4] == MAGIC
    }

    pub fn parse<F>(data: &[u8], mut log_fn: F) -> Result<Vec<CursorFrame>>
    where
        F: FnMut(String),
    {
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
            entry.validate(&mut log_fn)?;
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

        let (img, is_bmp) = if image_data.len() >= 8 && &image_data[0..8] == b"\x89PNG\r\n\x1a\n" {
            (
                image::load_from_memory_with_format(image_data, image::ImageFormat::Png)
                    .context("Failed to decode PNG cursor image")?,
                false,
            )
        } else {
            let bmp_data = create_bmp_from_dib(image_data)?;
            (
                image::load_from_memory_with_format(&bmp_data, image::ImageFormat::Bmp)
                    .context("Failed to decode DIB cursor image")?,
                true,
            )
        };

        let mut rgba = img.to_rgba8();

        if is_bmp {
            apply_and_mask(&mut rgba, image_data)?;
        }

        let _width = rgba.width();
        let _height = rgba.height();

        let actual_width = if entry.width == 0 {
            256
        } else {
            entry.width as u32
        };
        let actual_height = if entry.height == 0 {
            256
        } else {
            entry.height as u32
        };

        let nominal_size = actual_width.max(actual_height);

        Ok(CursorImage {
            image: rgba,
            hotspot: (entry.hotspot_x, entry.hotspot_y),
            nominal_size,
        })
    }
}

fn apply_and_mask(image: &mut RgbaImage, dib_data: &[u8]) -> Result<()> {
    if dib_data.len() < 40 {
        return Ok(());
    }

    let header_size =
        u32::from_le_bytes([dib_data[0], dib_data[1], dib_data[2], dib_data[3]]) as usize;
    let width =
        i32::from_le_bytes([dib_data[4], dib_data[5], dib_data[6], dib_data[7]]).unsigned_abs();
    let height = i32::from_le_bytes([dib_data[8], dib_data[9], dib_data[10], dib_data[11]])
        .unsigned_abs()
        / 2;
    let bits_per_pixel = u16::from_le_bytes([dib_data[14], dib_data[15]]);

    let palette_size = calculate_palette_size(dib_data)? as usize;

    let xor_row_size = (width * bits_per_pixel as u32).div_ceil(32) * 4;
    let xor_size = xor_row_size * height;

    let and_mask_offset = header_size + palette_size + xor_size as usize;

    if dib_data.len() <= and_mask_offset {
        return Ok(());
    }

    let and_mask_data = &dib_data[and_mask_offset..];
    let and_row_size = width.div_ceil(32) * 4;

    for y in 0..height {
        for x in 0..width {
            // BMPs are stored bottom-up
            let bmp_y = height - 1 - y;

            let row_offset = (bmp_y * and_row_size) as usize;
            if row_offset >= and_mask_data.len() {
                continue;
            }

            let byte_offset = row_offset + (x / 8) as usize;
            if byte_offset >= and_mask_data.len() {
                continue;
            }

            let byte = and_mask_data[byte_offset];
            let bit = (byte >> (7 - (x % 8))) & 1;

            if bit == 1 {
                // Transparent
                if x < image.width() && y < image.height() {
                    let pixel = image.get_pixel_mut(x, y);
                    pixel[3] = 0;
                }
            }
        }
    }

    Ok(())
}

/// Create a complete BMP file from DIB data
fn create_bmp_from_dib(dib_data: &[u8]) -> Result<Vec<u8>> {
    if dib_data.len() < 40 {
        bail!("DIB data too small");
    }

    let header_size = u32::from_le_bytes([dib_data[0], dib_data[1], dib_data[2], dib_data[3]]);

    let width = i32::from_le_bytes([dib_data[4], dib_data[5], dib_data[6], dib_data[7]]);
    let height = i32::from_le_bytes([dib_data[8], dib_data[9], dib_data[10], dib_data[11]]);
    let actual_height = height / 2;

    let mut modified_dib = dib_data.to_vec();
    let actual_height_bytes = actual_height.to_le_bytes();
    modified_dib[8] = actual_height_bytes[0];
    modified_dib[9] = actual_height_bytes[1];
    modified_dib[10] = actual_height_bytes[2];
    modified_dib[11] = actual_height_bytes[3];

    // Calculate how much data we need (only the XOR mask)
    let palette_size = calculate_palette_size(&modified_dib)?;
    let bits_per_pixel = u16::from_le_bytes([dib_data[14], dib_data[15]]);

    let row_size = (width.unsigned_abs() * bits_per_pixel as u32).div_ceil(32) * 4;
    let xor_mask_size = row_size * actual_height.unsigned_abs();
    let dib_data_size = header_size + palette_size + xor_mask_size;

    // Truncate to only include XOR mask data
    if modified_dib.len() > dib_data_size as usize {
        modified_dib.truncate(dib_data_size as usize);
    }

    let file_size = 14 + modified_dib.len() as u32;
    let pixel_data_offset = 14 + header_size + palette_size;

    let mut bmp_data = Vec::new();

    // BMP file header
    bmp_data.write_all(b"BM")?; // Signature
    bmp_data.write_u32::<LittleEndian>(file_size)?;
    bmp_data.write_u16::<LittleEndian>(0)?; // Reserved1
    bmp_data.write_u16::<LittleEndian>(0)?; // Reserved2
    bmp_data.write_u32::<LittleEndian>(pixel_data_offset)?;

    bmp_data.write_all(&modified_dib)?;

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
