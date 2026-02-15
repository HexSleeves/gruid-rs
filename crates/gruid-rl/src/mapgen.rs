//! Map generation algorithms for roguelike games.
//!
//! Provides two cave generators:
//! - **Random Walk Cave**: uses a drunk-walk approach to carve open space.
//! - **Cellular Automata Cave**: initializes random walls then smooths
//!   with cellular automata rules.

use crate::grid::{Cell, Grid};
use gruid_core::Point;
use rand::Rng;

/// Trait for choosing a random neighbor during random-walk cave generation.
pub trait RandomWalker {
    /// Given a position `p`, return a random neighbor using `rng`.
    fn neighbor(&self, p: Point, rng: &mut impl Rng) -> Point;
}

/// A simple 4-directional random walker.
pub struct FourDirectionWalker;

impl RandomWalker for FourDirectionWalker {
    fn neighbor(&self, p: Point, rng: &mut impl Rng) -> Point {
        match rng.random_range(0..4u32) {
            0 => Point::new(p.x + 1, p.y),
            1 => Point::new(p.x - 1, p.y),
            2 => Point::new(p.x, p.y + 1),
            _ => Point::new(p.x, p.y - 1),
        }
    }
}

/// A rule for one iteration of cellular automata smoothing.
#[derive(Debug, Clone)]
pub struct CellularAutomataRule {
    /// If a cell has >= this many wall neighbors in the 1-ring (8 neighbors),
    /// it becomes a wall.
    pub w_cutoff1: i32,
    /// If a cell has >= this many wall neighbors in the 2-ring
    /// (24 neighbors), it becomes a wall.
    pub w_cutoff2: i32,
    /// Whether cells outside the grid boundary count as walls.
    pub walls_out_of_range: bool,
    /// How many times to apply this rule.
    pub reps: usize,
}

impl Default for CellularAutomataRule {
    fn default() -> Self {
        Self {
            w_cutoff1: 5,
            w_cutoff2: 2,
            walls_out_of_range: true,
            reps: 4,
        }
    }
}

/// Map generator operating on an [`Grid`] of [`Cell`] values.
pub struct MapGen<R: Rng> {
    pub rng: R,
    pub grid: Grid,
}

impl<R: Rng> MapGen<R> {
    /// Create a new MapGen with the given grid.
    pub fn with_grid(grid: Grid, rng: R) -> Self {
        Self { rng, grid }
    }

    /// Generate a cave using random walk.
    ///
    /// Starting from the center of the grid, perform `walks` random walks.
    /// Each walk carves out cells by setting them to `cell`.
    /// The walk continues until the proportion of `cell` cells reaches
    /// `fill_pct` (0.0–1.0) of the total area.
    ///
    /// Returns the number of cells carved.
    pub fn random_walk_cave(
        &mut self,
        walker: &impl RandomWalker,
        cell: Cell,
        fill_pct: f64,
        walks: usize,
    ) -> usize {
        let sz = self.grid.size();
        let w = sz.x;
        let h = sz.y;
        let total = (w * h) as usize;
        let target = (total as f64 * fill_pct) as usize;
        let bounds = self.grid.bounds();
        let mut carved = 0usize;

        // Start at center.
        let start = Point::new(
            bounds.min.x + w / 2,
            bounds.min.y + h / 2,
        );

        for _ in 0..walks {
            let mut pos = start;
            let step_limit = total * 4; // safety limit per walk

            for _ in 0..step_limit {
                if carved >= target {
                    return carved;
                }

                if self.grid.at(pos) != Some(cell) {
                    self.grid.set(pos, cell);
                    carved += 1;
                }

                // Walk to a neighbor, clamped to bounds.
                let next = walker.neighbor(pos, &mut self.rng);
                if bounds.contains(next) {
                    pos = next;
                }
                // else stay put
            }
        }

        carved
    }

    /// Generate a cave using cellular automata.
    ///
    /// 1. Initialize each cell randomly: `wall_init_pct` chance of being `wall`,
    ///    otherwise `ground`.
    /// 2. Apply each rule in `rules` for its specified number of repetitions.
    ///
    /// Returns the number of ground cells in the final grid.
    pub fn cellular_automata_cave(
        &mut self,
        wall: Cell,
        ground: Cell,
        wall_init_pct: f64,
        rules: &[CellularAutomataRule],
    ) -> usize {
        let bounds = self.grid.bounds();
        let sz = self.grid.size();
        let w = sz.x;
        let h = sz.y;

        // Step 1: random initialization.
        for p in bounds.iter() {
            let r: f64 = self.rng.random();
            if r < wall_init_pct {
                self.grid.set(p, wall);
            } else {
                self.grid.set(p, ground);
            }
        }

        // Step 2: apply rules.
        // We need a scratch buffer for the next generation.
        let mut scratch = vec![Cell::default(); (w * h) as usize];

        for rule in rules {
            for _ in 0..rule.reps {
                // Compute next generation into scratch.
                for p in bounds.iter() {
                    let walls1 = self.count_walls_ring(
                        p, 1, wall, rule.walls_out_of_range,
                    );
                    let walls2 = self.count_walls_ring(
                        p, 2, wall, rule.walls_out_of_range,
                    );

                    let idx = ((p.y - bounds.min.y) * w + (p.x - bounds.min.x)) as usize;
                    if walls1 >= rule.w_cutoff1 || walls2 <= rule.w_cutoff2 {
                        scratch[idx] = wall;
                    } else {
                        scratch[idx] = ground;
                    }
                }

                // Copy scratch back to grid.
                for p in bounds.iter() {
                    let idx = ((p.y - bounds.min.y) * w + (p.x - bounds.min.x)) as usize;
                    self.grid.set(p, scratch[idx]);
                }
            }
        }

        // Count ground cells.
        self.grid.count(ground)
    }

    /// Count wall cells within Chebyshev distance `radius` of `center`.
    fn count_walls_ring(
        &self,
        center: Point,
        radius: i32,
        wall: Cell,
        walls_out_of_range: bool,
    ) -> i32 {
        let mut count = 0;
        for dy in -radius..=radius {
            for dx in -radius..=radius {
                if dx == 0 && dy == 0 {
                    continue;
                }
                let p = Point::new(center.x + dx, center.y + dy);
                match self.grid.at(p) {
                    Some(c) => {
                        if c == wall {
                            count += 1;
                        }
                    }
                    None => {
                        if walls_out_of_range {
                            count += 1;
                        }
                    }
                }
            }
        }
        count
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_random_walk_carves_cells() {
        let grid = Grid::new(20, 20);
        grid.fill(Cell(1)); // all walls
        let mut mg = MapGen::with_grid(grid, rand::rng());
        let carved = mg.random_walk_cave(&FourDirectionWalker, Cell(0), 0.4, 10);
        assert!(carved > 0);
        let ground_count = mg.grid.count(Cell(0));
        assert!(ground_count >= carved);
    }

    #[test]
    fn test_cellular_automata_produces_mixed() {
        let grid = Grid::new(30, 30);
        let mut mg = MapGen::with_grid(grid, rand::rng());
        let rules = vec![CellularAutomataRule::default()];
        let ground = mg.cellular_automata_cave(Cell(1), Cell(0), 0.45, &rules);
        // Should have some ground and some wall.
        assert!(ground > 0);
        let total = 30 * 30;
        assert!(ground < total);
    }
}
