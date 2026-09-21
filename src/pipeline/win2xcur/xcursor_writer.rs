use anyhow::Result;
use byteorder::{LittleEndian, WriteBytesExt};
use image::imageops::FilterType;
use std::io::Write;

use super::cur::CursorFrame;

const MAGIC: &[u8] = b"Xcur";
const VERSION: u32 = 0x0001_0000;
const CHUNK_IMAGE: u32 = 0xFFFD_0002;

pub fn to_x11(frames: &[CursorFrame]) -> Result<Vec<u8>> {
    let mut output = Vec::new();
    let mut chunks = Vec::new();

    for frame in frames {
        for cursor in &frame.images {
            let nominal = cursor.nominal_size;
            let actual_w = cursor.image.width();
            let actual_h = cursor.image.height();

            // Resize image to match its nominal size if they differ
            // This ensures consistent sizing between xcursor and hyprcursor outputs
            let (image, hotspot_x, hotspot_y) = if (actual_w != nominal || actual_h != nominal)
                && actual_w > 0
                && actual_h > 0
            {
                let scale_x = nominal as f32 / actual_w as f32;
                let scale_y = nominal as f32 / actual_h as f32;
                (
                    image::imageops::resize(&cursor.image, nominal, nominal, FilterType::Lanczos3),
                    (cursor.hotspot.0 as f32 * scale_x).round() as u16,
                    (cursor.hotspot.1 as f32 * scale_y).round() as u16,
                )
            } else {
                (cursor.image.clone(), cursor.hotspot.0, cursor.hotspot.1)
            };

            let width = image.width();
            let height = image.height();
            let delay = frame.delay;

            let pixels = premultiply_alpha(&image);

            chunks.push(ChunkData {
                chunk_type: CHUNK_IMAGE,
                nominal,
                width,
                height,
                hotspot_x,
                hotspot_y,
                delay,
                pixels,
            });
        }
    }

    output.write_all(MAGIC)?;
    output.write_u32::<LittleEndian>(16)?; // header size
    output.write_u32::<LittleEndian>(VERSION)?;
    output.write_u32::<LittleEndian>(chunks.len() as u32)?;

    let toc_size = chunks.len() * 12; // Each TOC entry is 12 bytes
    let mut offset = 16 + toc_size; // After header and TOC

    for chunk in &chunks {
        output.write_u32::<LittleEndian>(chunk.chunk_type)?;
        output.write_u32::<LittleEndian>(chunk.nominal)?;
        output.write_u32::<LittleEndian>(offset as u32)?;

        let image_size = chunk.pixels.len();
        offset += 36 + image_size; // 36 byte header + image data
    }

    for chunk in &chunks {
        output.write_u32::<LittleEndian>(36)?; // header size
        output.write_u32::<LittleEndian>(chunk.chunk_type)?;
        output.write_u32::<LittleEndian>(chunk.nominal)?;
        output.write_u32::<LittleEndian>(1)?; // version
        output.write_u32::<LittleEndian>(chunk.width)?;
        output.write_u32::<LittleEndian>(chunk.height)?;
        output.write_u32::<LittleEndian>(chunk.hotspot_x as u32)?;
        output.write_u32::<LittleEndian>(chunk.hotspot_y as u32)?;
        output.write_u32::<LittleEndian>(chunk.delay)?;

        // Image data (BGRA format)
        output.write_all(&chunk.pixels)?;
    }

    Ok(output)
}

struct ChunkData {
    chunk_type: u32,
    nominal: u32,
    width: u32,
    height: u32,
    hotspot_x: u16,
    hotspot_y: u16,
    delay: u32,
    pixels: Vec<u8>,
}

fn premultiply_alpha(image: &image::RgbaImage) -> Vec<u8> {
    let mut result = Vec::with_capacity((image.width() * image.height() * 4) as usize);

    for pixel in image.pixels() {
        let r = pixel[0] as f64;
        let g = pixel[1] as f64;
        let b = pixel[2] as f64;
        let a = pixel[3] as f64;

        let alpha_factor = a / 255.0;

        let b_pre = (b * alpha_factor) as u8;
        let g_pre = (g * alpha_factor) as u8;
        let r_pre = (r * alpha_factor) as u8;
        let a_byte = a as u8;

        result.push(b_pre);
        result.push(g_pre);
        result.push(r_pre);
        result.push(a_byte);
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pipeline::win2xcur::cur::CursorImage;
    use image::{Rgba, RgbaImage};

    #[test]
    fn test_premultiply_alpha() {
        let mut img = RgbaImage::new(2, 2);
        img.put_pixel(0, 0, Rgba([255, 255, 255, 128]));
        img.put_pixel(1, 0, Rgba([255, 0, 0, 255]));

        let result = premultiply_alpha(&img);

        assert!(result[0] >= 127 && result[0] <= 128);
        assert!(result[1] >= 127 && result[1] <= 128);
        assert!(result[2] >= 127 && result[2] <= 128);
        assert_eq!(result[3], 128);

        assert_eq!(result[4], 0);
        assert_eq!(result[5], 0);
        assert_eq!(result[6], 255);
        assert_eq!(result[7], 255);
    }

    #[test]
    fn test_xcursor_format() {
        let mut img = RgbaImage::new(32, 32);
        for y in 0..32 {
            for x in 0..32 {
                img.put_pixel(x, y, Rgba([255, 255, 255, 255]));
            }
        }

        let cursor = CursorImage {
            image: img,
            hotspot: (16, 16),
            nominal_size: 32,
        };

        let frame = CursorFrame {
            images: vec![cursor],
            delay: 0,
        };

        let result = to_x11(&[frame]).unwrap();

        assert_eq!(&result[0..4], b"Xcur");

        let version = u32::from_le_bytes([result[8], result[9], result[10], result[11]]);
        assert_eq!(version, 0x0001_0000);
    }

    #[test]
    fn test_xcursor_resizes_and_scales_hotspot() {
        let img = RgbaImage::new(64, 64);
        let cursor = CursorImage {
            image: img,
            hotspot: (32, 32),
            nominal_size: 32,
        };

        let frame = CursorFrame {
            images: vec![cursor],
            delay: 0,
        };

        let result = to_x11(&[frame]).unwrap();
        // Offset for first chunk header: 16 (header) + 12 (TOC) = 28
        let chunk_offset = 28;
        let width = u32::from_le_bytes([
            result[chunk_offset + 16],
            result[chunk_offset + 17],
            result[chunk_offset + 18],
            result[chunk_offset + 19],
        ]);
        let height = u32::from_le_bytes([
            result[chunk_offset + 20],
            result[chunk_offset + 21],
            result[chunk_offset + 22],
            result[chunk_offset + 23],
        ]);
        let hotspot_x = u32::from_le_bytes([
            result[chunk_offset + 24],
            result[chunk_offset + 25],
            result[chunk_offset + 26],
            result[chunk_offset + 27],
        ]);
        let hotspot_y = u32::from_le_bytes([
            result[chunk_offset + 28],
            result[chunk_offset + 29],
            result[chunk_offset + 30],
            result[chunk_offset + 31],
        ]);

        assert_eq!(width, 32);
        assert_eq!(height, 32);
        assert_eq!(hotspot_x, 16);
        assert_eq!(hotspot_y, 16);
    }
}
