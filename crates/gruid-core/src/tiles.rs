//! Tile rendering support.
//!
//! The [`TileManager`] trait allows graphical backends to render grid cells
//! using custom tile images instead of (or in addition to) font glyphs.

use crate::Cell;

/// A tile manager provides custom tile images for grid cells.
///
/// Each tile is a **monochrome alpha bitmap** (one byte per pixel,
/// 0 = background, 255 = foreground). The graphical backend colorizes
/// tiles at render time using each cell's foreground/background colors.
///
/// When [`get_tile`](TileManager::get_tile) returns `None`, the backend
/// falls back to font-based glyph rendering.
pub trait TileManager: Send + 'static {
    /// Tile size in pixels (width, height). All tiles must be this size.
    fn tile_size(&self) -> (usize, usize);

    /// Return the monochrome alpha bitmap for the given cell, if a custom
    /// tile exists. Return `None` to fall back to font-based rendering.
    /// The returned slice must have exactly `tile_width * tile_height` bytes.
    fn get_tile(&self, cell: &Cell) -> Option<&[u8]>;
}
