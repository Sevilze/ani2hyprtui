use anyhow::Result;
use byteorder::{LittleEndian, WriteBytesExt};
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
            let width = cursor.image.width();
            let height = cursor.image.height();
            let (hotspot_x, hotspot_y) = cursor.hotspot;
            let nominal = cursor.nominal_size;
            let delay = frame.delay;
            
            let pixels = premultiply_alpha(&cursor.image);
            
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
    use image::{Rgba, RgbaImage};
    use crate::pipeline::win2xcur::cur::CursorImage;

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
}
