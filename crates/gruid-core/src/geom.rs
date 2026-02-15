//! Geometry primitives: [`Point`] and [`Range`].
//!
//! These mirror Go gruid's `gruid.Point` and `gruid.Range` but are idiomatic Rust.

use std::fmt;
use std::hash::{Hash, Hasher};
use std::ops::{Add, Sub, Mul, Div};

// ---------------------------------------------------------------------------
// Point
// ---------------------------------------------------------------------------

/// A 2D integer point. X grows right, Y grows down (screen coordinates).
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

impl Point {
    /// Origin (0, 0).
    pub const ZERO: Self = Self { x: 0, y: 0 };

    /// Create a new point.
    #[inline]
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    /// Return a point shifted by (dx, dy).
    #[inline]
    pub const fn shift(self, dx: i32, dy: i32) -> Self {
        Self {
            x: self.x + dx,
            y: self.y + dy,
        }
    }

    /// Whether the point is inside the half-open range.
    #[inline]
    pub fn in_range(self, r: &Range) -> bool {
        r.contains(self)
    }

    /// The four cardinal neighbours (up, right, down, left).
    #[inline]
    pub fn neighbors_4(self) -> [Point; 4] {
        [
            Self::new(self.x, self.y - 1),
            Self::new(self.x + 1, self.y),
            Self::new(self.x, self.y + 1),
            Self::new(self.x - 1, self.y),
        ]
    }

    /// All eight neighbours (cardinal + diagonal).
    #[inline]
    pub fn neighbors_8(self) -> [Point; 8] {
        [
            Self::new(self.x, self.y - 1),
            Self::new(self.x + 1, self.y - 1),
            Self::new(self.x + 1, self.y),
            Self::new(self.x + 1, self.y + 1),
            Self::new(self.x, self.y + 1),
            Self::new(self.x - 1, self.y + 1),
            Self::new(self.x - 1, self.y),
            Self::new(self.x - 1, self.y - 1),
        ]
    }
}

// --- trait impls for Point ---

impl Hash for Point {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.x.hash(state);
        self.y.hash(state);
    }
}

impl PartialOrd for Point {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Point {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.y.cmp(&other.y).then(self.x.cmp(&other.x))
    }
}

impl fmt::Display for Point {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}, {})", self.x, self.y)
    }
}

impl Add for Point {
    type Output = Self;
    #[inline]
    fn add(self, rhs: Self) -> Self {
        Self::new(self.x + rhs.x, self.y + rhs.y)
    }
}

impl Sub for Point {
    type Output = Self;
    #[inline]
    fn sub(self, rhs: Self) -> Self {
        Self::new(self.x - rhs.x, self.y - rhs.y)
    }
}

impl Mul<i32> for Point {
    type Output = Self;
    #[inline]
    fn mul(self, rhs: i32) -> Self {
        Self::new(self.x * rhs, self.y * rhs)
    }
}

impl Div<i32> for Point {
    type Output = Self;
    #[inline]
    fn div(self, rhs: i32) -> Self {
        Self::new(self.x / rhs, self.y / rhs)
    }
}

// ---------------------------------------------------------------------------
// Range
// ---------------------------------------------------------------------------

/// A half-open rectangle \[min, max). `min` is inclusive, `max` is exclusive.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Range {
    pub min: Point,
    pub max: Point,
}

impl Range {
    /// Create a new range from two corners and auto-canonicalize so that
    /// `min` ≤ `max` on each axis.
    #[inline]
    pub fn new(x0: i32, y0: i32, x1: i32, y1: i32) -> Self {
        Self {
            min: Point::new(x0.min(x1), y0.min(y1)),
            max: Point::new(x0.max(x1), y0.max(y1)),
        }
    }

    /// Size as a `Point` (width = max.x - min.x, height = max.y - min.y).
    #[inline]
    pub fn size(self) -> Point {
        Point::new(self.max.x - self.min.x, self.max.y - self.min.y)
    }

    /// Width of the range.
    #[inline]
    pub fn width(self) -> i32 {
        self.max.x - self.min.x
    }

    /// Height of the range.
    #[inline]
    pub fn height(self) -> i32 {
        self.max.y - self.min.y
    }

    /// Return a range shifted by the given deltas.
    #[inline]
    pub fn shift(self, dx0: i32, dy0: i32, dx1: i32, dy1: i32) -> Self {
        Self {
            min: self.min.shift(dx0, dy0),
            max: self.max.shift(dx1, dy1),
        }
    }

    /// A single-row sub-range at row `y`.
    #[inline]
    pub fn line(self, y: i32) -> Self {
        Self {
            min: Point::new(self.min.x, y),
            max: Point::new(self.max.x, y + 1),
        }
    }

    /// Sub-range spanning rows `[y0, y1)`.
    #[inline]
    pub fn lines(self, y0: i32, y1: i32) -> Self {
        Self {
            min: Point::new(self.min.x, y0),
            max: Point::new(self.max.x, y1),
        }
    }

    /// A single-column sub-range at column `x`.
    #[inline]
    pub fn column(self, x: i32) -> Self {
        Self {
            min: Point::new(x, self.min.y),
            max: Point::new(x + 1, self.max.y),
        }
    }

    /// Sub-range spanning columns `[x0, x1)`.
    #[inline]
    pub fn columns(self, x0: i32, x1: i32) -> Self {
        Self {
            min: Point::new(x0, self.min.y),
            max: Point::new(x1, self.max.y),
        }
    }

    /// Total number of cells in the range.
    #[inline]
    pub fn len(self) -> usize {
        if self.is_empty() {
            return 0;
        }
        (self.width() as usize) * (self.height() as usize)
    }

    /// Whether the range has zero or negative area.
    #[inline]
    pub fn is_empty(self) -> bool {
        self.min.x >= self.max.x || self.min.y >= self.max.y
    }

    /// Whether `p` is inside the half-open range.
    #[inline]
    pub fn contains(self, p: Point) -> bool {
        p.x >= self.min.x && p.x < self.max.x && p.y >= self.min.y && p.y < self.max.y
    }

    /// Intersection of two ranges (may be empty).
    #[inline]
    pub fn intersect(self, other: Range) -> Self {
        Self {
            min: Point::new(self.min.x.max(other.min.x), self.min.y.max(other.min.y)),
            max: Point::new(self.max.x.min(other.max.x), self.max.y.min(other.max.y)),
        }
    }

    /// Smallest range that contains both ranges.
    #[inline]
    pub fn union(self, other: Range) -> Self {
        if self.is_empty() {
            return other;
        }
        if other.is_empty() {
            return self;
        }
        Self {
            min: Point::new(self.min.x.min(other.min.x), self.min.y.min(other.min.y)),
            max: Point::new(self.max.x.max(other.max.x), self.max.y.max(other.max.y)),
        }
    }

    /// Whether the two ranges overlap (non-empty intersection).
    #[inline]
    pub fn overlaps(self, other: Range) -> bool {
        !self.intersect(other).is_empty()
    }

    /// Row-major iterator over every point in the range.
    #[inline]
    pub fn iter(self) -> RangeIter {
        RangeIter {
            range: self,
            cur: self.min,
        }
    }
}

impl IntoIterator for Range {
    type Item = Point;
    type IntoIter = RangeIter;
    #[inline]
    fn into_iter(self) -> RangeIter {
        self.iter()
    }
}

impl fmt::Display for Range {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}-{})", self.min, self.max)
    }
}

// ---------------------------------------------------------------------------
// RangeIter
// ---------------------------------------------------------------------------

/// Row-major iterator over the points in a [`Range`].
#[derive(Clone, Debug)]
pub struct RangeIter {
    range: Range,
    cur: Point,
}

impl Iterator for RangeIter {
    type Item = Point;

    #[inline]
    fn next(&mut self) -> Option<Point> {
        if self.cur.y >= self.range.max.y || self.range.is_empty() {
            return None;
        }
        let p = self.cur;
        self.cur.x += 1;
        if self.cur.x >= self.range.max.x {
            self.cur.x = self.range.min.x;
            self.cur.y += 1;
        }
        Some(p)
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        if self.range.is_empty() || self.cur.y >= self.range.max.y {
            return (0, Some(0));
        }
        let w = self.range.width() as usize;
        let remaining_in_row = (self.range.max.x - self.cur.x) as usize;
        let remaining_rows = (self.range.max.y - self.cur.y - 1) as usize;
        let total = remaining_in_row + remaining_rows * w;
        (total, Some(total))
    }
}

impl ExactSizeIterator for RangeIter {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn point_arithmetic() {
        let a = Point::new(1, 2);
        let b = Point::new(3, 4);
        assert_eq!(a + b, Point::new(4, 6));
        assert_eq!(b - a, Point::new(2, 2));
        assert_eq!(a * 3, Point::new(3, 6));
        assert_eq!(b / 2, Point::new(1, 2));
    }

    #[test]
    fn range_basics() {
        let r = Range::new(0, 0, 3, 2);
        assert_eq!(r.size(), Point::new(3, 2));
        assert!(!r.is_empty());
        assert!(r.contains(Point::new(0, 0)));
        assert!(r.contains(Point::new(2, 1)));
        assert!(!r.contains(Point::new(3, 0)));
        assert!(!r.contains(Point::new(0, 2)));
    }

    #[test]
    fn range_auto_canonicalize() {
        let r = Range::new(3, 2, 0, 0);
        assert_eq!(r.min, Point::new(0, 0));
        assert_eq!(r.max, Point::new(3, 2));
    }

    #[test]
    fn range_iter_count() {
        let r = Range::new(0, 0, 3, 2);
        let pts: Vec<_> = r.iter().collect();
        assert_eq!(pts.len(), 6);
        assert_eq!(pts[0], Point::new(0, 0));
        assert_eq!(pts[5], Point::new(2, 1));
    }

    #[test]
    fn range_intersect() {
        let a = Range::new(0, 0, 4, 4);
        let b = Range::new(2, 2, 6, 6);
        let c = a.intersect(b);
        assert_eq!(c, Range::new(2, 2, 4, 4));
    }

    #[test]
    fn empty_range_iter() {
        let r = Range::new(0, 0, 0, 0);
        assert!(r.is_empty());
        assert_eq!(r.iter().count(), 0);
    }
}
