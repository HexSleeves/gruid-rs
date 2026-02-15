use gruid_core::Point;

/// Cached neighbor computation helper.
///
/// Provides methods for enumerating cardinal (4-way) or all (8-way)
/// neighbors of a grid point, filtered by a predicate.
pub struct Neighbors {
    buf: Vec<Point>,
}

impl Default for Neighbors {
    fn default() -> Self {
        Self::new()
    }
}

impl Neighbors {
    /// Create a new `Neighbors` helper.
    pub fn new() -> Self {
        Self {
            buf: Vec::with_capacity(8),
        }
    }

    /// Return 4-directional (cardinal) neighbors of `p`, keeping only those
    /// for which `keep` returns `true`.
    pub fn cardinal(&mut self, p: Point, keep: impl Fn(Point) -> bool) -> &[Point] {
        self.buf.clear();
        const DIRS: [Point; 4] = [
            Point::new(0, -1),
            Point::new(1, 0),
            Point::new(0, 1),
            Point::new(-1, 0),
        ];
        for d in DIRS {
            let n = p + d;
            if keep(n) {
                self.buf.push(n);
            }
        }
        &self.buf
    }

    /// Return 8-directional neighbors of `p`, keeping only those for which
    /// `keep` returns `true`.
    pub fn all(&mut self, p: Point, keep: impl Fn(Point) -> bool) -> &[Point] {
        self.buf.clear();
        for dy in -1..=1 {
            for dx in -1..=1 {
                if dx == 0 && dy == 0 {
                    continue;
                }
                let n = p + Point::new(dx, dy);
                if keep(n) {
                    self.buf.push(n);
                }
            }
        }
        &self.buf
    }
}
