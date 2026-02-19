//! Tile manager for graphical backends.
//!
//! Implements [`gruid_core::TileManager`] to render shamogu with custom
//! monochrome tile images instead of font-based glyphs.
//!
//! Tiles are pre-rendered 16×24 monochrome bitmaps embedded at compile time.
//! Map cells are identified by the [`ATTR_IN_MAP`] attribute flag on the
//! cell's style.

use gruid_core::style::AttrMask;

/// Custom attribute flag indicating a cell is part of the game map
/// (as opposed to UI text).
pub const ATTR_IN_MAP: AttrMask = AttrMask(1 << 8);

/// Tile manager that uses embedded monochrome tile images.
#[derive(Default)]
pub struct ShamoguTileManager;

impl ShamoguTileManager {
    pub fn new() -> Self {
        Self
    }
}

impl gruid_core::TileManager for ShamoguTileManager {
    fn tile_size(&self) -> (usize, usize) {
        (crate::tile_data::TILE_WIDTH, crate::tile_data::TILE_HEIGHT)
    }

    fn get_tile(&self, cell: &gruid_core::Cell) -> Option<&[u8]> {
        let ch = cell.ch;
        let in_map = cell.style.attrs.contains(ATTR_IN_MAP);

        if in_map {
            if let Some(tile) = crate::tile_data::map_tile(ch) {
                return Some(tile);
            }
        }

        crate::tile_data::letter_tile(ch)
    }
}
