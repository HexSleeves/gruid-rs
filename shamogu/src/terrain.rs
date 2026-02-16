//! Terrain types and helpers.

use gruid_rl::grid::Cell as RlCell;

pub const WALL: RlCell = RlCell(0);
pub const FLOOR: RlCell = RlCell(1);
pub const FOLIAGE: RlCell = RlCell(2);
pub const RUBBLE: RlCell = RlCell(3);
pub const TRANSLUCENT_WALL: RlCell = RlCell(4);
pub const UNKNOWN_PASSABLE: RlCell = RlCell(5);
pub const UNKNOWN: RlCell = RlCell(6);

/// Whether a terrain cell is passable.
pub fn passable(c: RlCell) -> bool {
    matches!(c, FLOOR | FOLIAGE | RUBBLE)
}

/// Whether a terrain cell blocks vision.
pub fn blocks_los(c: RlCell) -> bool {
    matches!(c, WALL | RUBBLE)
}

/// Whether terrain is known (not Unknown).
pub fn is_known(c: RlCell) -> bool {
    c != UNKNOWN
}

/// Character representation of terrain.
pub fn terrain_rune(c: RlCell) -> char {
    match c {
        WALL => '#',
        FLOOR => '.',
        FOLIAGE => '"',
        RUBBLE => '^',
        TRANSLUCENT_WALL => '◊',
        UNKNOWN_PASSABLE => '♫',
        _ => ' ',
    }
}
