use image::{DynamicImage, GenericImageView, Rgba, RgbaImage};
use imageproc::drawing::draw_line_segment_mut;
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Paragraph, StatefulWidget, Widget},
};
use ratatui_image::{StatefulImage, picker::Picker, protocol::StatefulProtocol};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

use crate::model::cursor::{CursorMeta, Frame, SizeVariant};

pub type PreviewData<'a> = (
    &'a str,
    (u32, u32),
    u32,
    &'a CursorMeta,
    &'a SizeVariant,
    &'a Frame,
    usize,
);

pub struct PreviewState {
    pub picker: Arc<Mutex<Picker>>,
    pub image_cache: HashMap<String, StatefulProtocol>,
}

impl PreviewState {
    pub fn new(picker: Arc<Mutex<Picker>>) -> Self {
        Self {
            picker,
            image_cache: HashMap::new(),
        }
    }

    // Helper to process image: scale to target_size canvas and draw hotspot
    fn process_image(
        &self,
        path: &str,
        hotspot: (u32, u32),
        _original_size: u32,
        target_size: (u32, u32),
    ) -> Option<DynamicImage> {
        let img = image::open(path).ok()?;
        let (w, h) = img.dimensions();

        // Canvas size
        let (canvas_w, canvas_h) = target_size;

        // Determine scale factor. We want to make it big but fit.
        // e.g. 32x32 -> 256x256 (x8)
        let scale = (canvas_w as f32 / w as f32).min(canvas_h as f32 / h as f32);

        let new_w = (w as f32 * scale) as u32;
        let new_h = (h as f32 * scale) as u32;

        let resized = img.resize_exact(new_w, new_h, image::imageops::FilterType::Nearest);

        let mut canvas = RgbaImage::new(canvas_w, canvas_h);

        // Center the resized image on canvas
        let offset_x = (canvas_w - new_w) / 2;
        let offset_y = (canvas_h - new_h) / 2;

        image::imageops::overlay(&mut canvas, &resized, offset_x as i64, offset_y as i64);

        // Draw pixel grid if scale is large enough
        if scale >= 4.0 {
            let grid_color = Rgba([128, 128, 128, 100]); // Semi-transparent gray

            // Vertical lines
            for i in 0..=w {
                let mut x = (i as f32 * scale) as i32 + offset_x as i32;
                // Clamp to be inside canvas if it's exactly on the edge
                if x == canvas_w as i32 {
                    x = canvas_w as i32 - 1;
                }

                if x >= 0 && x < canvas_w as i32 {
                    draw_line_segment_mut(
                        &mut canvas,
                        (x as f32, offset_y as f32),
                        (x as f32, (offset_y + new_h) as f32),
                        grid_color,
                    );
                }
            }

            // Horizontal lines
            for i in 0..=h {
                let mut y = (i as f32 * scale) as i32 + offset_y as i32;
                // Clamp to be inside canvas if it's exactly on the edge
                if y == canvas_h as i32 {
                    y = canvas_h as i32 - 1;
                }

                if y >= 0 && y < canvas_h as i32 {
                    draw_line_segment_mut(
                        &mut canvas,
                        (offset_x as f32, y as f32),
                        ((offset_x + new_w) as f32, y as f32),
                        grid_color,
                    );
                }
            }
        }

        // Draw hotspot
        // Hotspot is in original coordinates. Map to new coordinates.
        // We want to draw a box AROUND the pixel.
        // Pixel at (x, y) starts at (x*scale, y*scale) and has size scale*scale.
        let hx = (hotspot.0 as f32 * scale) + offset_x as f32;
        let hy = (hotspot.1 as f32 * scale) + offset_y as f32;

        let color = Rgba([255, 0, 0, 255]); // Red

        // Draw box
        // We subtract 1.0 from the end coordinates to ensure we stay INSIDE the pixel block
        // and don't overflow into the next pixel.
        let box_w = scale - 1.0;
        let box_h = scale - 1.0;

        // Top
        draw_line_segment_mut(&mut canvas, (hx, hy), (hx + box_w, hy), color);
        // Bottom
        draw_line_segment_mut(
            &mut canvas,
            (hx, hy + box_h),
            (hx + box_w, hy + box_h),
            color,
        );
        // Left
        draw_line_segment_mut(&mut canvas, (hx, hy), (hx, hy + box_h), color);
        // Right
        draw_line_segment_mut(
            &mut canvas,
            (hx + box_w, hy),
            (hx + box_w, hy + box_h),
            color,
        );

        Some(DynamicImage::ImageRgba8(canvas))
    }

    fn ensure_image_cached(
        &mut self,
        path: &str,
        hotspot: (u32, u32),
        size: u32,
        target_size: (u32, u32),
    ) {
        let key = format!("{}|{}x{}", path, target_size.0, target_size.1);
        if !self.image_cache.contains_key(&key)
            && let Some(img) = self.process_image(path, hotspot, size, target_size)
            && let Ok(picker) = self.picker.lock()
        {
            let proto = picker.new_resize_protocol(img);
            self.image_cache.insert(key, proto);
        }
    }

    pub fn invalidate_cache_for_variant(&mut self, variant: &SizeVariant) {
        let paths_to_remove: HashSet<String> = variant
            .frames
            .iter()
            .map(|f| f.png_path.to_string_lossy().to_string())
            .collect();
        self.image_cache.retain(|k, _| {
            // Key format: path|WxH
            let path = k.split('|').next().unwrap_or("");
            !paths_to_remove.contains(path)
        });
    }

    pub fn clear_cache(&mut self) {
        self.image_cache.clear();
    }

    fn center_image_rect(area: Rect) -> Rect {
        // If the area is too small, just return it
        if area.width == 0 || area.height == 0 {
            return area;
        }

        let width = area.width as f32;
        let height = area.height as f32;

        // Assume cell aspect ratio of 1:2 (width:height)
        // So a square image takes 2x width units for x height units.
        let image_aspect = 2.0;
        let area_aspect = width / height;

        if area_aspect > image_aspect {
            // Too wide: constrain width to match height * aspect
            let new_width = height * image_aspect;
            let offset_x = (width - new_width) / 2.0;
            Rect {
                x: area.x + offset_x as u16,
                y: area.y,
                width: new_width as u16,
                height: area.height,
            }
        } else {
            // Too tall: constrain height to match width / aspect
            let new_height = width / image_aspect;
            let offset_y = (height - new_height) / 2.0;
            Rect {
                x: area.x,
                y: area.y + offset_y as u16,
                width: area.width,
                height: new_height as u16,
            }
        }
    }

    pub fn render(
        &mut self,
        area: Rect,
        buf: &mut Buffer,
        _is_focused: bool,
        _playing: bool,
        data: Option<PreviewData>,
    ) {
        let chunks = Layout::default()
            .constraints([Constraint::Min(10), Constraint::Length(4)])
            .direction(ratatui::layout::Direction::Vertical)
            .split(area);

        let image_area = Self::center_image_rect(chunks[0]);

        // Calculate target pixel size
        let (font_w, font_h) = if let Ok(picker) = self.picker.lock() {
            picker.font_size()
        } else {
            (8, 16)
        };

        let target_w = (image_area.width as u32 * font_w as u32).max(1);
        let target_h = (image_area.height as u32 * font_h as u32).max(1);

        if let Some((path, hotspot, size, _, _, _, _)) = &data {
            self.ensure_image_cached(path, *hotspot, *size, (target_w, target_h));
        }

        if let Some((path, hotspot, size, cursor, variant, frame, frame_ix)) = data {
            let key = format!("{}|{}x{}", path, target_w, target_h);
            if let Some(proto) = self.image_cache.get_mut(&key) {
                StatefulImage::default().render(image_area, buf, proto);

                let info_text = format!(
                    "Frame: {}/{} | Delay: {}ms | Hotspot: ({}, {}) | Size: {}x{}",
                    frame_ix + 1,
                    variant.frames.len(),
                    frame.delay_ms,
                    hotspot.0,
                    hotspot.1,
                    size,
                    size
                );

                Paragraph::new(vec![
                    Line::from(info_text),
                    Line::from(Span::styled(
                        cursor.info(),
                        Style::default().fg(Color::Cyan),
                    )),
                ])
                .wrap(ratatui::widgets::Wrap { trim: true })
                .block(
                    ratatui::widgets::Block::default()
                        .padding(ratatui::widgets::Padding::horizontal(1)),
                )
                .render(chunks[1], buf);
            } else {
                Paragraph::new("Loading image...").render(area, buf);
            }
        }
    }
}
