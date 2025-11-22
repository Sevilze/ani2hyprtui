use anyhow::{Result, anyhow};
use byteorder::{LittleEndian, ReadBytesExt};
use image::{Rgba, RgbaImage};
use std::io::{Cursor, Read};
use std::path::Path;

const XCURSOR_MAGIC: &[u8] = b"Xcur";
const XCURSOR_VERSION: u32 = 0x0001_0000;
const XCURSOR_IMAGE_TYPE: u32 = 0xfffd0002;

#[derive(Debug, Clone)]
pub struct XcursorImage {
    pub size: u32,
    pub width: u32,
    pub height: u32,
    pub xhot: u32,
    pub yhot: u32,
    pub delay: u32,
    pub pixels: RgbaImage,
}

#[derive(Debug)]
pub struct XcursorFile {
    pub images: Vec<XcursorImage>,
}

impl XcursorFile {
    pub fn from_file(path: &Path) -> Result<Self> {
        let data = std::fs::read(path)?;
        Self::from_bytes(&data)
    }

    pub fn from_bytes(data: &[u8]) -> Result<Self> {
        let mut cursor = Cursor::new(data);

        // Read and validate magic
        let mut magic = [0u8; 4];
        cursor.read_exact(&mut magic)?;
        if magic != XCURSOR_MAGIC {
            return Err(anyhow!("Invalid Xcursor magic bytes"));
        }

        let header_size = cursor.read_u32::<LittleEndian>()?;
        if header_size != 16 {
            return Err(anyhow!("Invalid Xcursor header size: {}", header_size));
        }

        let version = cursor.read_u32::<LittleEndian>()?;
        if version != XCURSOR_VERSION {
            return Err(anyhow!("Unsupported Xcursor version: 0x{:08x}", version));
        }

        let ntoc = cursor.read_u32::<LittleEndian>()?;

        // Read TOC
        let mut toc_entries = Vec::new();
        for _ in 0..ntoc {
            let chunk_type = cursor.read_u32::<LittleEndian>()?;
            let chunk_subtype = cursor.read_u32::<LittleEndian>()?;
            let chunk_position = cursor.read_u32::<LittleEndian>()?;

            if chunk_type == XCURSOR_IMAGE_TYPE {
                toc_entries.push((chunk_subtype, chunk_position));
            }
        }

        // Read image chunks
        let mut images = Vec::new();
        for (size, position) in toc_entries {
            cursor.set_position(position as u64);

            // Read chunk header
            let chunk_header = cursor.read_u32::<LittleEndian>()?;
            let chunk_type = cursor.read_u32::<LittleEndian>()?;
            let _chunk_size = cursor.read_u32::<LittleEndian>()?;

            if chunk_type != XCURSOR_IMAGE_TYPE {
                continue;
            }

            if chunk_header != 36 {
                return Err(anyhow!("Invalid chunk header size: {}", chunk_header));
            }

            // Read image header
            let version = cursor.read_u32::<LittleEndian>()?;
            if version != 1 {
                return Err(anyhow!("Unsupported image version: {}", version));
            }

            let width = cursor.read_u32::<LittleEndian>()?;
            let height = cursor.read_u32::<LittleEndian>()?;
            let xhot = cursor.read_u32::<LittleEndian>()?;
            let yhot = cursor.read_u32::<LittleEndian>()?;
            let delay = cursor.read_u32::<LittleEndian>()?;

            // Read pixels (BGRA format with premultiplied alpha)
            let _pixel_count = (width * height) as usize;
            let mut pixels = RgbaImage::new(width, height);

            for y in 0..height {
                for x in 0..width {
                    let b = cursor.read_u8()?;
                    let g = cursor.read_u8()?;
                    let r = cursor.read_u8()?;
                    let a = cursor.read_u8()?;

                    // Undo premultiplied alpha
                    let (r_out, g_out, b_out) = if a == 0 {
                        (255, 255, 255)
                    } else {
                        let alpha_factor = 255.0 / a as f64;
                        let r_unpre = ((r as f64 * alpha_factor).min(255.0)) as u8;
                        let g_unpre = ((g as f64 * alpha_factor).min(255.0)) as u8;
                        let b_unpre = ((b as f64 * alpha_factor).min(255.0)) as u8;
                        (r_unpre, g_unpre, b_unpre)
                    };

                    pixels.put_pixel(x, y, Rgba([r_out, g_out, b_out, a]));
                }
            }

            images.push(XcursorImage {
                size,
                width,
                height,
                xhot,
                yhot,
                delay,
                pixels,
            });
        }

        if images.is_empty() {
            return Err(anyhow!("No valid cursor images found"));
        }

        Ok(XcursorFile { images })
    }

    /// Get the nominal size of cursors in this file
    pub fn get_sizes(&self) -> Vec<u32> {
        let mut sizes: Vec<u32> = self.images.iter().map(|img| img.size).collect();
        sizes.sort_unstable();
        sizes.dedup();
        sizes
    }

    /// Get images for a specific nominal size
    pub fn get_images_for_size(&self, size: u32) -> Vec<&XcursorImage> {
        self.images.iter().filter(|img| img.size == size).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_xcursor_magic_validation() {
        let invalid_data = b"INVALID";
        let result = XcursorFile::from_bytes(invalid_data);
        assert!(result.is_err());
    }

    #[test]
    fn test_xcursor_parsing() {
        let mut data = Vec::new();

        data.extend_from_slice(b"Xcur");
        data.extend_from_slice(&16u32.to_le_bytes()); // header size
        data.extend_from_slice(&0x0001_0000u32.to_le_bytes()); // version
        data.extend_from_slice(&1u32.to_le_bytes()); // ntoc

        // TOC entry
        data.extend_from_slice(&0xfffd0002u32.to_le_bytes()); // type
        data.extend_from_slice(&32u32.to_le_bytes()); // subtype (size)
        data.extend_from_slice(&28u32.to_le_bytes()); // position

        // Image chunk
        data.extend_from_slice(&36u32.to_le_bytes()); // chunk header
        data.extend_from_slice(&0xfffd0002u32.to_le_bytes()); // type
        data.extend_from_slice(&32u32.to_le_bytes()); // nominal size
        data.extend_from_slice(&1u32.to_le_bytes()); // version
        data.extend_from_slice(&2u32.to_le_bytes()); // width
        data.extend_from_slice(&2u32.to_le_bytes()); // height
        data.extend_from_slice(&1u32.to_le_bytes()); // xhot
        data.extend_from_slice(&1u32.to_le_bytes()); // yhot
        data.extend_from_slice(&0u32.to_le_bytes()); // delay

        // Pixels (2x2 BGRA)
        for _ in 0..4 {
            data.extend_from_slice(&[255, 128, 64, 255]); // BGRA
        }

        let xcursor = XcursorFile::from_bytes(&data).unwrap();
        assert_eq!(xcursor.images.len(), 1);
        assert_eq!(xcursor.images[0].width, 2);
        assert_eq!(xcursor.images[0].height, 2);
        assert_eq!(xcursor.images[0].xhot, 1);
        assert_eq!(xcursor.images[0].yhot, 1);
    }
}
