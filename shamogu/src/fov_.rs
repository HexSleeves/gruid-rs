//! FOV (Field of View) integration with game map.

use gruid_core::Point;
use gruid_rl::fov::Lighter;
use gruid_rl::grid::Grid as RlGrid;

use crate::gamemap::MAX_FOV_RANGE;
use crate::terrain::*;

/// Lighter implementation for the game map.
pub struct MapLighter<'a> {
    pub terrain: &'a RlGrid,
}

impl Lighter for MapLighter<'_> {
    fn max_cost(&self, _src: Point) -> i32 {
        MAX_FOV_RANGE
    }

    fn cost(&self, _src: Point, _from: Point, to: Point) -> i32 {
        match self.terrain.at(to).unwrap_or(WALL) {
            WALL | RUBBLE => -1,
            FOLIAGE => MAX_FOV_RANGE - 2,
            _ => 1,
        }
    }
}
