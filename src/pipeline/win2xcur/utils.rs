use anyhow::Result;
use image::{ImageBuffer, Rgba, RgbaImage};
use imageproc::filter::gaussian_blur_f32;

use super::cur::CursorFrame;

pub fn scale_frames(frames: &mut [CursorFrame], scale: f32) {
    for frame in frames {
        for cursor in &mut frame.images {
            let width = cursor.image.width();
            let height = cursor.image.height();

            let new_width = (width as f32 * scale).round() as u32;
            let new_height = (height as f32 * scale).round() as u32;

            let scaled = image::imageops::resize(
                &cursor.image,
                new_width,
                new_height,
                image::imageops::FilterType::Lanczos3,
            );

            cursor.image = scaled;
            cursor.nominal_size = new_width.max(new_height);

            cursor.hotspot.0 = (cursor.hotspot.0 as f32 * scale).round() as u16;
            cursor.hotspot.1 = (cursor.hotspot.1 as f32 * scale).round() as u16;
        }
    }
}

#[derive(Debug, Clone)]
pub struct ShadowConfig {
    pub color: [u8; 3],
    pub radius: f32,
    pub sigma: f32,
    pub x_offset: f32,
    pub y_offset: f32,
    pub opacity: u8,
}

impl Default for ShadowConfig {
    fn default() -> Self {
        Self {
            color: [0, 0, 0],
            radius: 0.1,
            sigma: 0.1,
            x_offset: 0.05,
            y_offset: 0.05,
            opacity: 128, // 50%
        }
    }
}

pub fn apply_shadows(frames: &mut [CursorFrame], config: &ShadowConfig) -> Result<()> {
    for frame in frames {
        for cursor in &mut frame.images {
            let shadowed = apply_shadow_to_image(&cursor.image, config)?;
            cursor.image = shadowed;
        }
    }
    Ok(())
}

fn apply_shadow_to_image(image: &RgbaImage, config: &ShadowConfig) -> Result<RgbaImage> {
    let width = image.width();
    let height = image.height();

    let x_offset = (config.x_offset * width as f32).round() as i32;
    let y_offset = (config.y_offset * height as f32).round() as i32;

    let new_width = width + (3 * x_offset.unsigned_abs());
    let new_height = height + (3 * y_offset.unsigned_abs());

    let mut alpha_mask = ImageBuffer::new(new_width, new_height);
    for (_x, _y, pixel) in alpha_mask.enumerate_pixels_mut() {
        *pixel = Rgba([255, 255, 255, 0]);
    }

    for y in 0..height {
        for x in 0..width {
            let pixel = image.get_pixel(x, y);
            let alpha = pixel[3];
            alpha_mask.put_pixel(
                x + x_offset as u32,
                y + y_offset as u32,
                Rgba([255, 255, 255, alpha]),
            );
        }
    }

    // Apply Gaussian blur to create shadow
    let sigma = config.sigma * width as f32;
    let blurred = gaussian_blur_f32(&alpha_mask, sigma);

    // Create shadow layer
    let mut shadow = ImageBuffer::new(new_width, new_height);
    for (x, y, pixel) in shadow.enumerate_pixels_mut() {
        let blur_alpha = blurred.get_pixel(x, y)[3];
        let shadow_alpha = ((blur_alpha as u16 * config.opacity as u16) / 255) as u8;
        *pixel = Rgba([
            config.color[0],
            config.color[1],
            config.color[2],
            shadow_alpha,
        ]);
    }

    let mut result = ImageBuffer::new(new_width, new_height);
    for (_x, _y, pixel) in result.enumerate_pixels_mut() {
        *pixel = Rgba([0, 0, 0, 0]);
    }

    composite_over(&mut result, &shadow, 0, 0);
    composite_over(&mut result, image, 0, 0);

    // Trim to minimum size while keeping original image fully visible
    let trimmed = trim_to_content(&result, width, height);

    Ok(trimmed)
}

/// Composite source over destination using alpha blending
fn composite_over(dst: &mut RgbaImage, src: &RgbaImage, x_offset: i32, y_offset: i32) {
    for y in 0..src.height() {
        for x in 0..src.width() {
            let dst_x = x as i32 + x_offset;
            let dst_y = y as i32 + y_offset;

            if dst_x >= 0 && dst_y >= 0 && dst_x < dst.width() as i32 && dst_y < dst.height() as i32
            {
                let src_pixel = src.get_pixel(x, y);
                let dst_pixel = dst.get_pixel(dst_x as u32, dst_y as u32);

                let blended = blend_over(*src_pixel, *dst_pixel);
                dst.put_pixel(dst_x as u32, dst_y as u32, blended);
            }
        }
    }
}

/// Alpha blend: src over dst
fn blend_over(src: Rgba<u8>, dst: Rgba<u8>) -> Rgba<u8> {
    let src_a = src[3] as f32 / 255.0;
    let dst_a = dst[3] as f32 / 255.0;

    let out_a = src_a + dst_a * (1.0 - src_a);

    if out_a == 0.0 {
        return Rgba([0, 0, 0, 0]);
    }

    let r = ((src[0] as f32 * src_a + dst[0] as f32 * dst_a * (1.0 - src_a)) / out_a) as u8;
    let g = ((src[1] as f32 * src_a + dst[1] as f32 * dst_a * (1.0 - src_a)) / out_a) as u8;
    let b = ((src[2] as f32 * src_a + dst[2] as f32 * dst_a * (1.0 - src_a)) / out_a) as u8;
    let a = (out_a * 255.0) as u8;

    Rgba([r, g, b, a])
}

fn trim_to_content(image: &RgbaImage, min_width: u32, min_height: u32) -> RgbaImage {
    let (width, height) = (image.width(), image.height());

    let mut min_x = width;
    let mut max_x = 0;
    let mut min_y = height;
    let mut max_y = 0;

    for y in 0..height {
        for x in 0..width {
            let pixel = image.get_pixel(x, y);
            if pixel[3] > 0 {
                min_x = min_x.min(x);
                max_x = max_x.max(x);
                min_y = min_y.min(y);
                max_y = max_y.max(y);
            }
        }
    }

    if min_x > max_x {
        return RgbaImage::new(min_width, min_height);
    }

    let content_width = (max_x - min_x + 1).max(min_width);
    let content_height = (max_y - min_y + 1).max(min_height);

    let mut result = RgbaImage::new(content_width, content_height);
    for y in 0..content_height {
        for x in 0..content_width {
            let src_x = min_x + x;
            let src_y = min_y + y;
            if src_x < width && src_y < height {
                let pixel = image.get_pixel(src_x, src_y);
                result.put_pixel(x, y, *pixel);
            }
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scale_frames() {
        let img = RgbaImage::new(32, 32);
        let cursor = super::super::cur::CursorImage {
            image: img,
            hotspot: (16, 16),
            nominal_size: 32,
        };
        let mut frames = vec![super::super::cur::CursorFrame {
            images: vec![cursor],
            delay: 0,
        }];

        scale_frames(&mut frames, 2.0);

        assert_eq!(frames[0].images[0].image.width(), 64);
        assert_eq!(frames[0].images[0].image.height(), 64);
        assert_eq!(frames[0].images[0].hotspot, (32, 32));
    }

    #[test]
    fn test_blend_over() {
        let src = Rgba([255, 0, 0, 128]);
        let dst = Rgba([0, 0, 255, 255]);

        let result = blend_over(src, dst);

        assert!(result[0] > 0);
        assert!(result[2] > 0);
        assert_eq!(result[3], 255);
    }
}
