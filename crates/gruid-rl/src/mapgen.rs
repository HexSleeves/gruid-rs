//! Map generation algorithms for roguelike games.
//!
//! Provides two cave generators:
//! - **Random Walk Cave**: uses a drunk-walk approach to carve open space.
//! - **Cellular Automata Cave**: initializes random walls then smooths
//!   with cellular automata rules.

use crate::grid::{Cell, Grid};
use gruid_core::Point;
use gruid_paths::PathRange;
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
    /// it becomes a wall. Set to `0` to disable this check.
    pub w_cutoff1: i32,
    /// If a cell has <= this many wall neighbors in the 2-ring
    /// (24 neighbors), it becomes a wall. Set to `>= 25` to disable
    /// this check (the 2-ring has at most 24 cells).
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

    /// Keep only the connected component reachable from `p`, filling
    /// everything else with `wall`.
    ///
    /// Uses the last `cc_map_all` (or `cc_map`) results from `pr`.
    /// Paths are assumed to be bidirectional.
    ///
    /// Returns the number of cells in the surviving connected component,
    /// or 0 if `p` is not reachable.
    pub fn keep_connected(&self, pr: &PathRange, p: Point, wall: Cell) -> usize {
        let id = match pr.cc_at(p) {
            Some(id) => id,
            None => {
                self.grid.fill(wall);
                return 0;
            }
        };
        let mut count = 0;
        let sz = self.grid.size();
        for y in 0..sz.y {
            for x in 0..sz.x {
                let q = Point::new(x, y);
                if pr.cc_at(q) != Some(id) {
                    self.grid.set(q, wall);
                } else {
                    count += 1;
                }
            }
        }
        count
    }

    /// Generate a cave using random walk.
    ///
    /// Performs `walks` random walks starting from random positions.
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
        let fill_pct = fill_pct.clamp(0.01, 0.9);
        let sz = self.grid.size();
        let w = sz.x;
        let h = sz.y;
        let total = (w * h) as usize;
        let target = (total as f64 * fill_pct) as usize;
        let already_dug = self.grid.count(cell);
        let mut digs = already_dug;
        let wlk_max = if walks > 0 {
            (target - already_dug) / walks
        } else {
            target - already_dug
        };

        while digs < target {
            // Start each walk from a random position (matching Go).
            let mut pos = Point::new(
                self.rng.random_range(0..w),
                self.rng.random_range(0..h),
            );
            if self.grid.at(pos) == Some(cell) {
                continue;
            }
            self.grid.set(pos, cell);
            digs += 1;
            let mut wlk_digs = 1;
            let mut out_digs = 0;
            let mut last_in_range = pos;

            while digs < target && wlk_digs <= wlk_max {
                let q = walker.neighbor(pos, &mut self.rng);
                // If current pos is out of range but next is in range and
                // not yet dug, snap back to last known good position.
                if !self.grid.contains(pos) && self.grid.contains(q) {
                    if self.grid.at(q) != Some(cell) {
                        pos = last_in_range;
                        continue;
                    }
                }
                pos = q;
                if self.grid.contains(pos) {
                    if self.grid.at(pos) != Some(cell) {
                        self.grid.set(pos, cell);
                        digs += 1;
                        wlk_digs += 1;
                    }
                    last_in_range = pos;
                } else {
                    out_digs += 1;
                }
                if out_digs > wlk_max || out_digs > 150 {
                    out_digs = 0;
                    pos = last_in_range;
                }
            }
        }

        digs - already_dug
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
        let wall_init_pct = wall_init_pct.clamp(0.1, 0.9);
        let sz = self.grid.size();
        let w = sz.x;
        let h = sz.y;

        // Step 1: random initialization (using relative coords).
        for y in 0..h {
            for x in 0..w {
                let r: f64 = self.rng.random();
                let c = if r < wall_init_pct { wall } else { ground };
                self.grid.set(Point::new(x, y), c);
            }
        }

        // Step 2: apply rules.
        let mut scratch = vec![Cell::default(); (w * h) as usize];

        for rule in rules {
            let use_w1 = rule.w_cutoff1 > 0;
            let use_w2 = rule.w_cutoff2 < 25;

            for _ in 0..rule.reps {
                for y in 0..h {
                    for x in 0..w {
                        let p = Point::new(x, y);
                        let idx = (y * w + x) as usize;

                        let is_wall = match (use_w1, use_w2) {
                            (true, true) => {
                                let w1 =
                                    self.count_walls(p, 1, wall, rule.walls_out_of_range);
                                let w2 =
                                    self.count_walls(p, 2, wall, rule.walls_out_of_range);
                                w1 >= rule.w_cutoff1 || w2 <= rule.w_cutoff2
                            }
                            (true, false) => {
                                let w1 =
                                    self.count_walls(p, 1, wall, rule.walls_out_of_range);
                                w1 >= rule.w_cutoff1
                            }
                            (false, true) => {
                                let w2 =
                                    self.count_walls(p, 2, wall, rule.walls_out_of_range);
                                w2 <= rule.w_cutoff2
                            }
                            (false, false) => false,
                        };

                        scratch[idx] = if is_wall { wall } else { ground };
                    }
                }

                // Copy scratch back to grid.
                for y in 0..h {
                    for x in 0..w {
                        let idx = (y * w + x) as usize;
                        self.grid.set(Point::new(x, y), scratch[idx]);
                    }
                }
            }
        }

        self.grid.count(ground)
    }

    /// Count wall cells within Chebyshev distance `radius` of `center`.
    /// Matches Go: includes the center cell itself in the count.
    fn count_walls(
        &self,
        center: Point,
        radius: i32,
        wall: Cell,
        walls_out_of_range: bool,
    ) -> i32 {
        let mut count = 0;
        let rg = gruid_core::Range::new(
            center.x - radius,
            center.y - radius,
            center.x + radius + 1,
            center.y + radius + 1,
        );
        let grid_rg = self.grid.range_();

        if walls_out_of_range {
            let orig_size = rg.size();
            let clamped = rg.intersect(grid_rg);
            let clamped_size = clamped.size();
            // Out-of-range cells count as walls.
            count += orig_size.x * orig_size.y - clamped_size.x * clamped_size.y;
        }

        // Count in-range walls using a slice of the grid.
        let clamped = rg.intersect(grid_rg);
        let sub = self.grid.slice(clamped);
        count += sub.count(wall) as i32;
        count
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gruid_paths::Pather;

    /// Simple pather for testing: treats Cell(0) as passable with 4-directional movement.
    struct FloorPather<'a> {
        grid: &'a Grid,
    }

    impl Pather for FloorPather<'_> {
        fn neighbors(&self, p: Point, buf: &mut Vec<Point>) {
            for &d in &[
                Point::new(1, 0),
                Point::new(-1, 0),
                Point::new(0, 1),
                Point::new(0, -1),
            ] {
                let np = Point::new(p.x + d.x, p.y + d.y);
                if self.grid.at(np) == Some(Cell(0)) {
                    buf.push(np);
                }
            }
        }
    }

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
        assert!(ground > 0);
        let total = 30 * 30;
        assert!(ground < total);
    }

    #[test]
    fn test_count_walls_includes_center() {
        // A 3x3 grid of all walls. countWalls at center with radius 1
        // should count all 9 cells (including center).
        let grid = Grid::new(3, 3);
        grid.fill(Cell(1));
        let mg = MapGen::with_grid(grid, rand::rng());
        let count = mg.count_walls(Point::new(1, 1), 1, Cell(1), false);
        assert_eq!(count, 9); // 3x3 = 9 including center
    }

    #[test]
    fn test_keep_connected() {
        // 5x5 grid with two disconnected floor regions:
        //   00100
        //   00100
        //   11111  (wall row)
        //   00100
        //   00100
        let grid = Grid::new(5, 5);
        grid.fill(Cell(0)); // all floor
        // Create a wall cross that separates corners
        for i in 0..5 {
            grid.set(Point::new(2, i), Cell(1)); // vertical wall
            grid.set(Point::new(i, 2), Cell(1)); // horizontal wall
        }
        // Now we have 4 disconnected 2x2 floor regions
        // Top-left: (0,0),(1,0),(0,1),(1,1)
        // Top-right: (3,0),(4,0),(3,1),(4,1)
        // etc.

        let rng = gruid_core::Range::new(0, 0, 5, 5);
        let mut pr = PathRange::new(rng);
        let pather = FloorPather { grid: &grid };
        pr.cc_map_all(&pather);

        let mg = MapGen::with_grid(grid, rand::rng());
        let kept = mg.keep_connected(&pr, Point::new(0, 0), Cell(1));
        assert_eq!(kept, 4); // top-left 2x2 region

        // All other floor regions should now be walls
        assert_eq!(mg.grid.at(Point::new(3, 0)), Some(Cell(1)));
        assert_eq!(mg.grid.at(Point::new(0, 3)), Some(Cell(1)));
        assert_eq!(mg.grid.at(Point::new(3, 3)), Some(Cell(1)));

        // The kept region should still be floor
        assert_eq!(mg.grid.at(Point::new(0, 0)), Some(Cell(0)));
        assert_eq!(mg.grid.at(Point::new(1, 1)), Some(Cell(0)));
    }
}
