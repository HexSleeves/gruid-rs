use gruid_core::Point;
use image::{Rgba, RgbaImage};
use rusttype::{Font, Scale, point as rt_point};

/// Error type for Drawer construction.
#[derive(Debug)]
pub enum DrawerError {
    /// The font data could not be parsed.
    InvalidFont,
}

impl std::fmt::Display for DrawerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DrawerError::InvalidFont => write!(f, "invalid font data"),
        }
    }
}

impl std::error::Error for DrawerError {}

/// Renders characters as RGBA tile images using a TrueType font.
pub struct Drawer {
    font: Font<'static>,
    scale: Scale,
    tile_width: u32,
    tile_height: u32,
    baseline_y: f32,
}

impl Drawer {
    /// Create a new drawer from raw TrueType font data and a pixel scale.
    ///
    /// The scale controls the font size; tile dimensions are derived from the
    /// font metrics at that scale (using the 'M' glyph for width).
    pub fn new(font_data: &[u8], scale: f32) -> Result<Self, DrawerError> {
        let _probe = Font::try_from_bytes_and_index(font_data, 0)
            .or_else(|| Font::try_from_bytes(font_data))
            .ok_or(DrawerError::InvalidFont)?;

        // We need an owned 'static font, so clone the data.
        let owned_data: Vec<u8> = font_data.to_vec();
        let font = Font::try_from_vec(owned_data).ok_or(DrawerError::InvalidFont)?;

        let sc = Scale::uniform(scale);
        let v_metrics = font.v_metrics(sc);
        let tile_height = (v_metrics.ascent - v_metrics.descent + v_metrics.line_gap).ceil() as u32;
        let baseline_y = v_metrics.ascent;

        // Use 'M' glyph to determine tile width (monospace assumption).
        let glyph = font.glyph('M').scaled(sc);
        let h_metrics = glyph.h_metrics();
        let tile_width = h_metrics.advance_width.ceil() as u32;

        Ok(Self {
            font,
            scale: sc,
            tile_width: tile_width.max(1),
            tile_height: tile_height.max(1),
            baseline_y,
        })
    }

    /// Render a single character as an RGBA image with the given foreground
    /// and background colors (each `[r, g, b, a]`).
    pub fn draw(&self, ch: char, fg: [u8; 4], bg: [u8; 4]) -> RgbaImage {
        let mut img = RgbaImage::from_pixel(self.tile_width, self.tile_height, Rgba(bg));

        let glyph = self
            .font
            .glyph(ch)
            .scaled(self.scale)
            .positioned(rt_point(0.0, self.baseline_y));

        if let Some(bb) = glyph.pixel_bounding_box() {
            glyph.draw(|gx, gy, v| {
                let px = (gx as i32 + bb.min.x) as u32;
                let py = (gy as i32 + bb.min.y) as u32;
                if px < self.tile_width && py < self.tile_height {
                    // Alpha-blend foreground over background.
                    let alpha = v;
                    let r = blend(bg[0], fg[0], alpha);
                    let g = blend(bg[1], fg[1], alpha);
                    let b = blend(bg[2], fg[2], alpha);
                    let a = blend(bg[3], fg[3], alpha);
                    img.put_pixel(px, py, Rgba([r, g, b, a]));
                }
            });
        }

        img
    }

    /// Return the tile size in grid units (width, height) as a [`Point`].
    pub fn tile_size(&self) -> Point {
        Point::new(self.tile_width as i32, self.tile_height as i32)
    }
}

/// Simple alpha-blend of two u8 color channels.
fn blend(bg: u8, fg: u8, alpha: f32) -> u8 {
    ((1.0 - alpha) * bg as f32 + alpha * fg as f32) as u8
}
