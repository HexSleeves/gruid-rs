//! Field of Vision algorithms.
//!
//! Provides two FOV algorithms:
//! - **Ray-based** (`vision_map`): casts rays outward from a source through
//!   octants, accumulating costs via a user-provided [`Lighter`] trait.
//! - **Symmetric Shadow Casting** (`ssc_vision_map`): implements the iterative
//!   version of the algorithm described at
//!   <https://www.albertford.com/shadowcasting/>.

use gruid_core::{Point, Range};

/// A node that has been lit by the FOV computation, with its accumulated cost.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LightNode {
    pub pos: Point,
    pub cost: i32,
}

/// Trait for providing the cost of light passing through a cell.
pub trait Lighter {
    /// Return the cost of seeing from `from` into `to`.
    ///
    /// A return value of `i32::MAX` means the cell blocks vision entirely.
    fn cost(&self, from: Point, to: Point) -> i32;
}

/// Octant transformation: maps (row, col) in abstract octant space back
/// to real (dx, dy) offsets relative to the source.
struct Octant {
    xx: i32,
    xy: i32,
    yx: i32,
    yy: i32,
}

const OCTANTS: [Octant; 8] = [
    Octant {
        xx: 1,
        xy: 0,
        yx: 0,
        yy: 1,
    },
    Octant {
        xx: 0,
        xy: 1,
        yx: 1,
        yy: 0,
    },
    Octant {
        xx: 0,
        xy: -1,
        yx: 1,
        yy: 0,
    },
    Octant {
        xx: -1,
        xy: 0,
        yx: 0,
        yy: 1,
    },
    Octant {
        xx: -1,
        xy: 0,
        yx: 0,
        yy: -1,
    },
    Octant {
        xx: 0,
        xy: -1,
        yx: -1,
        yy: 0,
    },
    Octant {
        xx: 0,
        xy: 1,
        yx: -1,
        yy: 0,
    },
    Octant {
        xx: 1,
        xy: 0,
        yx: 0,
        yy: -1,
    },
];

impl Octant {
    fn transform(&self, row: i32, col: i32) -> (i32, i32) {
        (row * self.xx + col * self.xy, row * self.yx + col * self.yy)
    }
}

/// Fractional value used for shadow boundaries, represented as numerator/denominator.
#[derive(Debug, Clone, Copy)]
struct Fraction {
    num: i32,
    den: i32,
}

impl Fraction {
    fn new(num: i32, den: i32) -> Self {
        Self { num, den }
    }
}

/// A pending scan row for the iterative shadow casting.
#[derive(Debug, Clone, Copy)]
struct SscRow {
    depth: i32,
    start_slope: Fraction,
    end_slope: Fraction,
}

impl SscRow {
    fn tiles(&self) -> (i32, i32) {
        // min_col = round_ties_up(depth * start_slope)
        // max_col = round_ties_down(depth * end_slope)
        let depth = self.depth;
        let min_col = {
            // (depth * start_slope.num + start_slope.den/2) / start_slope.den
            // We use: floor((2*depth*num + den) / (2*den))
            let n = 2 * depth * self.start_slope.num + self.start_slope.den;
            let d = 2 * self.start_slope.den;
            div_floor(n, d)
        };
        let max_col = {
            // round_ties_down: floor((2*depth*num - den) / (2*den)) + 1
            // Simplified: ceiling((2*depth*num - den + 1) / (2*den))
            let n = 2 * depth * self.end_slope.num - self.end_slope.den;
            let d = 2 * self.end_slope.den;
            // We want floor(n / d) but n can be negative
            div_floor(n, d)
        };
        (min_col, max_col)
    }

    fn next(self) -> SscRow {
        SscRow {
            depth: self.depth + 1,
            start_slope: self.start_slope,
            end_slope: self.end_slope,
        }
    }
}

fn div_floor(a: i32, b: i32) -> i32 {
    // Euclidean-style floor division
    let d = a / b;
    let r = a % b;
    if (r != 0) && ((r ^ b) < 0) { d - 1 } else { d }
}

/// Field of Vision computation.
pub struct FOV {
    /// The rectangular range this FOV operates within.
    range: Range,
    /// Cost map for ray-based FOV, indexed as (y - range.min.y) * w + (x - range.min.x).
    /// -1 means not visited.
    costs: Vec<i32>,
    /// Visibility map for SSC FOV.
    visible: Vec<bool>,
    /// Cached list of lighted nodes from the last `vision_map` call.
    lighted: Vec<LightNode>,
    /// Cached list of visible points from the last `ssc_vision_map` call.
    visible_points: Vec<Point>,
}

impl FOV {
    /// Create a new FOV for the given range.
    pub fn new(range: Range) -> Self {
        let w = range.width();
        let h = range.height();
        let size = (w * h) as usize;
        Self {
            range,
            costs: vec![-1; size],
            visible: vec![false; size],
            lighted: Vec::new(),
            visible_points: Vec::new(),
        }
    }

    /// Change the range and reset internal buffers.
    pub fn set_range(&mut self, range: Range) {
        let w = range.width();
        let h = range.height();
        let size = (w * h) as usize;
        self.range = range;
        self.costs.resize(size, -1);
        self.visible.resize(size, false);
        self.reset_costs();
        self.reset_visible();
    }

    fn idx(&self, p: Point) -> Option<usize> {
        if !self.range.contains(p) {
            return None;
        }
        let w = self.range.width();
        Some(((p.y - self.range.min.y) * w + (p.x - self.range.min.x)) as usize)
    }

    fn reset_costs(&mut self) {
        for c in &mut self.costs {
            *c = -1;
        }
    }

    fn reset_visible(&mut self) {
        for v in &mut self.visible {
            *v = false;
        }
    }

    // ── Ray-based FOV ──────────────────────────────────────────────

    /// Compute a ray-based field of vision from `source`.
    ///
    /// Rays are cast outward through each octant. The accumulated cost
    /// to reach each cell is stored; cells whose cost exceeds `max_cost`
    /// are not included. Use [`at`](Self::at) or [`iter_lighted`](Self::iter_lighted)
    /// to query results.
    pub fn vision_map(&mut self, lighter: &impl Lighter, source: Point, max_cost: i32) {
        self.reset_costs();
        self.lighted.clear();

        // Mark the source.
        if let Some(idx) = self.idx(source) {
            self.costs[idx] = 0;
            self.lighted.push(LightNode {
                pos: source,
                cost: 0,
            });
        } else {
            return;
        }

        // Maximum extent in any direction.
        let max_radius = self.range.width().max(self.range.height());

        for oct in &OCTANTS {
            self.cast_ray_octant(lighter, source, max_cost, max_radius, oct);
        }
    }

    fn cast_ray_octant(
        &mut self,
        lighter: &impl Lighter,
        source: Point,
        max_cost: i32,
        max_radius: i32,
        oct: &Octant,
    ) {
        // Cast rays along the leading edge of the octant.
        // For each "column" in the octant we send a ray outward.
        // The octant covers depth 1..max_radius, col 0..=depth.
        //
        // We process each cell (depth, col) in order and propagate
        // the minimum accumulated cost from the previous depth.
        //
        // For simplicity we iterate depth-first and track costs
        // from the previous depth row.

        // We use a simple approach: for each cell at (depth, col)
        // compute cost as min over relevant parents + lighter cost.
        // Parents are cells at (depth-1, col-1), (depth-1, col).
        // We store per-row costs in a temporary Vec.

        let mut prev_row_costs: Vec<i32> = vec![0]; // depth=0 has only source, cost=0

        for depth in 1..=max_radius {
            let mut cur_row_costs: Vec<i32> = Vec::with_capacity((depth + 1) as usize);
            let mut any_visible = false;

            for col in 0..=depth {
                let (dx, dy) = oct.transform(depth, col);
                let p = Point::new(source.x + dx, source.y + dy);
                if !self.range.contains(p) {
                    cur_row_costs.push(i32::MAX);
                    continue;
                }

                // Find the minimum parent cost.
                let mut parent_cost = i32::MAX;

                // Parent at (depth-1, col) — straight ahead
                if col < prev_row_costs.len() as i32 {
                    let pc = prev_row_costs[col as usize];
                    if pc < parent_cost {
                        parent_cost = pc;
                    }
                }
                // Parent at (depth-1, col-1) — diagonal
                if col > 0 && (col - 1) < prev_row_costs.len() as i32 {
                    let pc = prev_row_costs[(col - 1) as usize];
                    if pc < parent_cost {
                        parent_cost = pc;
                    }
                }

                if parent_cost == i32::MAX {
                    cur_row_costs.push(i32::MAX);
                    continue;
                }

                // Compute the parent point (use the one with minimum cost).
                let parent_col = if col > 0
                    && (col - 1) < prev_row_costs.len() as i32
                    && prev_row_costs[(col - 1) as usize] <= parent_cost
                {
                    col - 1
                } else {
                    col.min(prev_row_costs.len() as i32 - 1)
                };
                let (pdx, pdy) = oct.transform(depth - 1, parent_col);
                let parent_p = Point::new(source.x + pdx, source.y + pdy);

                let step_cost = lighter.cost(parent_p, p);
                let total = if step_cost == i32::MAX || parent_cost == i32::MAX {
                    i32::MAX
                } else {
                    parent_cost.saturating_add(step_cost)
                };

                if total <= max_cost {
                    // Record in our map (keep minimum if already set from another octant).
                    if let Some(idx) = self.idx(p) {
                        if self.costs[idx] < 0 || total < self.costs[idx] {
                            let was_new = self.costs[idx] < 0;
                            self.costs[idx] = total;
                            if was_new {
                                self.lighted.push(LightNode {
                                    pos: p,
                                    cost: total,
                                });
                            } else {
                                // Update cost in lighted list.
                                if let Some(node) = self.lighted.iter_mut().find(|n| n.pos == p) {
                                    node.cost = total;
                                }
                            }
                        }
                    }
                    cur_row_costs.push(total);
                    any_visible = true;
                } else {
                    cur_row_costs.push(i32::MAX);
                }
            }

            prev_row_costs = cur_row_costs;
            if !any_visible {
                break;
            }
        }
    }

    /// Query the accumulated cost to reach `p` from the last `vision_map` call.
    ///
    /// Returns `None` if the point was not reached or is out of range.
    pub fn at(&self, p: Point) -> Option<i32> {
        let idx = self.idx(p)?;
        let c = self.costs[idx];
        if c < 0 { None } else { Some(c) }
    }

    /// Iterate over all lighted nodes from the last `vision_map` call.
    pub fn iter_lighted(&self) -> impl Iterator<Item = LightNode> + '_ {
        self.lighted.iter().copied()
    }

    // ── Symmetric Shadow Casting ───────────────────────────────────

    /// Compute field of vision using symmetric shadow casting.
    ///
    /// `passable` returns `true` if the given point does not block vision.
    /// All cells within Manhattan/Chebyshev distance `max_range` of `source`
    /// that are visible will be recorded.
    ///
    /// Based on the algorithm described at
    /// <https://www.albertford.com/shadowcasting/>, adapted from recursive
    /// to iterative with an explicit stack.
    pub fn ssc_vision_map(
        &mut self,
        source: Point,
        max_range: i32,
        passable: impl Fn(Point) -> bool,
    ) {
        self.reset_visible();
        self.visible_points.clear();

        // Mark source visible.
        if let Some(idx) = self.idx(source) {
            self.visible[idx] = true;
            self.visible_points.push(source);
        } else {
            return;
        }

        for oct in &OCTANTS {
            self.ssc_scan_octant(source, max_range, &passable, oct);
        }
    }

    fn ssc_scan_octant(
        &mut self,
        source: Point,
        max_range: i32,
        passable: &impl Fn(Point) -> bool,
        oct: &Octant,
    ) {
        let mut stack: Vec<SscRow> = Vec::new();
        stack.push(SscRow {
            depth: 1,
            start_slope: Fraction::new(-1, 1),
            end_slope: Fraction::new(1, 1),
        });

        while let Some(row) = stack.pop() {
            if row.depth > max_range {
                continue;
            }

            let (min_col, max_col) = row.tiles();
            let mut prev_was_wall: Option<bool> = None;
            let mut next_start_slope = row.start_slope;

            for col in min_col..=max_col {
                let (dx, dy) = oct.transform(row.depth, col);
                let p = Point::new(source.x + dx, source.y + dy);

                if !self.range.contains(p) {
                    // Treat out-of-range as wall.
                    prev_was_wall = Some(true);
                    continue;
                }

                let is_wall = !passable(p);
                // A tile is symmetric if its center column is within the
                // sector [start_slope, end_slope] at this depth.
                // col >= depth * start_slope  AND  col <= depth * end_slope
                // Using fractions: col/1 >= start.num/start.den
                //   => col * start.den >= start.num * depth  ... but start
                //      is already a slope (rise/run), so we compare:
                //   col * 2 + 1 > start_slope * depth * 2  (round-up for start)
                //   col * 2 - 1 < end_slope * depth * 2    (round-down for end)
                let is_symmetric = {
                    // col >= depth * start_slope
                    // (2*col+1) * start.den > 2 * depth * start.num
                    let start_ok =
                        (2 * col + 1) * row.start_slope.den > 2 * row.depth * row.start_slope.num;
                    // col <= depth * end_slope
                    // (2*col-1) * end.den < 2 * depth * end.num
                    let end_ok =
                        (2 * col - 1) * row.end_slope.den < 2 * row.depth * row.end_slope.num;
                    start_ok && end_ok
                };

                // Reveal cell if it's a floor or if it's symmetric (allows
                // seeing walls adjacent to visible floor).
                if !is_wall || is_symmetric {
                    self.mark_visible(p);
                }

                if let Some(prev_wall) = prev_was_wall {
                    if prev_wall && !is_wall {
                        // Transition from wall to floor: narrow start slope.
                        next_start_slope = Fraction::new(2 * col - 1, 2 * row.depth);
                    }
                    if !prev_wall && is_wall {
                        // Transition from floor to wall: push a child row
                        // covering the floor segment we just passed.
                        let mut child = row.next();
                        child.start_slope = next_start_slope;
                        child.end_slope = Fraction::new(2 * col - 1, 2 * row.depth);
                        stack.push(child);
                    }
                }

                prev_was_wall = Some(is_wall);
            }

            // If the last cell in the row was floor, continue scanning.
            if prev_was_wall == Some(false) {
                let mut child = row.next();
                child.start_slope = next_start_slope;
                stack.push(child);
            }
        }
    }

    fn mark_visible(&mut self, p: Point) {
        if let Some(idx) = self.idx(p) {
            if !self.visible[idx] {
                self.visible[idx] = true;
                self.visible_points.push(p);
            }
        }
    }

    /// Query whether `p` is visible from the last `ssc_vision_map` call.
    pub fn visible(&self, p: Point) -> bool {
        match self.idx(p) {
            Some(idx) => self.visible[idx],
            None => false,
        }
    }

    /// Iterate over all visible points from the last `ssc_vision_map` call.
    pub fn iter_visible(&self) -> impl Iterator<Item = Point> + '_ {
        self.visible_points.iter().copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct SimpleWalls {
        walls: Vec<Point>,
    }

    impl Lighter for SimpleWalls {
        fn cost(&self, _from: Point, to: Point) -> i32 {
            if self.walls.contains(&to) {
                i32::MAX
            } else {
                1
            }
        }
    }

    #[test]
    fn test_vision_map_open() {
        let range = Range::new(0, 0, 10, 10);
        let mut fov = FOV::new(range);
        let lighter = SimpleWalls { walls: vec![] };
        fov.vision_map(&lighter, Point::new(5, 5), 3);

        // Source should have cost 0.
        assert_eq!(fov.at(Point::new(5, 5)), Some(0));
        // Adjacent cells should have cost 1.
        assert_eq!(fov.at(Point::new(6, 5)), Some(1));
        // Far away cell should not be reached.
        assert_eq!(fov.at(Point::new(0, 0)), None);
    }

    #[test]
    fn test_vision_map_wall() {
        let range = Range::new(0, 0, 10, 10);
        let mut fov = FOV::new(range);
        let lighter = SimpleWalls {
            walls: vec![Point::new(6, 5)],
        };
        fov.vision_map(&lighter, Point::new(5, 5), 10);

        // Source visible.
        assert_eq!(fov.at(Point::new(5, 5)), Some(0));
        // Wall cell itself should NOT be reached (cost = MAX).
        assert_eq!(fov.at(Point::new(6, 5)), None);
    }

    #[test]
    fn test_ssc_open_field() {
        let range = Range::new(0, 0, 11, 11);
        let mut fov = FOV::new(range);
        let source = Point::new(5, 5);
        fov.ssc_vision_map(source, 3, |_| true);

        assert!(fov.visible(source));
        assert!(fov.visible(Point::new(6, 5)));
        assert!(fov.visible(Point::new(5, 6)));
        // Beyond range.
        assert!(!fov.visible(Point::new(0, 0)));
    }

    #[test]
    fn test_ssc_wall_blocks() {
        let range = Range::new(0, 0, 11, 11);
        let mut fov = FOV::new(range);
        let source = Point::new(5, 5);
        // Wall at (6, 5) — should block (7,5), (8,5), etc.
        let wall = Point::new(6, 5);
        fov.ssc_vision_map(source, 5, |p| p != wall);

        assert!(fov.visible(source));
        // The wall itself is visible (you can see a wall).
        assert!(fov.visible(wall));
        // Behind the wall should be blocked.
        assert!(!fov.visible(Point::new(7, 5)));
        assert!(!fov.visible(Point::new(8, 5)));
    }

    #[test]
    fn test_ssc_symmetry() {
        // Symmetric shadow casting: if A sees B, then B sees A.
        let range = Range::new(0, 0, 20, 20);
        let wall = Point::new(8, 10);
        let passable = |p: Point| p != wall;

        let a = Point::new(10, 10);
        let b = Point::new(6, 10);

        let mut fov = FOV::new(range);
        fov.ssc_vision_map(a, 10, passable);
        let a_sees_b = fov.visible(b);

        fov.ssc_vision_map(b, 10, passable);
        let b_sees_a = fov.visible(a);

        assert_eq!(a_sees_b, b_sees_a, "SSC should be symmetric");
    }
}
