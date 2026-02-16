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

    /// Return 4 diagonal (inter-cardinal) neighbors of `p`, keeping only
    /// those for which `keep` returns `true`.
    ///
    /// The order is NW, NE, SW, SE (matching the Go reference
    /// implementation).
    pub fn diagonal(&mut self, p: Point, keep: impl Fn(Point) -> bool) -> &[Point] {
        self.buf.clear();
        const DIRS: [Point; 4] = [
            Point::new(-1, -1), // NW
            Point::new(1, -1),  // NE
            Point::new(-1, 1),  // SW
            Point::new(1, 1),   // SE
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diagonal_returns_four_diagonal_neighbors() {
        let mut nb = Neighbors::new();
        let p = Point::new(5, 5);
        let result = nb.diagonal(p, |_| true);
        assert_eq!(result.len(), 4);
        assert_eq!(
            result,
            &[
                Point::new(4, 4), // NW
                Point::new(6, 4), // NE
                Point::new(4, 6), // SW
                Point::new(6, 6), // SE
            ]
        );
    }

    #[test]
    fn diagonal_filters_with_keep() {
        let mut nb = Neighbors::new();
        let p = Point::new(0, 0);
        // Only keep points with non-negative coordinates
        let result = nb.diagonal(p, |q| q.x >= 0 && q.y >= 0);
        assert_eq!(result, &[Point::new(1, 1)]); // only SE
    }

    #[test]
    fn diagonal_order_matches_go() {
        // Go iterates y in {-1,1}, x in {-1,1} → (-1,-1),(1,-1),(-1,1),(1,1)
        let mut nb = Neighbors::new();
        let p = Point::new(10, 10);
        let result = nb.diagonal(p, |_| true);
        assert_eq!(result[0], Point::new(9, 9)); // NW: shift(-1,-1)
        assert_eq!(result[1], Point::new(11, 9)); // NE: shift( 1,-1)
        assert_eq!(result[2], Point::new(9, 11)); // SW: shift(-1, 1)
        assert_eq!(result[3], Point::new(11, 11)); // SE: shift( 1, 1)
    }

    #[test]
    fn cardinal_still_works() {
        let mut nb = Neighbors::new();
        let p = Point::new(3, 3);
        let result = nb.cardinal(p, |_| true);
        assert_eq!(result.len(), 4);
    }

    #[test]
    fn all_still_works() {
        let mut nb = Neighbors::new();
        let p = Point::new(3, 3);
        let result = nb.all(p, |_| true);
        assert_eq!(result.len(), 8);
    }
}
