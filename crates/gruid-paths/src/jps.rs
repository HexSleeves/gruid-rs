//! Jump Point Search (JPS) on uniform-cost grids.
//!
//! JPS is an optimised A* variant for grids where every passable step has
//! the same cost. It "jumps" along straight lines, only adding nodes to the
//! open list at *jump points* — positions with forced neighbours.
//!
//! Faithfully ported from Go gruid's `paths/jps.go`.

use std::collections::BinaryHeap;

use gruid_core::Point;

use crate::PathRange;
use crate::pathrange::NodeRef;

fn sign(n: i32) -> i32 {
    if n > 0 {
        1
    } else if n < 0 {
        -1
    } else {
        0
    }
}

/// Normalize direction between two points. Non-axis, non-diagonal directions
/// are projected onto the cardinal component (pruned intermediate diagonal nodes).
fn dirnorm(p: Point, q: Point) -> Point {
    let d = q - p;
    let dx = d.x.abs();
    let dy = d.y.abs();
    let mut r = Point::new(sign(d.x), sign(d.y));
    if dx != dy {
        if dx > dy {
            r.y = 0;
        } else {
            r.x = 0;
        }
    }
    r
}

fn right(p: Point, dir: Point) -> Point {
    Point::new(p.x - dir.y, p.y + dir.x)
}

fn left(p: Point, dir: Point) -> Point {
    Point::new(p.x + dir.y, p.y - dir.x)
}

#[derive(Clone, Copy, PartialEq)]
enum ForcedSucc {
    None,
    Left,
    Right,
    Both,
}

fn diag_cost(diags: bool) -> i32 {
    if diags { 1 } else { 2 }
}

impl PathRange {
    /// Compute a shortest path using Jump Point Search.
    ///
    /// Returns the full path (including endpoints) or `None` if unreachable.
    pub fn jps_path(
        &mut self,
        from: Point,
        to: Point,
        passable: impl Fn(Point) -> bool,
        diags: bool,
    ) -> Option<Vec<Point>> {
        if !self.rng.contains(from) || !self.rng.contains(to) {
            return None;
        }
        if from == to {
            return Some(vec![from]);
        }

        self.astar_generation = self.astar_generation.wrapping_add(1);
        let cur_gen = self.astar_generation;

        // Mark start as closed.
        let si = self.idx(from)?;
        {
            let n = &mut self.astar_nodes[si];
            n.g = 0;
            n.f = 0;
            n.parent = usize::MAX;
            n.generation = cur_gen;
            n.open = false;
        }

        let mut open: BinaryHeap<NodeRef> = BinaryHeap::new();

        // Expand origin.
        for y in -1..=1i32 {
            for x in -1..=1i32 {
                if x == 0 && y == 0 {
                    continue;
                }
                let dir = Point::new(x, y);
                let q = from + dir;
                if !diags && dir.x != 0 && dir.y != 0 {
                    if self.jps_pass(from + Point::new(dir.x, 0), &passable)
                        || self.jps_pass(from + Point::new(0, dir.y), &passable)
                    {
                        self.jps_add(q, from, to, 2, diags, cur_gen, &mut open, &passable);
                    }
                    continue;
                }
                let c = if dir.x != 0 && dir.y != 0 { 1 } else { 1 };
                self.jps_add(q, from, to, c, diags, cur_gen, &mut open, &passable);
            }
        }

        loop {
            let cur = open.pop()?;
            let ci = cur.idx;
            let nd = &self.astar_nodes[ci];
            if nd.generation != cur_gen || !nd.open {
                continue;
            }
            let cp = self.point(ci);
            let cur_g = nd.g;
            let parent_idx = nd.parent;
            self.astar_nodes[ci].open = false;

            if cp == to {
                return Some(self.jps_reconstruct(from, ci, &passable, diags, cur_gen));
            }

            let parent_p = self.point(parent_idx);

            // Natural neighbors + forced neighbors.
            let (nats, forced) =
                self.jps_get_neighbors(cp, parent_p, to, cur_g, &passable, diags);

            // Add forced neighbors.
            for (fp, fc) in forced {
                self.jps_add(fp, cp, to, cur_g + fc, diags, cur_gen, &mut open, &passable);
            }

            // Jump from natural neighbors.
            for np in nats {
                let dir = np - cp;
                let (q, i) = if diags {
                    self.jps_jump(np, dir, to, cur_g, &passable, cp, cur_gen, &mut open)
                } else {
                    self.jps_jump_nd(np, dir, to, cur_g, &passable, cp, cur_gen, &mut open)
                };
                if i > 0 {
                    self.jps_add(q, cp, to, cur_g + i, diags, cur_gen, &mut open, &passable);
                }
            }
        }
    }

    // -- Helpers --

    fn jps_pass(&self, p: Point, passable: &impl Fn(Point) -> bool) -> bool {
        self.rng.contains(p) && passable(p)
    }

    fn jps_obstacle(&self, p: Point, passable: &impl Fn(Point) -> bool) -> bool {
        self.rng.contains(p) && !passable(p)
    }

    fn jps_add(
        &mut self,
        p: Point,
        parent: Point,
        to: Point,
        cost: i32,
        diags: bool,
        cur_gen: u32,
        open: &mut BinaryHeap<NodeRef>,
        passable: &impl Fn(Point) -> bool,
    ) {
        if !self.jps_pass(p, passable) {
            return;
        }
        let Some(pi) = self.idx(p) else { return };
        let nb = &mut self.astar_nodes[pi];
        if nb.generation == cur_gen {
            if cost < nb.g {
                // Better path; remove old.
                if nb.open {
                    // We can't efficiently remove from BinaryHeap,
                    // but the stale entry will be skipped.
                    nb.open = false;
                }
                // Fall through to re-add.
            } else {
                return; // equal or worse
            }
        }
        if nb.generation == cur_gen && nb.open {
            return; // shouldn't reach here, but safety check
        }
        if nb.generation == cur_gen && !nb.open && nb.g <= cost {
            return; // closed with better cost
        }
        let parent_idx = self.idx(parent).unwrap_or(usize::MAX);
        let delta = p - to;
        let dx = delta.x.abs();
        let dy = delta.y.abs();
        let h = if diags { dx.max(dy) } else { dx + dy };
        let rank = cost + h;

        let nb = &mut self.astar_nodes[pi];
        nb.g = cost;
        nb.f = rank;
        nb.parent = parent_idx;
        nb.generation = cur_gen;
        nb.open = true;
        open.push(NodeRef { idx: pi, f: rank });
    }

    // -- Straight max (edge proximity for forced successor optimization) --

    fn jps_straight_max(&self, p: Point, dir: Point) -> (i32, ForcedSucc) {
        let mut fs = ForcedSucc::Both;
        let max;
        if dir.x > 0 {
            max = self.rng.max.x - p.x;
            if p.y == self.rng.min.y {
                fs = sub_left(fs);
            }
            if p.y == self.rng.max.y - 1 {
                fs = sub_right(fs);
            }
        } else if dir.x < 0 {
            max = p.x - self.rng.min.x + 1;
            if p.y == self.rng.min.y {
                fs = sub_right(fs);
            }
            if p.y == self.rng.max.y - 1 {
                fs = sub_left(fs);
            }
        } else if dir.y > 0 {
            max = self.rng.max.y - p.y;
            if p.x == self.rng.min.x {
                fs = sub_right(fs);
            }
            if p.x == self.rng.max.x - 1 {
                fs = sub_left(fs);
            }
        } else {
            max = p.y - self.rng.min.y + 1;
            if p.x == self.rng.min.x {
                fs = sub_left(fs);
            }
            if p.x == self.rng.max.x - 1 {
                fs = sub_right(fs);
            }
        }
        (max, fs)
    }

    // -- Straight jumps (8-way) --

    fn jps_straight(
        &self,
        mut p: Point,
        dir: Point,
        to: Point,
        passable: &impl Fn(Point) -> bool,
    ) -> (Point, i32) {
        let (max, fs) = self.jps_straight_max(p, dir);
        match fs {
            ForcedSucc::None => {
                for i in 1..=max {
                    if !passable(p) { return (p, 0); }
                    if p == to { return (p, i); }
                    p = p + dir;
                }
                (p, 0)
            }
            ForcedSucc::Left => {
                for i in 1..=max {
                    if !passable(p) { return (p, 0); }
                    if p == to { return (p, i); }
                    let np = p + dir;
                    if !passable(left(p, dir)) && self.jps_pass(left(p, dir) + dir, passable) {
                        return (p, i);
                    }
                    p = np;
                }
                (p, 0)
            }
            ForcedSucc::Right => {
                for i in 1..=max {
                    if !passable(p) { return (p, 0); }
                    if p == to { return (p, i); }
                    let np = p + dir;
                    if !passable(right(p, dir)) && self.jps_pass(right(p, dir) + dir, passable) {
                        return (p, i);
                    }
                    p = np;
                }
                (p, 0)
            }
            ForcedSucc::Both => {
                for i in 1..=max {
                    if !passable(p) { return (p, 0); }
                    if p == to { return (p, i); }
                    let np = p + dir;
                    if !passable(left(p, dir)) && self.jps_pass(left(p, dir) + dir, passable) {
                        return (p, i);
                    }
                    if !passable(right(p, dir)) && self.jps_pass(right(p, dir) + dir, passable) {
                        return (p, i);
                    }
                    p = np;
                }
                (p, 0)
            }
        }
    }

    // -- Straight jumps (4-way / no-diags) --

    fn jps_straight_nd(
        &self,
        mut p: Point,
        dir: Point,
        to: Point,
        passable: &impl Fn(Point) -> bool,
    ) -> (Point, i32) {
        let (max, fs) = self.jps_straight_max(p, dir);
        match fs {
            ForcedSucc::None => {
                for i in 1..=max {
                    if !passable(p) { return (p, 0); }
                    if p == to { return (p, i); }
                    p = p + dir;
                }
                (p, 0)
            }
            ForcedSucc::Left => {
                for i in 1..=max {
                    if !passable(p) { return (p, 0); }
                    if p == to { return (p, i); }
                    let np = p + dir;
                    let ql = left(p, dir);
                    if !passable(ql) && self.jps_pass(ql + dir, passable) && self.jps_pass(np, passable) {
                        return (p, i);
                    }
                    p = np;
                }
                (p, 0)
            }
            ForcedSucc::Right => {
                for i in 1..=max {
                    if !passable(p) { return (p, 0); }
                    if p == to { return (p, i); }
                    let np = p + dir;
                    let qr = right(p, dir);
                    if !passable(qr) && self.jps_pass(qr + dir, passable) && self.jps_pass(np, passable) {
                        return (p, i);
                    }
                    p = np;
                }
                (p, 0)
            }
            ForcedSucc::Both => {
                for i in 1..=max {
                    if !passable(p) { return (p, 0); }
                    if p == to { return (p, i); }
                    let np = p + dir;
                    let ql = left(p, dir);
                    if !passable(ql) && self.jps_pass(ql + dir, passable) && self.jps_pass(np, passable) {
                        return (p, i);
                    }
                    let qr = right(p, dir);
                    if !passable(qr) && self.jps_pass(qr + dir, passable) && self.jps_pass(np, passable) {
                        return (p, i);
                    }
                    p = np;
                }
                (p, 0)
            }
        }
    }

    // -- Diagonal jumps --

    fn jps_diag(
        &mut self,
        mut p: Point,
        dir: Point,
        to: Point,
        cost: i32,
        passable: &impl Fn(Point) -> bool,
        _from: Point,
        cur_gen: u32,
        open: &mut BinaryHeap<NodeRef>,
    ) -> (Point, i32) {
        let mut i = 1;
        let origin = p - dir;
        loop {
            if !self.jps_pass(p, passable) {
                return (p, 0);
            }
            if p == to {
                return (p, i);
            }
            if self.jps_obstacle(p.shift(-dir.x, 0), passable)
                && self.jps_pass(p + Point::new(-dir.x, dir.y), passable)
            {
                return (p, i);
            }
            if self.jps_obstacle(p.shift(0, -dir.y), passable)
                && self.jps_pass(p + Point::new(dir.x, -dir.y), passable)
            {
                return (p, i);
            }
            let (q, j) = self.jps_straight(p.shift(dir.x, 0), Point::new(dir.x, 0), to, passable);
            if j > 0 {
                self.jps_add(q, origin, to, cost + i + j, true, cur_gen, open, passable);
            }
            let (q, j) = self.jps_straight(p.shift(0, dir.y), Point::new(0, dir.y), to, passable);
            if j > 0 {
                self.jps_add(q, origin, to, cost + i + j, true, cur_gen, open, passable);
            }
            p = p + dir;
            i += 1;
        }
    }

    fn jps_diag_nd(
        &mut self,
        mut p: Point,
        dir: Point,
        to: Point,
        cost: i32,
        passable: &impl Fn(Point) -> bool,
        _from: Point,
        cur_gen: u32,
        open: &mut BinaryHeap<NodeRef>,
    ) -> (Point, i32) {
        let mut i = 2; // diagonals cost 2 in 4-way
        let origin = p - dir;
        loop {
            if !self.jps_pass(p, passable) {
                return (p, 0);
            }
            let px_pass = self.jps_pass(p.shift(-dir.x, 0), passable);
            let py_pass = self.jps_pass(p.shift(0, -dir.y), passable);
            if !px_pass && !py_pass {
                return (p, 0);
            }
            if p == to {
                return (p, i);
            }
            if !px_pass
                && self.jps_pass(p + Point::new(-dir.x, dir.y), passable)
                && self.jps_pass(p + Point::new(0, dir.y), passable)
            {
                return (p, i);
            }
            if !py_pass
                && self.jps_pass(p + Point::new(dir.x, -dir.y), passable)
                && self.jps_pass(p + Point::new(dir.x, 0), passable)
            {
                return (p, i);
            }
            let (q, j) = self.jps_straight_nd(p.shift(dir.x, 0), Point::new(dir.x, 0), to, passable);
            if j > 0 {
                self.jps_add(q, origin, to, cost + i + j, false, cur_gen, open, passable);
            }
            let (q, j) = self.jps_straight_nd(p.shift(0, dir.y), Point::new(0, dir.y), to, passable);
            if j > 0 {
                self.jps_add(q, origin, to, cost + i + j, false, cur_gen, open, passable);
            }
            p = p + dir;
            i += 2;
        }
    }

    // -- Jump dispatchers --

    fn jps_jump(
        &mut self,
        p: Point,
        dir: Point,
        to: Point,
        cost: i32,
        passable: &impl Fn(Point) -> bool,
        from: Point,
        cur_gen: u32,
        open: &mut BinaryHeap<NodeRef>,
    ) -> (Point, i32) {
        if dir.x == 0 || dir.y == 0 {
            self.jps_straight(p, dir, to, passable)
        } else {
            self.jps_diag(p, dir, to, cost, passable, from, cur_gen, open)
        }
    }

    fn jps_jump_nd(
        &mut self,
        p: Point,
        dir: Point,
        to: Point,
        cost: i32,
        passable: &impl Fn(Point) -> bool,
        from: Point,
        cur_gen: u32,
        open: &mut BinaryHeap<NodeRef>,
    ) -> (Point, i32) {
        if dir.x == 0 || dir.y == 0 {
            self.jps_straight_nd(p, dir, to, passable)
        } else {
            self.jps_diag_nd(p, dir, to, cost, passable, from, cur_gen, open)
        }
    }

    // -- Neighbor generation --

    /// Returns (natural_neighbors, forced_neighbors_with_cost).
    fn jps_get_neighbors(
        &self,
        p: Point,
        parent: Point,
        _to: Point,
        _cost: i32,
        passable: &impl Fn(Point) -> bool,
        diags: bool,
    ) -> (Vec<Point>, Vec<(Point, i32)>) {
        let dir = dirnorm(parent, p);
        let mut nats = Vec::with_capacity(3);
        let mut forced = Vec::with_capacity(4);
        let dc = diag_cost(diags);

        if dir.x == 0 || dir.y == 0 {
            // Straight: natural = forward.
            nats.push(p + dir);
            let ql = left(p, dir);
            if !self.jps_pass(ql, passable) {
                if diags || self.jps_pass(p + dir, passable) {
                    forced.push((ql + dir, dc));
                }
            }
            let qr = right(p, dir);
            if !self.jps_pass(qr, passable) {
                if diags || self.jps_pass(p + dir, passable) {
                    forced.push((qr + dir, dc));
                }
            }
        } else {
            // Diagonal.
            let q0 = p.shift(dir.x, 0);
            let q1 = p.shift(0, dir.y);
            nats.push(q0);
            nats.push(q1);
            if diags || self.jps_pass(q0, passable) || self.jps_pass(q1, passable) {
                nats.push(p + dir);
            }
            let qx = p.shift(-dir.x, 0);
            if !self.jps_pass(qx, passable) {
                if diags || self.jps_pass(p + Point::new(0, dir.y), passable) {
                    forced.push((qx.shift(0, dir.y), dc));
                }
            }
            let qy = p.shift(0, -dir.y);
            if !self.jps_pass(qy, passable) {
                if diags || self.jps_pass(p + Point::new(dir.x, 0), passable) {
                    forced.push((qy.shift(dir.x, 0), dc));
                }
            }
        }

        (nats, forced)
    }

    // -- Path reconstruction --

    fn jps_jump_path(
        &self,
        path: &mut Vec<Point>,
        p: Point,
        q: Point,
        passable: &impl Fn(Point) -> bool,
        diags: bool,
    ) {
        let d = q - p;
        let dx = d.x.abs();
        let dy = d.y.abs();
        let dir = Point::new(sign(d.x), sign(d.y));

        let mut cur = p;
        // Straight portion first.
        if dx > dy {
            for _ in 0..(dx - dy) {
                path.push(cur);
                cur = cur + Point::new(dir.x, 0);
            }
        } else if dx < dy {
            for _ in 0..(dy - dx) {
                path.push(cur);
                cur = cur + Point::new(0, dir.y);
            }
        }
        // Diagonal portion.
        while cur != q {
            path.push(cur);
            if !diags && dir.x != 0 && dir.y != 0 {
                // Insert cardinal intermediate for the diagonal step.
                let px = cur + Point::new(dir.x, 0);
                if self.jps_pass(px, passable) {
                    path.push(px);
                } else {
                    let py = cur + Point::new(0, dir.y);
                    if self.jps_pass(py, passable) {
                        path.push(py);
                    }
                }
            }
            cur = cur + dir;
        }
    }

    fn jps_reconstruct(
        &self,
        from: Point,
        goal_idx: usize,
        passable: &impl Fn(Point) -> bool,
        diags: bool,
        _cur_gen: u32,
    ) -> Vec<Point> {
        let mut path = Vec::new();
        let mut ci = goal_idx;
        loop {
            let p = self.point(ci);
            if p == from {
                path.push(p);
                break;
            }
            let pi = self.astar_nodes[ci].parent;
            let pp = self.point(pi);
            self.jps_jump_path(&mut path, p, pp, passable, diags);
            ci = pi;
        }
        path.reverse();
        path
    }
}

fn sub_left(fs: ForcedSucc) -> ForcedSucc {
    match fs {
        ForcedSucc::Both => ForcedSucc::Right,
        ForcedSucc::Left => ForcedSucc::None,
        other => other,
    }
}

fn sub_right(fs: ForcedSucc) -> ForcedSucc {
    match fs {
        ForcedSucc::Both => ForcedSucc::Left,
        ForcedSucc::Right => ForcedSucc::None,
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gruid_core::Range;

    fn open_grid(_p: Point) -> bool {
        true
    }

    #[test]
    fn jps_8way_simple() {
        let mut pr = PathRange::new(Range::new(0, 0, 10, 10));
        let path = pr.jps_path(Point::new(0, 0), Point::new(5, 5), open_grid, true);
        assert!(path.is_some());
        let path = path.unwrap();
        assert_eq!(*path.first().unwrap(), Point::new(0, 0));
        assert_eq!(*path.last().unwrap(), Point::new(5, 5));
        // Chebyshev distance = 5, so optimal path has 6 points.
        assert_eq!(path.len(), 6);
    }

    #[test]
    fn jps_4way_simple() {
        let mut pr = PathRange::new(Range::new(0, 0, 10, 10));
        let path = pr.jps_path(Point::new(0, 0), Point::new(3, 0), open_grid, false);
        assert!(path.is_some());
        let path = path.unwrap();
        assert_eq!(*path.first().unwrap(), Point::new(0, 0));
        assert_eq!(*path.last().unwrap(), Point::new(3, 0));
        assert_eq!(path.len(), 4); // Manhattan distance = 3
    }

    #[test]
    fn jps_4way_around_wall() {
        // Grid with a wall:
        //   . . . . .
        //   . . # . .
        //   . . # . .
        //   . . . . .
        let mut pr = PathRange::new(Range::new(0, 0, 5, 4));
        let wall = |p: Point| !(p.x == 2 && (p.y == 1 || p.y == 2));
        let path = pr.jps_path(Point::new(0, 1), Point::new(4, 1), wall, false);
        assert!(path.is_some());
        let path = path.unwrap();
        assert_eq!(*path.first().unwrap(), Point::new(0, 1));
        assert_eq!(*path.last().unwrap(), Point::new(4, 1));
        // Should go around the wall via row 0 or row 3.
        assert!(path.len() >= 7); // minimum 4-way path around
    }

    #[test]
    fn jps_4way_matches_manhattan() {
        // On an open grid, 4-way JPS path length should equal Manhattan distance + 1.
        let mut pr = PathRange::new(Range::new(0, 0, 20, 20));
        let from = Point::new(2, 3);
        let to = Point::new(15, 10);
        let path = pr.jps_path(from, to, open_grid, false).unwrap();
        let manhattan = (to.x - from.x).abs() + (to.y - from.y).abs();
        assert_eq!(path.len(), manhattan as usize + 1);
    }

    #[test]
    fn jps_no_path() {
        // Completely walled off.
        let mut pr = PathRange::new(Range::new(0, 0, 5, 5));
        let wall = |p: Point| p.x < 2; // only left half passable
        let path = pr.jps_path(Point::new(0, 0), Point::new(4, 4), wall, true);
        assert!(path.is_none());
    }
}
