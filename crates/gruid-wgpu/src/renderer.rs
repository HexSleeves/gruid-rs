//! GPU-accelerated grid renderer using wgpu.
//!
//! Renders the grid as instanced quads. Each cell is one quad instance with:
//! - position (col, row)
//! - fg/bg colors
//! - atlas UV rectangle for the glyph bitmap
//!
//! Glyph bitmaps are rasterized via fontdue and packed into a single-channel
//! texture atlas. Custom tiles from a [`TileManager`] are also packed into
//! the atlas.

use std::collections::HashMap;

use bytemuck::{Pod, Zeroable};
use fontdue::{Font, FontSettings};
use gruid_core::{Cell, grid::Frame, style::Color};

use crate::TileManager;

// ---------------------------------------------------------------------------
// GPU types (must match grid.wgsl)
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub(crate) struct Uniforms {
    pub cell_size: [f32; 4],   // cell_w, cell_h, atlas_w, atlas_h
    pub screen_size: [f32; 2], // pixel width, pixel height
    pub _pad: [f32; 2],
}

/// Per-instance data for one grid cell.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub(crate) struct CellInstance {
    pub grid_pos: [f32; 2],   // col, row
    pub fg_color: u32,        // packed RGBA
    pub bg_color: u32,        // packed RGBA
    pub atlas_rect: [f32; 4], // x, y, w, h in texels (w=0 means no glyph)
}

// ---------------------------------------------------------------------------
// Atlas
// ---------------------------------------------------------------------------

struct AtlasEntry {
    x: u32,
    y: u32,
    w: u32,
    h: u32,
}

pub(crate) struct GlyphAtlas {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>, // single-channel (R8)
    entries: HashMap<AtlasKey, AtlasEntry>,
    cursor_x: u32,
    cursor_y: u32,
    row_height: u32,
}

#[derive(Hash, Eq, PartialEq, Clone)]
enum AtlasKey {
    Char(char),
    // Tile(char, u32, u32), // reserved for future tile-variant caching
}

const FALLBACK_FONT: &[u8] = include_bytes!("../../gruid-winit/src/builtin_font.ttf");

impl GlyphAtlas {
    fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            data: vec![0; (width * height) as usize],
            entries: HashMap::new(),
            cursor_x: 0,
            cursor_y: 0,
            row_height: 0,
        }
    }

    /// Insert a glyph bitmap into the atlas. Returns the rect.
    fn insert(&mut self, key: AtlasKey, bitmap: &[u8], w: u32, h: u32) -> AtlasEntry {
        if let Some(e) = self.entries.get(&key) {
            return AtlasEntry {
                x: e.x,
                y: e.y,
                w: e.w,
                h: e.h,
            };
        }

        // Advance to next row if needed
        if self.cursor_x + w > self.width {
            self.cursor_x = 0;
            self.cursor_y += self.row_height;
            self.row_height = 0;
        }

        // Grow atlas if needed (double height)
        while self.cursor_y + h > self.height {
            let old_h = self.height;
            self.height *= 2;
            self.data.resize((self.width * self.height) as usize, 0);
            log::debug!(
                "Atlas grew to {}x{} (was {})",
                self.width,
                old_h,
                self.height
            );
        }

        let x = self.cursor_x;
        let y = self.cursor_y;

        // Copy bitmap into atlas
        for row in 0..h {
            let src_start = (row * w) as usize;
            let dst_start = ((y + row) * self.width + x) as usize;
            let len = w as usize;
            if src_start + len <= bitmap.len() && dst_start + len <= self.data.len() {
                self.data[dst_start..dst_start + len]
                    .copy_from_slice(&bitmap[src_start..src_start + len]);
            }
        }

        self.cursor_x += w;
        self.row_height = self.row_height.max(h);

        let entry = AtlasEntry { x, y, w, h };
        self.entries.insert(key, AtlasEntry { x, y, w, h });
        entry
    }

    fn get(&self, key: &AtlasKey) -> Option<&AtlasEntry> {
        self.entries.get(key)
    }
}

// ---------------------------------------------------------------------------
// GridRenderer
// ---------------------------------------------------------------------------

pub(crate) struct GridRenderer {
    font: Font,
    font_size: f32,
    pub cell_width: usize,
    pub cell_height: usize,
    pub cols: usize,
    pub rows: usize,
    pub atlas: GlyphAtlas,
    /// Instance buffer data (rebuilt each frame from dirty cells).
    pub instances: Vec<CellInstance>,
    /// Whether the instance buffer needs re-upload.
    pub dirty: bool,
    /// Whether the atlas texture needs re-upload.
    pub atlas_dirty: bool,
    tile_manager: Option<Box<dyn TileManager>>,
    tile_scale: u32,
}

impl GridRenderer {
    pub fn new(
        font_data: Option<&[u8]>,
        font_size: f32,
        cols: usize,
        rows: usize,
        tile_manager: Option<Box<dyn TileManager>>,
        tile_scale: u32,
    ) -> Self {
        let tile_scale = tile_scale.max(1);
        let data = font_data.unwrap_or(FALLBACK_FONT);
        let font = Font::from_bytes(data, FontSettings::default()).expect("failed to parse font");

        let (cell_width, cell_height) = if let Some(ref tm) = tile_manager {
            let (tw, th) = tm.tile_size();
            let s = tile_scale as usize;
            ((tw * s).max(1), (th * s).max(1))
        } else {
            let metrics = font
                .horizontal_line_metrics(font_size)
                .unwrap_or(fontdue::LineMetrics {
                    ascent: font_size * 0.8,
                    descent: -(font_size * 0.2),
                    line_gap: 0.0,
                    new_line_size: font_size,
                });
            let ch = (metrics.ascent - metrics.descent).ceil() as usize;
            let (m_metrics, _) = font.rasterize('M', font_size);
            let cw = m_metrics.advance_width.ceil() as usize;
            (cw.max(1), ch.max(1))
        };

        let n = cols * rows;
        let instances: Vec<CellInstance> = (0..n)
            .map(|i| {
                let col = i % cols;
                let row = i / cols;
                CellInstance {
                    grid_pos: [col as f32, row as f32],
                    fg_color: pack_color(Color::DEFAULT, true),
                    bg_color: pack_color(Color::DEFAULT, false),
                    atlas_rect: [0.0, 0.0, 0.0, 0.0],
                }
            })
            .collect();

        // Start with a reasonable atlas size
        let atlas_w = 1024u32;
        let atlas_h = 512u32;

        Self {
            font,
            font_size,
            cell_width,
            cell_height,
            cols,
            rows,
            atlas: GlyphAtlas::new(atlas_w, atlas_h),
            instances,
            dirty: true,
            atlas_dirty: true,
            tile_manager,
            tile_scale,
        }
    }

    pub fn pixel_width(&self) -> usize {
        self.cols * self.cell_width
    }

    pub fn pixel_height(&self) -> usize {
        self.rows * self.cell_height
    }

    pub fn resize_grid(&mut self, cols: usize, rows: usize) {
        self.cols = cols;
        self.rows = rows;
        let n = cols * rows;
        self.instances.resize(
            n,
            CellInstance {
                grid_pos: [0.0, 0.0],
                fg_color: pack_color(Color::DEFAULT, true),
                bg_color: pack_color(Color::DEFAULT, false),
                atlas_rect: [0.0, 0.0, 0.0, 0.0],
            },
        );
        // Recompute all positions
        for i in 0..n {
            let col = i % cols;
            let row = i / cols;
            self.instances[i].grid_pos = [col as f32, row as f32];
        }
        self.dirty = true;
    }

    /// Apply a frame diff.
    pub fn apply_frame(&mut self, frame: &Frame) {
        let old_atlas_len = self.atlas.entries.len();

        for fc in &frame.cells {
            let col = fc.pos.x as usize;
            let row = fc.pos.y as usize;
            if col >= self.cols || row >= self.rows {
                continue;
            }
            let idx = row * self.cols + col;
            let cell = &fc.cell;

            self.instances[idx].fg_color = pack_color(cell.style.fg, true);
            self.instances[idx].bg_color = pack_color(cell.style.bg, false);
            self.instances[idx].atlas_rect = self.rasterize_cell(cell);
        }

        self.dirty = true;
        if self.atlas.entries.len() != old_atlas_len {
            self.atlas_dirty = true;
        }
    }

    /// Rasterize a cell's glyph/tile and return its atlas rect.
    fn rasterize_cell(&mut self, cell: &Cell) -> [f32; 4] {
        // Try tile manager first
        if let Some(ref tm) = self.tile_manager {
            if let Some(bitmap) = tm.get_tile(cell) {
                let (tw, th) = tm.tile_size();
                let s = self.tile_scale as usize;
                let scaled_w = tw * s;
                let scaled_h = th * s;

                // Scale the bitmap
                let mut scaled = vec![0u8; scaled_w * scaled_h];
                for ty in 0..th {
                    for tx in 0..tw {
                        let src_idx = ty * tw + tx;
                        let alpha = if src_idx < bitmap.len() {
                            bitmap[src_idx]
                        } else {
                            0
                        };
                        for sy in 0..s {
                            for sx in 0..s {
                                let di = (ty * s + sy) * scaled_w + (tx * s + sx);
                                scaled[di] = alpha;
                            }
                        }
                    }
                }

                let key = AtlasKey::Char(cell.ch); // tiles keyed by char
                let entry = self
                    .atlas
                    .insert(key, &scaled, scaled_w as u32, scaled_h as u32);
                return [
                    entry.x as f32,
                    entry.y as f32,
                    entry.w as f32,
                    entry.h as f32,
                ];
            }
        }

        let ch = cell.ch;
        if ch == ' ' || ch == '\0' {
            return [0.0, 0.0, 0.0, 0.0];
        }

        // Check atlas cache
        let key = AtlasKey::Char(ch);
        if let Some(e) = self.atlas.get(&key) {
            return [e.x as f32, e.y as f32, e.w as f32, e.h as f32];
        }

        // Rasterize glyph
        let (metrics, bitmap) = self.font.rasterize(ch, self.font_size);
        if metrics.width == 0 || metrics.height == 0 {
            return [0.0, 0.0, 0.0, 0.0];
        }

        // Composite glyph into a cell-sized bitmap so the atlas entry
        // matches cell dimensions and the shader doesn't need per-glyph offsets.
        let cw = self.cell_width as u32;
        let ch_px = self.cell_height as u32;
        let mut cell_bitmap = vec![0u8; (cw * ch_px) as usize];

        let font_metrics = self.font.horizontal_line_metrics(self.font_size);
        let ascent = font_metrics
            .map(|m| m.ascent.ceil() as i32)
            .unwrap_or(ch_px as i32);

        let glyph_y = ascent - metrics.ymin - metrics.height as i32;
        let glyph_x = metrics.xmin;

        for gy in 0..metrics.height {
            for gx in 0..metrics.width {
                let alpha = bitmap[gy * metrics.width + gx];
                if alpha == 0 {
                    continue;
                }
                let px = glyph_x + gx as i32;
                let py = glyph_y + gy as i32;
                if px < 0 || py < 0 || px >= cw as i32 || py >= ch_px as i32 {
                    continue;
                }
                cell_bitmap[(py as u32 * cw + px as u32) as usize] = alpha;
            }
        }

        let entry = self.atlas.insert(key, &cell_bitmap, cw, ch_px);
        [
            entry.x as f32,
            entry.y as f32,
            entry.w as f32,
            entry.h as f32,
        ]
    }

    pub fn uniforms(&self) -> Uniforms {
        Uniforms {
            cell_size: [
                self.cell_width as f32,
                self.cell_height as f32,
                self.atlas.width as f32,
                self.atlas.height as f32,
            ],
            screen_size: [self.pixel_width() as f32, self.pixel_height() as f32],
            _pad: [0.0; 2],
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn pack_color(c: Color, is_fg: bool) -> u32 {
    let (r, g, b) = if c == Color::DEFAULT {
        if is_fg {
            (200u8, 200u8, 200u8)
        } else {
            (0u8, 0u8, 0u8)
        }
    } else {
        (c.r(), c.g(), c.b())
    };
    (r as u32) | ((g as u32) << 8) | ((b as u32) << 16) | (0xFF << 24)
}
