use image::{DynamicImage, GenericImageView, Rgba, RgbaImage};
use imageproc::drawing::draw_line_segment_mut;
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    style::Style,
    text::Line,
    widgets::{Paragraph, StatefulWidget, Widget},
};
use ratatui_image::{StatefulImage, picker::Picker, protocol::StatefulProtocol};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

use crate::model::cursor::{CursorMeta, Frame, SizeVariant};
use crate::widgets::theme::get_theme;

pub type PreviewData<'a> = (
    &'a str,
    (u32, u32),
    u32,
    &'a CursorMeta,
    &'a SizeVariant,
    &'a Frame,
    usize,
);

// Cached base image data (resized image + grid)
struct BaseImageData {
    canvas: RgbaImage,
    scale: f32,
    offset_x: u32,
    offset_y: u32,
}

pub struct PreviewState {
    pub picker: Arc<Mutex<Picker>>,
    base_cache: HashMap<String, BaseImageData>,
    // Cache for final encoded protocols: "path|WxH|hx,hy" -> ready to render
    protocol_cache: HashMap<String, StatefulProtocol>,
}

impl PreviewState {
    pub fn new(picker: Arc<Mutex<Picker>>) -> Self {
        Self {
            picker,
            base_cache: HashMap::new(),
            protocol_cache: HashMap::new(),
        }
    }

    fn base_key(path: &str, target_size: (u32, u32)) -> String {
        format!("{}|{}x{}", path, target_size.0, target_size.1)
    }

    fn proto_key(path: &str, target_size: (u32, u32), hotspot: (u32, u32)) -> String {
        format!(
            "{}|{}x{}|{},{}",
            path, target_size.0, target_size.1, hotspot.0, hotspot.1
        )
    }

    fn process_base_image(path: &str, target_size: (u32, u32)) -> Option<BaseImageData> {
        let img = image::open(path).ok()?;
        let (w, h) = img.dimensions();
        let (canvas_w, canvas_h) = target_size;

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

        Some(BaseImageData {
            canvas,
            scale,
            offset_x,
            offset_y,
        })
    }

    fn draw_hotspot(
        canvas: &mut RgbaImage,
        hotspot: (u32, u32),
        scale: f32,
        offset_x: u32,
        offset_y: u32,
    ) {
        let hx = (hotspot.0 as f32 * scale) + offset_x as f32;
        let hy = (hotspot.1 as f32 * scale) + offset_y as f32;
        let color = Rgba([255, 0, 0, 255]);

        let box_w = scale - 1.0;
        let box_h = scale - 1.0;

        // Top
        draw_line_segment_mut(canvas, (hx, hy), (hx + box_w, hy), color);
        // Bottom
        draw_line_segment_mut(canvas, (hx, hy + box_h), (hx + box_w, hy + box_h), color);
        // Left
        draw_line_segment_mut(canvas, (hx, hy), (hx, hy + box_h), color);
        // Right
        draw_line_segment_mut(canvas, (hx + box_w, hy), (hx + box_w, hy + box_h), color);
    }

    fn ensure_cached(&mut self, path: &str, hotspot: (u32, u32), target_size: (u32, u32)) {
        let proto_key = Self::proto_key(path, target_size, hotspot);

        if self.protocol_cache.contains_key(&proto_key) {
            return;
        }

        let base_key = Self::base_key(path, target_size);

        if !self.base_cache.contains_key(&base_key) {
            if let Some(base_data) = Self::process_base_image(path, target_size) {
                self.base_cache.insert(base_key.clone(), base_data);
            } else {
                return;
            }
        }

        if let Some(base_data) = self.base_cache.get(&base_key) {
            let mut final_canvas = base_data.canvas.clone();

            Self::draw_hotspot(
                &mut final_canvas,
                hotspot,
                base_data.scale,
                base_data.offset_x,
                base_data.offset_y,
            );

            // Encode to protocol
            if let Ok(picker) = self.picker.lock() {
                let proto = picker.new_resize_protocol(DynamicImage::ImageRgba8(final_canvas));
                self.protocol_cache.insert(proto_key, proto);
            }
        }
    }

    // Invalidate protocol cache only for a variant
    pub fn invalidate_protocol_for_variant(&mut self, variant: &SizeVariant) {
        let paths_to_remove: HashSet<String> = variant
            .frames
            .iter()
            .map(|f| f.png_path.to_string_lossy().to_string())
            .collect();

        // Only remove from protocol cache, keep base images
        self.protocol_cache.retain(|k, _| {
            let path = k.split('|').next().unwrap_or("");
            !paths_to_remove.contains(path)
        });
    }

    pub fn clear_cache(&mut self) {
        self.base_cache.clear();
        self.protocol_cache.clear();
    }

    fn center_image_rect(area: Rect) -> Rect {
        if area.width == 0 || area.height == 0 {
            return area;
        }

        let width = area.width as f32;
        let height = area.height as f32;
        let image_aspect = 2.2;
        let area_aspect = width / height;

        if area_aspect > image_aspect {
            let new_width = height * image_aspect;
            let offset_x = (width - new_width) / 2.0;
            Rect {
                x: area.x + offset_x as u16,
                y: area.y,
                width: new_width as u16,
                height: area.height,
            }
        } else {
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
        maximized: bool,
        data: Option<PreviewData>,
    ) {
        let chunks = if maximized {
            Layout::default()
                .constraints([Constraint::Percentage(100)])
                .split(area)
        } else {
            Layout::default()
                .constraints([Constraint::Min(10), Constraint::Length(1)])
                .direction(ratatui::layout::Direction::Vertical)
                .split(area)
        };

        let image_area = Self::center_image_rect(chunks[0]);

        let (font_w, font_h) = if let Ok(picker) = self.picker.lock() {
            picker.font_size()
        } else {
            (8, 16)
        };

        let target_w = (image_area.width as u32 * font_w as u32).max(1);
        let target_h = (image_area.height as u32 * font_h as u32).max(1);

        if let Some((path, hotspot, _, _, _, _, _)) = &data {
            self.ensure_cached(path, *hotspot, (target_w, target_h));
        }

        if let Some((path, hotspot, size, _, variant, frame, frame_ix)) = data {
            let key = Self::proto_key(path, (target_w, target_h), hotspot);

            if let Some(proto) = self.protocol_cache.get_mut(&key) {
                StatefulImage::default().render(image_area, buf, proto);

                let (text_content, text_area) = if maximized {
                    let lines = vec![
                        Line::from(format!("Frame: {}/{}", frame_ix + 1, variant.frames.len())),
                        Line::from(format!("Delay: {}ms", frame.delay_ms)),
                        Line::from(format!("Hotspot: ({}, {})", hotspot.0, hotspot.1)),
                        Line::from(format!("Size: {}x{}", size, size)),
                    ];
                    let height = lines.len() as u16;
                    let width = lines.iter().map(|l| l.width()).max().unwrap_or(0) as u16 + 2;
                    let centered_y = area.y + (area.height.saturating_sub(height)) / 2;
                    (lines, Rect::new(area.x, centered_y, width, height))
                } else {
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
                    (vec![Line::from(info_text)], chunks[1])
                };

                let theme = get_theme();
                let paragraph = Paragraph::new(text_content);

                if maximized {
                    paragraph
                        .style(Style::default().bg(theme.background))
                        .block(
                            ratatui::widgets::Block::default()
                                .padding(ratatui::widgets::Padding::left(1)),
                        )
                        .render(text_area, buf);
                } else {
                    paragraph
                        .block(
                            ratatui::widgets::Block::default()
                                .padding(ratatui::widgets::Padding::left(3)),
                        )
                        .render(text_area, buf);
                }
            } else {
                Paragraph::new("Loading image...").render(area, buf);
            }
        }
    }
}
