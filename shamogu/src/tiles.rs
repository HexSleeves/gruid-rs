//! Tile manager for the winit graphical backend.
//!
//! Implements [`gruid_winit::TileManager`] to render shamogu with custom
//! monochrome tile images instead of font-based glyphs. This matches the
//! Go shamogu SDL/JS tile rendering.
//!
//! Tiles are pre-rendered 16×24 monochrome bitmaps embedded at compile time.
//! The tile manager distinguishes between "map tiles" (terrain, entities,
//! effects on the game map) and "letter tiles" (UI text, menus, messages).
//!
//! Map cells are identified by the [`AttrInMap`] attribute flag on the cell's
//! style. When set, the tile manager looks up a map-specific tile image for
//! the character. When not set, it looks up a letter tile.

use gruid_core::style::AttrMask;

/// Custom attribute flag indicating a cell is part of the game map
/// (as opposed to UI text). Map cells use map tile images; other cells
/// use letter tile images.
///
/// This matches Go shamogu's `AttrInMap` constant.
pub const ATTR_IN_MAP: AttrMask = AttrMask(1 << 8);

/// Tile manager that uses embedded monochrome tile images.
///
/// Construct with [`ShamoguTileManager::new()`] and pass to
/// [`gruid_winit::WinitConfig::tile_manager`].
#[derive(Default)]
pub struct ShamoguTileManager;

impl ShamoguTileManager {
    pub fn new() -> Self {
        Self
    }

    /// Tile size in pixels (backend-agnostic).
    pub fn tile_size_raw(&self) -> (usize, usize) {
        (crate::tile_data::TILE_WIDTH, crate::tile_data::TILE_HEIGHT)
    }

    /// Get tile bitmap for a cell (backend-agnostic).
    pub fn get_tile_raw(&self, cell: &gruid_core::Cell) -> Option<&[u8]> {
        let ch = cell.ch;
        let in_map = cell.style.attrs.contains(ATTR_IN_MAP);
        if in_map {
            if let Some(tile) = crate::tile_data::map_tile(ch) {
                return Some(tile);
            }
        }
        if let Some(tile) = crate::tile_data::letter_tile(ch) {
            return Some(tile);
        }
        None
    }
}

#[cfg(feature = "winit")]
impl gruid_winit::TileManager for ShamoguTileManager {
    fn tile_size(&self) -> (usize, usize) {
        self.tile_size_raw()
    }

    fn get_tile(&self, cell: &gruid_core::Cell) -> Option<&[u8]> {
        self.get_tile_raw(cell)
    }
}
