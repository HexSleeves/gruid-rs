//! Field of Vision algorithms.
//!
//! Provides two FOV algorithms:
//! - **Ray-based** (`vision_map`): octant-parent ray propagation matching Go
//!   gruid's `VisionMap`. Produces continuous light rays with non-binary
//!   visibility. Supports `From`/`Ray` traceback.
//! - **Symmetric Shadow Casting** (`ssc_vision_map`): iterative SSC based on
//!   Albert Ford's algorithm. Binary visibility, expansive shadows.
//!
//! Both algorithms are symmetric (under certain conditions) with expansive
//! walls, and fast.

use gruid_core::{Point, Range};

/// A node that has been lit by the FOV computation, with its accumulated cost.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LightNode {
    pub pos: Point,
    pub cost: i32,
}

/// The shape of the FOV boundary.
///
/// By default, FOV algorithms produce a square (Chebyshev) boundary because
/// diagonal movement costs the same as cardinal. `Circle` clips results to
/// a Euclidean-distance circle instead.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum FovShape {
    /// Square boundary (Chebyshev distance). This is the default.
    #[default]
    Square,
    /// Circular boundary (Euclidean distance).
    Circle,
}

/// Trait for providing the cost of light passing through a cell.
///
/// Matches Go gruid's `Lighter` interface with `Cost(src, from, to)` and
/// `MaxCost(src)` methods.
pub trait Lighter {
    /// Return the cost of light propagation from `from` to adjacent `to`,
    /// given an original source `src`.
    ///
    /// As a special case, you normally want `cost(src, src, to) == 1`
    /// independently of terrain at src to guarantee symmetry.
    fn cost(&self, src: Point, from: Point, to: Point) -> i32;

    /// The maximum cost at which light can no longer propagate from `src`.
    /// Typically the maximum sight/light distance.
    fn max_cost(&self, src: Point) -> i32;
}

/// A wrapper around any [`Lighter`] that clips ray propagation to a
/// Euclidean-distance circle.
///
/// Use this with [`FOV::vision_map`] or [`FOV::light_map`] to get circular
/// FOV instead of the default square (Chebyshev) shape.
///
/// # Example
///
/// ```ignore
/// let lighter = CircularLighter::new(my_lighter);
/// fov.vision_map(&lighter, source);
/// ```
pub struct CircularLighter<L> {
    inner: L,
}

impl<L: Lighter> CircularLighter<L> {
    /// Wrap a lighter to produce circular FOV.
    pub fn new(inner: L) -> Self {
        Self { inner }
    }
}

impl<L: Lighter> Lighter for CircularLighter<L> {
    fn cost(&self, src: Point, from: Point, to: Point) -> i32 {
        let max = self.inner.max_cost(src);
        let dx = (to.x - src.x) as i64;
        let dy = (to.y - src.y) as i64;
        let dist_sq = dx * dx + dy * dy;
        let max_sq = (max as i64) * (max as i64);
        if dist_sq > max_sq {
            return i32::MAX;
        }
        self.inner.cost(src, from, to)
    }

    fn max_cost(&self, src: Point) -> i32 {
        self.inner.max_cost(src)
    }
}

fn sign(n: i32) -> i32 {
    if n > 0 {
        1
    } else if n < 0 {
        -1
    } else {
        0
    }
}

fn abs(x: i32) -> i32 {
    x.abs()
}

/// Field of Vision computation.
pub struct FOV {
    /// The rectangular range of valid positions.
    range: Range,
    /// Cost map for ray-based FOV. 0 = not visited, >0 = cost+1.
    /// (Uses Go's convention: stored value = actual_cost + 1, so 0 means unvisited.)
    costs: Vec<i32>,
    /// Visibility map for SSC FOV.
    shadow_casting: Vec<bool>,
    /// Cached list of lighted nodes from the last `vision_map`/`light_map` call.
    lighted: Vec<LightNode>,
    /// Cached list of visible points from the last `ssc_vision_map` call.
    visibles: Vec<Point>,
    /// Ray traceback cache.
    ray_cache: Vec<LightNode>,
    /// Source point from the last vision_map call.
    src: Point,
    /// Passable function for SSC (stored during computation).
    /// Tiles buffer for SSC scan.
    tiles_buf: Vec<Point>,
    /// Capacity (for lazy allocation).
    capacity: usize,
}

impl FOV {
    /// Create a new FOV for the given range.
    pub fn new(range: Range) -> Self {
        let w = range.width();
        let h = range.height();
        let cap = (w * h) as usize;
        Self {
            range,
            costs: Vec::new(),
            shadow_casting: Vec::new(),
            lighted: Vec::new(),
            visibles: Vec::new(),
            ray_cache: Vec::new(),
            src: Point::ZERO,
            tiles_buf: Vec::new(),
            capacity: cap,
        }
    }

    /// Change the range and reset internal buffers if needed.
    pub fn set_range(&mut self, range: Range) {
        let w = range.width();
        let h = range.height();
        let cap = (w * h) as usize;
        self.range = range;
        if cap > self.capacity {
            self.capacity = cap;
            self.costs = Vec::new();
            self.shadow_casting = Vec::new();
        }
    }

    /// Return the current range.
    pub fn range_(&self) -> Range {
        self.range
    }

    fn idx(&self, p: Point) -> usize {
        let q = p - self.range.min;
        let w = self.range.width();
        (q.y * w + q.x) as usize
    }

    fn ensure_costs(&mut self) {
        if self.costs.len() < self.capacity {
            self.costs.resize(self.capacity, 0);
        }
    }

    fn ensure_shadow_casting(&mut self) {
        if self.shadow_casting.len() < self.capacity {
            self.shadow_casting.resize(self.capacity, false);
        }
    }

    // ── Ray-based FOV (Go VisionMap) ───────────────────────────────

    /// Compute ray-based field of vision from `src`, matching Go's VisionMap.
    ///
    /// Returns a cached slice of lighted nodes. Values can also be consulted
    /// individually with [`at`](Self::at).
    pub fn vision_map(&mut self, lt: &impl Lighter, src: Point) -> &[LightNode] {
        self.lighted.clear();
        if !src.in_range(&self.range) {
            return &self.lighted;
        }
        self.ensure_costs();
        for c in &mut self.costs {
            *c = 0;
        }
        self.src = src;
        let src_idx = self.idx(src);
        self.costs[src_idx] = 1; // cost 0 stored as 1
        self.lighted.push(LightNode { pos: src, cost: 0 });

        let max_cost = lt.max_cost(src);
        for d in 1..=max_cost {
            let rg = self.range.intersect(Range::new(
                src.x - d,
                src.y - d + 1,
                src.x + d + 1,
                src.y + d,
            ));
            // South row
            if src.y + d < self.range.max.y {
                for x in rg.min.x..rg.max.x {
                    self.vision_update(lt, Point::new(x, src.y + d));
                }
            }
            // North row
            if src.y - d >= self.range.min.y {
                for x in rg.min.x..rg.max.x {
                    self.vision_update(lt, Point::new(x, src.y - d));
                }
            }
            // East column
            if src.x + d < self.range.max.x {
                for y in rg.min.y..rg.max.y {
                    self.vision_update(lt, Point::new(src.x + d, y));
                }
            }
            // West column
            if src.x - d >= self.range.min.x {
                for y in rg.min.y..rg.max.y {
                    self.vision_update(lt, Point::new(src.x - d, y));
                }
            }
        }
        &self.lighted
    }

    fn vision_update(&mut self, lt: &impl Lighter, to: Point) {
        let n = self.from_internal(lt, to);
        // Cost must be positive and finite (not overflowed/MAX).
        if n.cost > 0 && n.cost < i32::MAX {
            let to_idx = self.idx(to);
            self.costs[to_idx] = n.cost;
            self.lighted.push(LightNode {
                pos: to,
                cost: n.cost - 1,
            });
        }
    }

    /// Compute octant parents and find the minimum-cost parent for a position.
    /// Returns a LightNode with cost = stored cost (cost+1 in the array).
    fn from_internal(&self, lt: &impl Lighter, to: Point) -> LightNode {
        let q = self.src - to;
        let r = Point::new(sign(q.x), sign(q.y));

        // Primary parent: diagonal toward source
        let p0 = to + r;
        let c0 = self.costs[self.idx(p0)];

        // Secondary parent (only for non-axis, non-diagonal positions)
        let (has_p1, p1_cost, p1) = if q.x == 0 || q.y == 0 || abs(q.x) == abs(q.y) {
            (false, 0, Point::ZERO)
        } else if abs(q.x) > abs(q.y) {
            let p1 = to + Point::new(r.x, 0);
            (true, self.costs[self.idx(p1)], p1)
        } else {
            let p1 = to + Point::new(0, r.y);
            (true, self.costs[self.idx(p1)], p1)
        };

        // Collect valid parents
        let mut best = LightNode {
            pos: Point::ZERO,
            cost: 0,
        };

        if c0 > 0 && has_p1 && p1_cost > 0 {
            let cost0 = c0.saturating_add(lt.cost(self.src, p0, to));
            let cost1 = p1_cost.saturating_add(lt.cost(self.src, p1, to));
            if cost0 <= cost1 {
                best = LightNode {
                    pos: p0,
                    cost: cost0,
                };
            } else {
                best = LightNode {
                    pos: p1,
                    cost: cost1,
                };
            }
        } else if c0 > 0 {
            best = LightNode {
                pos: p0,
                cost: c0.saturating_add(lt.cost(self.src, p0, to)),
            };
        } else if has_p1 && p1_cost > 0 {
            best = LightNode {
                pos: p1,
                cost: p1_cost.saturating_add(lt.cost(self.src, p1, to)),
            };
        }

        best
    }

    /// Query the total ray cost at `p` from the last `vision_map`/`light_map`.
    /// Returns `None` if the position was not reached.
    pub fn at(&self, p: Point) -> Option<i32> {
        if !p.in_range(&self.range) || self.costs.is_empty() {
            return None;
        }
        let cost = self.costs[self.idx(p)];
        if cost <= 0 { None } else { Some(cost - 1) }
    }

    /// Iterate over all lighted nodes from the last `vision_map` call.
    pub fn iter_lighted(&self) -> impl Iterator<Item = LightNode> + '_ {
        self.lighted.iter().copied()
    }

    /// Return the previous position in the light ray to `to`,
    /// as computed in the last `vision_map` call.
    ///
    /// Returns `Some(LightNode)` with the parent position and accumulated
    /// cost (matching Go's `FOV.From`), or `None` if unreachable.
    pub fn from(&self, lt: &impl Lighter, to: Point) -> Option<LightNode> {
        self.at(to)?;
        let ln = self.from_internal(lt, to);
        if ln.cost == 0 {
            return None;
        }
        // `from_internal` already returns stored cost = parent_stored_cost + lt.cost(src, parent, to).
        // Subtract 1 to convert from stored (1-based) to actual (0-based) cost.
        Some(LightNode {
            pos: ln.pos,
            cost: ln.cost - 1,
        })
    }

    /// Return a full light ray from source to `to`.
    pub fn ray(&mut self, lt: &impl Lighter, to: Point) -> Option<&[LightNode]> {
        self.at(to)?;
        self.ray_cache.clear();
        let mut cur = to;
        while cur != self.src {
            let n = self.from_internal(lt, cur);
            self.ray_cache.push(LightNode {
                pos: cur,
                cost: n.cost - 1,
            });
            cur = n.pos;
        }
        self.ray_cache.push(LightNode {
            pos: self.src,
            cost: 0,
        });
        self.ray_cache.reverse();
        Some(&self.ray_cache)
    }

    // ── Multi-source light map ─────────────────────────────────────

    /// Build a lighting map with given light sources.
    pub fn light_map(&mut self, lt: &impl Lighter, srcs: &[Point]) -> &[LightNode] {
        self.ensure_costs();
        for c in &mut self.costs {
            *c = 0;
        }
        for &src in srcs {
            if !src.in_range(&self.range) {
                continue;
            }
            self.src = src;
            let src_idx = self.idx(src);
            self.costs[src_idx] = 1;
            let max_cost = lt.max_cost(src);
            for d in 1..=max_cost {
                let rg = self.range.intersect(Range::new(
                    src.x - d,
                    src.y - d + 1,
                    src.x + d + 1,
                    src.y + d,
                ));
                if src.y + d < self.range.max.y {
                    for x in rg.min.x..rg.max.x {
                        self.light_update(lt, Point::new(x, src.y + d));
                    }
                }
                if src.y - d >= self.range.min.y {
                    for x in rg.min.x..rg.max.x {
                        self.light_update(lt, Point::new(x, src.y - d));
                    }
                }
                if src.x + d < self.range.max.x {
                    for y in rg.min.y..rg.max.y {
                        self.light_update(lt, Point::new(src.x + d, y));
                    }
                }
                if src.x - d >= self.range.min.x {
                    for y in rg.min.y..rg.max.y {
                        self.light_update(lt, Point::new(src.x - d, y));
                    }
                }
            }
        }
        self.compute_lighted();
        &self.lighted
    }

    fn light_update(&mut self, lt: &impl Lighter, to: Point) {
        let n = self.from_internal(lt, to);
        if n.cost <= 0 || n.cost == i32::MAX {
            return;
        }
        let idx = self.idx(to);
        let cur = self.costs[idx];
        if cur > 0 && cur <= n.cost {
            return;
        }
        self.costs[idx] = n.cost;
    }

    fn compute_lighted(&mut self) {
        self.lighted.clear();
        let w = self.range.width();
        let h = self.range.height();
        let mut i = 0usize;
        for y in 0..h {
            for x in 0..w {
                let c = self.costs[i];
                if c > 0 {
                    self.lighted.push(LightNode {
                        pos: Point::new(x + self.range.min.x, y + self.range.min.y),
                        cost: c - 1,
                    });
                }
                i += 1;
            }
        }
    }

    // ── Symmetric Shadow Casting ───────────────────────────────────

    /// Compute field of vision using symmetric shadow casting.
    ///
    /// `passable` returns `true` if the given point does not block vision.
    /// `diags` controls whether diagonal adjacency is considered for
    /// revealing cells at the boundary.
    pub fn ssc_vision_map(
        &mut self,
        src: Point,
        max_depth: i32,
        passable: impl Fn(Point) -> bool,
        diags: bool,
    ) -> &[Point] {
        if !src.in_range(&self.range) {
            self.visibles.clear();
            return &self.visibles;
        }
        self.ensure_shadow_casting();
        for v in &mut self.shadow_casting {
            *v = false;
        }
        self.visibles.clear();
        self.ssc_internal(src, max_depth, &passable, diags);
        &self.visibles
    }

    /// Multi-source SSC.
    pub fn ssc_light_map(
        &mut self,
        srcs: &[Point],
        max_depth: i32,
        passable: impl Fn(Point) -> bool,
        diags: bool,
    ) -> &[Point] {
        self.ensure_shadow_casting();
        for v in &mut self.shadow_casting {
            *v = false;
        }
        self.visibles.clear();
        for &src in srcs {
            if src.in_range(&self.range) {
                self.ssc_internal(src, max_depth, &passable, diags);
            }
        }
        &self.visibles
    }

    /// Post-filter the current visibility results to a Euclidean circle.
    ///
    /// Call this after [`ssc_vision_map`](Self::ssc_vision_map) or
    /// [`ssc_light_map`](Self::ssc_light_map) to clip the square boundary
    /// to a circle centered on `center` with the given `radius`.
    pub fn retain_circular(&mut self, center: Point, radius: i32) {
        let r_sq = (radius as i64) * (radius as i64);
        self.visibles.retain(|&p| {
            let dx = (p.x - center.x) as i64;
            let dy = (p.y - center.y) as i64;
            dx * dx + dy * dy <= r_sq
        });
        // Update the shadow_casting bitmap to match.
        if !self.shadow_casting.is_empty() {
            // Clear all, then re-mark the retained points.
            for v in &mut self.shadow_casting {
                *v = false;
            }
            for &p in &self.visibles {
                if p.in_range(&self.range) {
                    let idx = self.idx(p);
                    self.shadow_casting[idx] = true;
                }
            }
        }
    }

    /// Convenience: SSC vision map clipped to a Euclidean circle.
    ///
    /// Equivalent to calling [`ssc_vision_map`](Self::ssc_vision_map) followed
    /// by [`retain_circular`](Self::retain_circular).
    pub fn ssc_vision_map_circular(
        &mut self,
        src: Point,
        radius: i32,
        passable: impl Fn(Point) -> bool,
        diags: bool,
    ) -> &[Point] {
        self.ssc_vision_map(src, radius, passable, diags);
        self.retain_circular(src, radius);
        &self.visibles
    }

    /// Convenience: SSC light map clipped to a Euclidean circle.
    pub fn ssc_light_map_circular(
        &mut self,
        srcs: &[Point],
        radius: i32,
        passable: impl Fn(Point) -> bool,
        diags: bool,
    ) -> &[Point] {
        self.ssc_light_map(srcs, radius, passable, diags);
        // For multi-source, clip each point against its nearest source.
        // Simplification: clip against the radius from any source.
        let r_sq = (radius as i64) * (radius as i64);
        self.visibles.retain(|&p| {
            srcs.iter().any(|&s| {
                let dx = (p.x - s.x) as i64;
                let dy = (p.y - s.y) as i64;
                dx * dx + dy * dy <= r_sq
            })
        });
        if !self.shadow_casting.is_empty() {
            for v in &mut self.shadow_casting {
                *v = false;
            }
            for &p in &self.visibles {
                if p.in_range(&self.range) {
                    let idx = self.idx(p);
                    self.shadow_casting[idx] = true;
                }
            }
        }
        &self.visibles
    }

    fn ssc_internal(
        &mut self,
        src: Point,
        max_depth: i32,
        passable: &impl Fn(Point) -> bool,
        diags: bool,
    ) {
        let idx = self.idx(src);
        if !self.shadow_casting[idx] {
            self.shadow_casting[idx] = true;
            self.visibles.push(src);
        }
        for dir in 0..4 {
            self.ssc_quadrant(src, max_depth, QuadDir(dir), passable, diags);
        }
    }

    fn reveal(&mut self, qt: Quadrant, tile: Point) {
        let p = qt.transform(tile);
        let idx = self.idx(p);
        if !self.shadow_casting[idx] {
            self.shadow_casting[idx] = true;
            self.visibles.push(p);
        }
    }

    fn ssc_quadrant(
        &mut self,
        src: Point,
        max_depth: i32,
        dir: QuadDir,
        passable: &impl Fn(Point) -> bool,
        diags: bool,
    ) {
        let qt = Quadrant { dir, p: src };
        let (colmin, colmax) = qt.max_cols(self.range);
        let mut dmax = qt.max_depth(self.range);
        if dmax > max_depth {
            dmax = max_depth;
        }
        if dmax == 0 {
            return;
        }

        let unreachable = max_depth + 1;
        let mut rows: Vec<SscRow> = vec![SscRow {
            depth: 1,
            slope_start: Point::new(-1, 1),
            slope_end: Point::new(1, 1),
        }];

        while let Some(mut r) = rows.pop() {
            let mut ptile = Point::new(unreachable, 0);
            self.tiles_buf.clear();
            r.tiles(&mut self.tiles_buf, colmin, colmax);
            let tiles_len = self.tiles_buf.len();
            for ti in 0..tiles_len {
                let tile = self.tiles_buf[ti];
                let wall = !passable(qt.transform(tile));
                if (wall || r.is_symmetric(tile))
                    && (diags
                        || (tile.x <= 1 && tile.y == 0)
                        || (tile.x > 1 && passable(qt.transform(tile.shift(-1, 0))))
                        || (tile.y >= 0 && passable(qt.transform(tile.shift(0, -1))))
                        || (tile.y <= 0 && passable(qt.transform(tile.shift(0, 1)))))
                {
                    self.reveal(qt, tile);
                }
                if ptile.x == unreachable {
                    ptile = tile;
                    continue;
                }
                let pwall = !passable(qt.transform(ptile));
                if pwall && !wall {
                    // Transition wall -> floor: update running start slope.
                    if !diags {
                        if tile.x < dmax && !passable(qt.transform(tile.shift(1, 0))) {
                            r.slope_start = slope_square(tile.shift(1, 0));
                        } else if tile.x > 1 && !passable(qt.transform(tile.shift(-1, 0))) {
                            r.slope_start = slope_diamond(tile.shift(-1, 1));
                        } else {
                            r.slope_start = slope_diamond(tile);
                        }
                    } else {
                        r.slope_start = slope_diamond(tile);
                    }
                }
                if !pwall && wall {
                    // Transition floor -> wall: push child row for the
                    // floor segment we just passed.
                    let mut nr = r.next();
                    if !diags {
                        if tile.x < dmax && !passable(qt.transform(ptile.shift(1, 0))) {
                            nr.slope_end = slope_square(tile.shift(1, 0));
                        } else if ptile.x > 1 && !passable(qt.transform(ptile.shift(-1, 0))) {
                            nr.slope_end = slope_diamond(ptile.shift(-1, 0));
                        } else {
                            nr.slope_end = slope_diamond(tile);
                        }
                    } else {
                        nr.slope_end = slope_diamond(tile);
                    }
                    if nr.depth <= dmax {
                        rows.push(nr);
                    }
                }
                ptile = tile;
            }
            if ptile.x == unreachable {
                continue;
            }
            if passable(qt.transform(ptile)) && r.depth < dmax {
                rows.push(r.next());
            }
        }
    }

    /// Query whether `p` is visible from the last `ssc_vision_map` call.
    pub fn visible(&self, p: Point) -> bool {
        if !p.in_range(&self.range) || self.shadow_casting.is_empty() {
            return false;
        }
        self.shadow_casting[self.idx(p)]
    }

    /// Iterate over all visible points from the last `ssc_vision_map` call.
    pub fn iter_visible(&self) -> impl Iterator<Item = Point> + '_ {
        self.visibles.iter().copied()
    }
}

// ── SSC helper types ──────────────────────────────────────────────

#[derive(Debug, Clone, Copy)]
struct QuadDir(i32);

#[derive(Debug, Clone, Copy)]
struct Quadrant {
    dir: QuadDir,
    p: Point,
}

impl Quadrant {
    fn transform(&self, tile: Point) -> Point {
        match self.dir.0 {
            0 => Point::new(self.p.x + tile.y, self.p.y - tile.x), // north
            1 => Point::new(self.p.x + tile.x, self.p.y + tile.y), // east
            2 => Point::new(self.p.x + tile.y, self.p.y + tile.x), // south
            _ => Point::new(self.p.x - tile.x, self.p.y + tile.y), // west
        }
    }

    fn max_cols(&self, rg: Range) -> (i32, i32) {
        match self.dir.0 {
            0 | 2 => {
                let dx = self.p.x - rg.min.x;
                let dy = rg.max.x - self.p.x - 1;
                (-dx, dy)
            }
            _ => {
                let dx = self.p.y - rg.min.y;
                let dy = rg.max.y - self.p.y - 1;
                (-dx, dy)
            }
        }
    }

    fn max_depth(&self, rg: Range) -> i32 {
        match self.dir.0 {
            0 => self.p.y - rg.min.y,
            1 => rg.max.x - self.p.x - 1,
            2 => rg.max.y - self.p.y - 1,
            _ => self.p.x - rg.min.x,
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct SscRow {
    depth: i32,
    slope_start: Point, // fractional as (num, den)
    slope_end: Point,
}

impl SscRow {
    fn tiles(&self, ts: &mut Vec<Point>, colmin: i32, colmax: i32) {
        let depth = self.depth;
        let mut min = {
            let n = depth * self.slope_start.x;
            let div = n / self.slope_start.y;
            let rem = n % self.slope_start.y;
            match sign(rem) {
                1 => {
                    if 2 * rem >= self.slope_start.y {
                        div + 1
                    } else {
                        div
                    }
                }
                -1 => {
                    if -2 * rem > self.slope_start.y {
                        div - 1
                    } else {
                        div
                    }
                }
                _ => div,
            }
        };
        let mut max = {
            let n = depth * self.slope_end.x;
            let div = n / self.slope_end.y;
            let rem = n % self.slope_end.y;
            match sign(rem) {
                1 => {
                    if 2 * rem > self.slope_end.y {
                        div + 1
                    } else {
                        div
                    }
                }
                -1 => {
                    if -2 * rem >= self.slope_end.y {
                        div - 1
                    } else {
                        div
                    }
                }
                _ => div,
            }
        };
        if min < colmin {
            min = colmin;
        }
        if max > colmax {
            max = colmax;
        }
        for col in min..=max {
            ts.push(Point::new(depth, col));
        }
    }

    fn next(self) -> SscRow {
        SscRow {
            depth: self.depth + 1,
            slope_start: self.slope_start,
            slope_end: self.slope_end,
        }
    }

    fn is_symmetric(&self, tile: Point) -> bool {
        let col = tile.y;
        col * self.slope_start.y >= self.depth * self.slope_start.x
            && col * self.slope_end.y <= self.depth * self.slope_end.x
    }
}

fn slope_diamond(tile: Point) -> Point {
    Point::new(2 * tile.y - 1, 2 * tile.x)
}

fn slope_square(tile: Point) -> Point {
    Point::new(2 * tile.y - 1, 2 * tile.x + 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct SimpleWalls {
        walls: Vec<Point>,
        max_cost: i32,
    }

    impl Lighter for SimpleWalls {
        fn cost(&self, _src: Point, from: Point, to: Point) -> i32 {
            // Source cell always costs 1 for symmetry.
            if from == to {
                return 1;
            }
            if self.walls.contains(&from) {
                i32::MAX
            } else {
                1
            }
        }

        fn max_cost(&self, _src: Point) -> i32 {
            self.max_cost
        }
    }

    #[test]
    fn test_vision_map_open() {
        let range = Range::new(0, 0, 10, 10);
        let mut fov = FOV::new(range);
        let lighter = SimpleWalls {
            walls: vec![],
            max_cost: 3,
        };
        fov.vision_map(&lighter, Point::new(5, 5));

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
            max_cost: 10,
        };
        fov.vision_map(&lighter, Point::new(5, 5));

        // Source visible.
        assert_eq!(fov.at(Point::new(5, 5)), Some(0));
        // Wall cell is reached (we see the wall) but with high cost.
        // Behind the wall should not be reached.
        assert_eq!(fov.at(Point::new(8, 5)), None);
    }

    #[test]
    fn test_ssc_open_field() {
        let range = Range::new(0, 0, 11, 11);
        let mut fov = FOV::new(range);
        let source = Point::new(5, 5);
        fov.ssc_vision_map(source, 3, |_| true, true);

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
        let wall = Point::new(6, 5);
        fov.ssc_vision_map(source, 5, |p| p != wall, true);

        assert!(fov.visible(source));
        assert!(fov.visible(wall));
        assert!(!fov.visible(Point::new(7, 5)));
        assert!(!fov.visible(Point::new(8, 5)));
    }

    #[test]
    fn test_ssc_symmetry() {
        let range = Range::new(0, 0, 20, 20);
        let wall = Point::new(8, 10);
        let passable = |p: Point| p != wall;

        let a = Point::new(10, 10);
        let b = Point::new(6, 10);

        let mut fov = FOV::new(range);
        fov.ssc_vision_map(a, 10, passable, true);
        let a_sees_b = fov.visible(b);

        fov.ssc_vision_map(b, 10, passable, true);
        let b_sees_a = fov.visible(a);

        assert_eq!(a_sees_b, b_sees_a, "SSC should be symmetric");
    }

    // ── Circular FOV tests ─────────────────────────────────────

    #[test]
    fn test_circular_lighter_clips_corners() {
        // With radius 5, the corner at (5,5) from source has Euclidean
        // distance sqrt(50) ≈ 7.07 — should NOT be visible.
        let range = Range::new(0, 0, 20, 20);
        let mut fov = FOV::new(range);
        let src = Point::new(10, 10);
        let radius = 5;

        // Square FOV first — corner should be visible.
        let lighter_sq = SimpleWalls {
            walls: vec![],
            max_cost: radius,
        };
        fov.vision_map(&lighter_sq, src);
        let corner = Point::new(15, 15);
        assert!(
            fov.at(corner).is_some(),
            "Square FOV should reach the corner"
        );

        // Circular FOV — corner should NOT be visible.
        let lighter_circ = CircularLighter::new(SimpleWalls {
            walls: vec![],
            max_cost: radius,
        });
        fov.vision_map(&lighter_circ, src);
        assert!(
            fov.at(corner).is_none(),
            "Circular FOV should NOT reach the corner at distance ~7.07"
        );

        // But a point along an axis at distance 5 should still be visible.
        let axis_pt = Point::new(15, 10);
        assert!(
            fov.at(axis_pt).is_some(),
            "Circular FOV should reach axis point at distance 5"
        );
    }

    #[test]
    fn test_circular_lighter_near_diagonal() {
        // Point at (3,4) from source has distance 5 — should be visible
        // with radius 5. Point at (4,4) has distance ~5.66 — should not.
        let range = Range::new(0, 0, 20, 20);
        let mut fov = FOV::new(range);
        let src = Point::new(10, 10);
        let lighter = CircularLighter::new(SimpleWalls {
            walls: vec![],
            max_cost: 5,
        });
        fov.vision_map(&lighter, src);

        assert!(
            fov.at(Point::new(13, 14)).is_some(),
            "(3,4) from src: distance=5, should be visible"
        );
        assert!(
            fov.at(Point::new(14, 14)).is_none(),
            "(4,4) from src: distance≈5.66, should NOT be visible"
        );
    }

    #[test]
    fn test_ssc_vision_map_circular() {
        let range = Range::new(0, 0, 20, 20);
        let mut fov = FOV::new(range);
        let src = Point::new(10, 10);

        // Square SSC first.
        fov.ssc_vision_map(src, 5, |_| true, true);
        let square_count = fov.iter_visible().count();

        // Circular SSC.
        fov.ssc_vision_map_circular(src, 5, |_| true, true);
        let circle_count = fov.iter_visible().count();

        // Circle should have fewer visible cells than square.
        assert!(
            circle_count < square_count,
            "Circle ({circle_count}) should have fewer visible cells than square ({square_count})"
        );

        // Corner should be clipped.
        assert!(
            !fov.visible(Point::new(15, 15)),
            "Corner should not be visible in circular SSC"
        );

        // Axis point should remain.
        assert!(
            fov.visible(Point::new(15, 10)),
            "Axis point should remain visible in circular SSC"
        );
    }

    #[test]
    fn test_retain_circular_updates_bitmap() {
        let range = Range::new(0, 0, 20, 20);
        let mut fov = FOV::new(range);
        let src = Point::new(10, 10);

        fov.ssc_vision_map(src, 5, |_| true, true);
        assert!(fov.visible(Point::new(15, 15)), "pre-filter: corner visible");

        fov.retain_circular(src, 5);
        assert!(
            !fov.visible(Point::new(15, 15)),
            "post-filter: corner not visible"
        );
        assert!(
            fov.visible(src),
            "post-filter: source still visible"
        );
    }

    #[test]
    fn test_fov_shape_default() {
        assert_eq!(FovShape::default(), FovShape::Square);
    }

    /// Test that `from()` returns the correct cost without double-counting
    /// the lt.cost() of the last step. Mirrors Go's TestFOV assertion:
    ///   `fov.From(lt, Point{5,0})` returns the parent at (4,0) with
    ///   the accumulated stored cost minus 1.
    #[test]
    fn test_fov_from_no_extra_cost() {
        // Lighter: cost(src, src, _) = 0, diagonal = 2, else = 1.
        // Matches Go test lighter.
        struct TestLighter {
            max: i32,
        }
        impl Lighter for TestLighter {
            fn cost(&self, src: Point, from: Point, to: Point) -> i32 {
                if src == from {
                    return 0;
                }
                let step = Point::new(to.x - from.x, to.y - from.y);
                if step.x != 0 && step.y != 0 {
                    2
                } else {
                    1
                }
            }
            fn max_cost(&self, _src: Point) -> i32 {
                self.max
            }
        }

        let max_los = 10;
        let range = Range::new(-max_los, -max_los, max_los + 2, max_los + 2);
        let mut fov = FOV::new(range);
        let lt = TestLighter { max: max_los };
        fov.vision_map(&lt, Point::new(0, 0));

        // From(lt, (5,0)) should return parent at (4,0).
        let node = fov.from(&lt, Point::new(5, 0));
        assert!(node.is_some(), "(5,0) should be reachable");
        let node = node.unwrap();
        assert_eq!(node.pos.x, 4, "parent should be at x=4");
        assert_eq!(node.pos.y, 0, "parent should be at y=0");

        // The cost at (5,0) via at() should equal the from() node cost.
        // Go: at(5,0) returns 5 (cost 0 at source + 0 for first step
        // + 1 each subsequent = 5 for axis). from() cost must equal at() cost.
        let at_cost = fov.at(Point::new(5, 0)).unwrap();
        assert_eq!(
            node.cost, at_cost,
            "from() cost ({}) should equal at() cost ({})",
            node.cost, at_cost
        );

        // Verify the ray has correct length (6 nodes: source + 5 steps).
        let ray = fov.ray(&lt, Point::new(5, 0));
        assert!(ray.is_some());
        assert_eq!(ray.unwrap().len(), 6, "ray from (0,0) to (5,0) should have 6 nodes");
    }
}
