//! Jump Point Search (JPS) on uniform-cost grids.
//!
//! JPS is an optimised A* variant for grids where every passable step has
//! the same cost.  It "jumps" along straight lines, only adding nodes to the
//! open list at *jump points* — positions with forced neighbours.

use std::collections::BinaryHeap;

use gruid_core::Point;

use crate::PathRange;
use crate::distance;
use crate::pathrange::NodeRef;

impl PathRange {
    /// Compute a shortest path from `from` to `to` using Jump Point Search.
    ///
    /// `passable` returns `true` for walkable positions.  If `diags` is
    /// `true`, diagonal movement is allowed (8-way); otherwise only cardinal
    /// movement (4-way) is used.
    ///
    /// Returns the full path (including endpoints) or `None` if unreachable.
    pub fn jps_path(
        &mut self,
        from: Point,
        to: Point,
        passable: impl Fn(Point) -> bool,
        diags: bool,
    ) -> Option<Vec<Point>> {
        let start_idx = self.idx(from)?;
        let goal_idx = self.idx(to)?;

        if !passable(from) || !passable(to) {
            return None;
        }
        if start_idx == goal_idx {
            return Some(vec![from]);
        }

        self.astar_generation = self.astar_generation.wrapping_add(1);
        let cur_gen = self.astar_generation;

        {
            let n = &mut self.astar_nodes[start_idx];
            n.g = 0;
            n.f = Self::jps_heuristic(from, to, diags);
            n.parent = usize::MAX;
            n.generation = cur_gen;
            n.open = true;
        }

        let mut open: BinaryHeap<NodeRef> = BinaryHeap::new();
        open.push(NodeRef {
            idx: start_idx,
            f: self.astar_nodes[start_idx].f,
        });

        let found = 'search: loop {
            let Some(cur) = open.pop() else {
                break 'search false;
            };
            let ci = cur.idx;
            if self.astar_nodes[ci].generation != cur_gen || !self.astar_nodes[ci].open {
                continue;
            }
            if ci == goal_idx {
                break 'search true;
            }
            self.astar_nodes[ci].open = false;

            let cp = self.point(ci);
            let cur_g = self.astar_nodes[ci].g;

            // Determine successors: initial expand from start has no parent direction
            let dirs = if self.astar_nodes[ci].parent == usize::MAX {
                Self::all_dirs(diags)
            } else {
                let pp = self.point(self.astar_nodes[ci].parent);
                self.jps_prune_dirs(cp, pp, &passable, diags)
            };

            for dir in dirs {
                if let Some((jp, dist)) = self.jps_jump(cp, dir, to, &passable, diags) {
                    let Some(ji) = self.idx(jp) else {
                        continue;
                    };
                    let tentative_g = cur_g + dist;
                    let jn = &mut self.astar_nodes[ji];
                    if jn.generation == cur_gen && tentative_g >= jn.g {
                        continue;
                    }
                    jn.generation = cur_gen;
                    jn.g = tentative_g;
                    jn.f = tentative_g + Self::jps_heuristic(jp, to, diags);
                    jn.parent = ci;
                    jn.open = true;
                    open.push(NodeRef { idx: ji, f: jn.f });
                }
            }
        };

        if !found {
            return None;
        }

        // Reconstruct jump-point path, then interpolate to get a step-by-step path.
        let mut jp_path = Vec::new();
        let mut ci = goal_idx;
        while ci != usize::MAX {
            jp_path.push(self.point(ci));
            ci = self.astar_nodes[ci].parent;
        }
        jp_path.reverse();

        Some(Self::interpolate_path(&jp_path))
    }

    // -----------------------------------------------------------------------
    // JPS internals
    // -----------------------------------------------------------------------

    fn jps_heuristic(a: Point, b: Point, diags: bool) -> i32 {
        if diags {
            // Octile distance with uniform cost 1
            let dx = (a.x - b.x).abs();
            let dy = (a.y - b.y).abs();
            dx.max(dy)
        } else {
            distance::manhattan(a, b)
        }
    }

    fn all_dirs(diags: bool) -> Vec<Point> {
        if diags {
            vec![
                Point::new(1, 0),
                Point::new(-1, 0),
                Point::new(0, 1),
                Point::new(0, -1),
                Point::new(1, 1),
                Point::new(1, -1),
                Point::new(-1, 1),
                Point::new(-1, -1),
            ]
        } else {
            vec![
                Point::new(1, 0),
                Point::new(-1, 0),
                Point::new(0, 1),
                Point::new(0, -1),
            ]
        }
    }

    /// Pruned direction set for JPS.
    fn jps_prune_dirs(
        &self,
        p: Point,
        parent: Point,
        passable: &impl Fn(Point) -> bool,
        diags: bool,
    ) -> Vec<Point> {
        let mut dirs = Vec::with_capacity(8);
        let d = Point::new((p.x - parent.x).signum(), (p.y - parent.y).signum());

        if diags {
            if d.x != 0 && d.y != 0 {
                // Diagonal move: natural neighbours
                if passable(p + Point::new(0, d.y)) {
                    dirs.push(Point::new(0, d.y));
                }
                if passable(p + Point::new(d.x, 0)) {
                    dirs.push(Point::new(d.x, 0));
                }
                if passable(p + Point::new(d.x, d.y)) {
                    dirs.push(Point::new(d.x, d.y));
                }
                // Forced neighbours
                if !passable(p + Point::new(-d.x, 0)) && passable(p + Point::new(-d.x, d.y)) {
                    dirs.push(Point::new(-d.x, d.y));
                }
                if !passable(p + Point::new(0, -d.y)) && passable(p + Point::new(d.x, -d.y)) {
                    dirs.push(Point::new(d.x, -d.y));
                }
            } else if d.x != 0 {
                // Horizontal
                if passable(p + Point::new(d.x, 0)) {
                    dirs.push(Point::new(d.x, 0));
                }
                if !passable(p + Point::new(0, 1)) && passable(p + Point::new(d.x, 1)) {
                    dirs.push(Point::new(d.x, 1));
                }
                if !passable(p + Point::new(0, -1)) && passable(p + Point::new(d.x, -1)) {
                    dirs.push(Point::new(d.x, -1));
                }
            } else {
                // Vertical
                if passable(p + Point::new(0, d.y)) {
                    dirs.push(Point::new(0, d.y));
                }
                if !passable(p + Point::new(1, 0)) && passable(p + Point::new(1, d.y)) {
                    dirs.push(Point::new(1, d.y));
                }
                if !passable(p + Point::new(-1, 0)) && passable(p + Point::new(-1, d.y)) {
                    dirs.push(Point::new(-1, d.y));
                }
            }
        } else {
            // 4-way: keep the natural direction and any forced neighbours.
            if d.x != 0 {
                if passable(p + Point::new(d.x, 0)) {
                    dirs.push(Point::new(d.x, 0));
                }
                if !passable(p + Point::new(0, 1)) && passable(p + Point::new(d.x, 1)) {
                    dirs.push(Point::new(0, 1));
                    dirs.push(Point::new(d.x, 1));
                }
                if !passable(p + Point::new(0, -1)) && passable(p + Point::new(d.x, -1)) {
                    dirs.push(Point::new(0, -1));
                    dirs.push(Point::new(d.x, -1));
                }
            } else if d.y != 0 {
                if passable(p + Point::new(0, d.y)) {
                    dirs.push(Point::new(0, d.y));
                }
                if !passable(p + Point::new(1, 0)) && passable(p + Point::new(1, d.y)) {
                    dirs.push(Point::new(1, 0));
                    dirs.push(Point::new(1, d.y));
                }
                if !passable(p + Point::new(-1, 0)) && passable(p + Point::new(-1, d.y)) {
                    dirs.push(Point::new(-1, 0));
                    dirs.push(Point::new(-1, d.y));
                }
            }
        }
        dirs
    }

    /// Jump along `dir` from `p` until we find a jump point or fail.
    /// Returns `(jump_point, distance)` if found.
    fn jps_jump(
        &self,
        p: Point,
        dir: Point,
        goal: Point,
        passable: &impl Fn(Point) -> bool,
        diags: bool,
    ) -> Option<(Point, i32)> {
        let mut n = p + dir;
        let mut dist = 1;

        loop {
            if !self.rng.contains(n) || !passable(n) {
                return None;
            }
            if n == goal {
                return Some((n, dist));
            }

            // Check for forced neighbours.
            if diags && dir.x != 0 && dir.y != 0 {
                // Diagonal: forced if blocked beside
                if (!passable(n + Point::new(-dir.x, 0)) && passable(n + Point::new(-dir.x, dir.y)))
                    || (!passable(n + Point::new(0, -dir.y))
                        && passable(n + Point::new(dir.x, -dir.y)))
                {
                    return Some((n, dist));
                }
                // Recurse along component axes.
                if self
                    .jps_jump(n, Point::new(dir.x, 0), goal, passable, diags)
                    .is_some()
                    || self
                        .jps_jump(n, Point::new(0, dir.y), goal, passable, diags)
                        .is_some()
                {
                    return Some((n, dist));
                }
            } else if dir.x != 0 {
                // Horizontal
                let has_forced = if diags {
                    (!passable(n + Point::new(0, 1)) && passable(n + Point::new(dir.x, 1)))
                        || (!passable(n + Point::new(0, -1)) && passable(n + Point::new(dir.x, -1)))
                } else {
                    (!passable(n + Point::new(0, 1)) && passable(n + Point::new(0, 1)))
                        || (!passable(n + Point::new(0, -1)) && passable(n + Point::new(0, -1)))
                };
                if has_forced {
                    return Some((n, dist));
                }
            } else {
                // Vertical
                let has_forced = if diags {
                    (!passable(n + Point::new(1, 0)) && passable(n + Point::new(1, dir.y)))
                        || (!passable(n + Point::new(-1, 0)) && passable(n + Point::new(-1, dir.y)))
                } else {
                    (!passable(n + Point::new(1, 0)) && passable(n + Point::new(1, 0)))
                        || (!passable(n + Point::new(-1, 0)) && passable(n + Point::new(-1, 0)))
                };
                if has_forced {
                    return Some((n, dist));
                }
            }

            n = n + dir;
            dist += 1;
        }
    }

    /// Expand jump-point path into a step-by-step path.
    fn interpolate_path(jp_path: &[Point]) -> Vec<Point> {
        if jp_path.len() <= 1 {
            return jp_path.to_vec();
        }
        let mut result = Vec::new();
        for window in jp_path.windows(2) {
            let a = window[0];
            let b = window[1];
            let dx = (b.x - a.x).signum();
            let dy = (b.y - a.y).signum();
            let mut c = a;
            while c != b {
                result.push(c);
                c = c + Point::new(dx, dy);
                // Handle non-diagonal segments of a diagonal jump
                // by doing diagonal first, then straight.
                if c.x == b.x {
                    // switch to vertical-only if we still need to go
                    while c != b {
                        result.push(c);
                        c = c + Point::new(0, (b.y - c.y).signum());
                    }
                    break;
                }
                if c.y == b.y {
                    while c != b {
                        result.push(c);
                        c = c + Point::new((b.x - c.x).signum(), 0);
                    }
                    break;
                }
            }
        }
        result.push(*jp_path.last().unwrap());
        result
    }
}
