//! Renders the gruid grid into a pixel buffer using fontdue for glyph
//! rasterization.
//!
//! Each grid cell is rendered as a monospace character tile with
//! foreground/background colors.

use std::collections::HashMap;

use fontdue::{Font, FontSettings};
use gruid_core::{grid::Frame, style::Color};

/// Default built-in font (DejaVu Sans Mono subset would be ideal, but for
/// size we embed nothing and fall back to fontdue's glyph-not-found box).
/// Users should supply their own font via [`WinitConfig::font_data`].
const FALLBACK_FONT: &[u8] = include_bytes!("builtin_font.ttf");

/// Cached rasterized glyph.
struct GlyphCache {
    bitmap: Vec<u8>, // alpha values, width*height
    width: usize,
    height: usize,
    x_offset: i32,
    y_offset: i32,
}

pub(crate) struct GridRenderer {
    font: Font,
    font_size: f32,
    cell_width: usize,
    cell_height: usize,
    cols: usize,
    rows: usize,
    /// RGBA pixel buffer (cell_width*cols) x (cell_height*rows)
    pixels: Vec<u32>,
    /// Glyph cache keyed by character
    glyph_cache: HashMap<char, GlyphCache>,
}

impl GridRenderer {
    pub fn new(font_data: Option<&[u8]>, font_size: f32, cols: usize, rows: usize) -> Self {
        let data = font_data.unwrap_or(FALLBACK_FONT);
        let font = Font::from_bytes(data, FontSettings::default()).expect("failed to parse font");

        // Compute cell size from font metrics
        let metrics = font
            .horizontal_line_metrics(font_size)
            .unwrap_or(fontdue::LineMetrics {
                ascent: font_size * 0.8,
                descent: -(font_size * 0.2),
                line_gap: 0.0,
                new_line_size: font_size,
            });

        let cell_height = (metrics.ascent - metrics.descent).ceil() as usize;
        // Use 'M' to determine cell width for monospace
        let (m_metrics, _) = font.rasterize('M', font_size);
        let cell_width = m_metrics.advance_width.ceil() as usize;

        let cell_width = cell_width.max(1);
        let cell_height = cell_height.max(1);

        let pixel_count = (cols * cell_width) * (rows * cell_height);
        let pixels = vec![0xFF000000; pixel_count]; // opaque black

        Self {
            font,
            font_size,
            cell_width,
            cell_height,
            cols,
            rows,
            pixels,
            glyph_cache: HashMap::new(),
        }
    }

    /// Cell size in pixels.
    pub fn cell_size(&self) -> (usize, usize) {
        (self.cell_width, self.cell_height)
    }

    /// Total pixel buffer width.
    pub fn pixel_width(&self) -> usize {
        self.cols * self.cell_width
    }

    /// Total pixel buffer height.
    pub fn pixel_height(&self) -> usize {
        self.rows * self.cell_height
    }

    /// Resize the internal grid (re-allocates pixel buffer).
    pub fn resize_grid(&mut self, cols: usize, rows: usize) {
        self.cols = cols;
        self.rows = rows;
        let pixel_count = self.pixel_width() * self.pixel_height();
        self.pixels.resize(pixel_count, 0xFF000000);
        self.pixels.fill(0xFF000000);
    }

    /// Apply a frame diff to the internal pixel buffer.
    pub fn apply_frame(&mut self, frame: &Frame) {
        for fc in &frame.cells {
            let col = fc.pos.x as usize;
            let row = fc.pos.y as usize;
            if col >= self.cols || row >= self.rows {
                continue;
            }
            self.draw_cell(col, row, fc.cell.ch, fc.cell.style.fg, fc.cell.style.bg);
        }
    }

    /// Ensure a glyph is cached, rasterizing it if needed.
    fn cache_glyph(&mut self, ch: char) {
        if self.glyph_cache.contains_key(&ch) {
            return;
        }
        let (metrics, bitmap) = self.font.rasterize(ch, self.font_size);
        self.glyph_cache.insert(
            ch,
            GlyphCache {
                bitmap,
                width: metrics.width,
                height: metrics.height,
                x_offset: metrics.xmin,
                y_offset: metrics.ymin,
            },
        );
    }

    /// Draw a single cell into the pixel buffer.
    fn draw_cell(&mut self, col: usize, row: usize, ch: char, fg: Color, bg: Color) {
        let cw = self.cell_width;
        let ch_px = self.cell_height;
        let buf_w = self.pixel_width();
        let px_h = self.pixel_height();
        let x0 = col * cw;
        let y0 = row * ch_px;

        let bg_pixel = color_to_pixel(bg);

        // Fill background
        for dy in 0..ch_px {
            let row_start = (y0 + dy) * buf_w + x0;
            if row_start + cw <= self.pixels.len() {
                for dx in 0..cw {
                    self.pixels[row_start + dx] = bg_pixel;
                }
            }
        }

        // Rasterize and draw glyph
        if ch == ' ' || ch == '\0' {
            return;
        }

        let (fg_r, fg_g, fg_b) = color_rgb(fg);
        let (bg_r, bg_g, bg_b) = color_rgb(bg);

        // Ensure glyph is cached
        self.cache_glyph(ch);
        let glyph = &self.glyph_cache[&ch];

        if glyph.width == 0 || glyph.height == 0 {
            return;
        }

        // Compute baseline position
        let font_metrics = self.font.horizontal_line_metrics(self.font_size);
        let ascent = font_metrics
            .map(|m| m.ascent.ceil() as i32)
            .unwrap_or(ch_px as i32);

        // glyph_y is the top-left pixel of the glyph bitmap relative to cell top
        let glyph_y = ascent - glyph.y_offset - glyph.height as i32;
        let glyph_x = glyph.x_offset;

        // Copy glyph info to locals to release borrow on self
        let gw = glyph.width;
        let gh = glyph.height;
        let gx_off = glyph_x;
        let gy_off = glyph_y;

        for gy in 0..gh {
            for gx in 0..gw {
                let alpha = self.glyph_cache[&ch].bitmap[gy * gw + gx];
                if alpha == 0 {
                    continue;
                }

                let px = x0 as i32 + gx_off + gx as i32;
                let py = y0 as i32 + gy_off + gy as i32;

                if px < 0 || py < 0 {
                    continue;
                }
                let px = px as usize;
                let py = py as usize;
                if px >= buf_w || py >= px_h {
                    continue;
                }

                let idx = py * buf_w + px;
                if idx >= self.pixels.len() {
                    continue;
                }

                // Alpha-blend foreground over background
                let a = alpha as u32;
                let inv_a = 255 - a;
                let r = (fg_r as u32 * a + bg_r as u32 * inv_a) / 255;
                let g = (fg_g as u32 * a + bg_g as u32 * inv_a) / 255;
                let b = (fg_b as u32 * a + bg_b as u32 * inv_a) / 255;
                self.pixels[idx] = 0xFF000000 | (r << 16) | (g << 8) | b;
            }
        }
    }

    /// Copy the internal pixel buffer into the softbuffer surface buffer.
    pub fn blit_to_buffer(&self, buf: &mut [u32], buf_width: usize, buf_height: usize) {
        let src_w = self.pixel_width();
        let src_h = self.pixel_height();
        let copy_w = src_w.min(buf_width);
        let copy_h = src_h.min(buf_height);

        // Clear areas outside the grid
        if buf_width > src_w || buf_height > src_h {
            for pixel in buf.iter_mut() {
                *pixel = 0xFF000000; // black
            }
        }

        for y in 0..copy_h {
            let src_start = y * src_w;
            let dst_start = y * buf_width;
            let src_end = src_start + copy_w;
            let dst_end = dst_start + copy_w;
            if src_end <= self.pixels.len() && dst_end <= buf.len() {
                buf[dst_start..dst_end].copy_from_slice(&self.pixels[src_start..src_end]);
            }
        }
    }
}

#[inline]
fn color_to_pixel(c: Color) -> u32 {
    if c == Color::DEFAULT {
        0xFF000000 // black
    } else {
        0xFF000000 | ((c.r() as u32) << 16) | ((c.g() as u32) << 8) | (c.b() as u32)
    }
}

#[inline]
fn color_rgb(c: Color) -> (u8, u8, u8) {
    if c == Color::DEFAULT {
        (200, 200, 200) // light gray for default foreground
    } else {
        (c.r(), c.g(), c.b())
    }
}
