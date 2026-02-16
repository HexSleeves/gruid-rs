//! Geometry primitives: [`Point`] and [`Range`].
//!
//! These mirror Go gruid's `gruid.Point` and `gruid.Range` but are idiomatic Rust.

use std::fmt;
use std::hash::{Hash, Hasher};
use std::ops::{Add, Div, Mul, Sub};

use crate::messages::Msg;

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
///
/// All empty ranges are considered equal (mirroring Go gruid's `Range.Eq`).
#[derive(Copy, Clone, Debug, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Range {
    pub min: Point,
    pub max: Point,
}

impl PartialEq for Range {
    /// Two ranges are equal if they describe the same set of points.
    /// All empty ranges are considered equal, matching Go gruid behavior.
    fn eq(&self, other: &Self) -> bool {
        (self.min == other.min && self.max == other.max) || (self.is_empty() && other.is_empty())
    }
}

impl Eq for Range {}

impl Hash for Range {
    fn hash<H: Hasher>(&self, state: &mut H) {
        if self.is_empty() {
            // All empty ranges hash the same.
            0i32.hash(state);
            0i32.hash(state);
            0i32.hash(state);
            0i32.hash(state);
        } else {
            self.min.hash(state);
            self.max.hash(state);
        }
    }
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

    /// Return a range of same size translated by `+p`.
    ///
    /// Matches Go gruid's `Range.Add`. Also available via `range + point`.
    #[inline]
    #[allow(clippy::should_implement_trait)]
    pub fn add(self, p: Point) -> Self {
        Self {
            min: self.min + p,
            max: self.max + p,
        }
    }

    /// Return a range of same size translated by `-p`.
    ///
    /// Matches Go gruid's `Range.Sub`. Also available via `range - point`.
    #[inline]
    #[allow(clippy::should_implement_trait)]
    pub fn sub(self, p: Point) -> Self {
        Self {
            min: self.min - p,
            max: self.max - p,
        }
    }

    /// Return a range with coordinates shifted by the given deltas.
    ///
    /// If the result would be empty, returns the zero (empty) range,
    /// matching Go gruid's `Range.Shift` behavior.
    #[inline]
    pub fn shift(self, dx0: i32, dy0: i32, dx1: i32, dy1: i32) -> Self {
        let r = Self {
            min: self.min.shift(dx0, dy0),
            max: self.max.shift(dx1, dy1),
        };
        if r.is_empty() { Self::default() } else { r }
    }

    /// Reduces the range to **relative** line `y` (0 = first line of the range).
    ///
    /// Returns the intersection with the range, or an empty range if `y` is
    /// out of bounds. Matches Go gruid's `Range.Line`.
    #[inline]
    pub fn line(self, y: i32) -> Self {
        if self.min.shift(0, y).in_range(&self) {
            Self {
                min: Point::new(self.min.x, self.min.y + y),
                max: Point::new(self.max.x, self.min.y + y + 1),
            }
        } else {
            Self::default()
        }
    }

    /// Reduces the range to **relative** rows `[y0, y1)` (0 = first line).
    ///
    /// Returns the intersection with the original range.
    /// Matches Go gruid's `Range.Lines`.
    #[inline]
    pub fn lines(self, y0: i32, y1: i32) -> Self {
        let nrg = Self {
            min: Point::new(self.min.x, self.min.y + y0),
            max: Point::new(self.max.x, self.min.y + y1),
        };
        self.intersect(nrg)
    }

    /// Reduces the range to **relative** column `x` (0 = first column).
    ///
    /// Returns the intersection with the range, or an empty range if `x` is
    /// out of bounds. Matches Go gruid's `Range.Column`.
    #[inline]
    pub fn column(self, x: i32) -> Self {
        if self.min.shift(x, 0).in_range(&self) {
            Self {
                min: Point::new(self.min.x + x, self.min.y),
                max: Point::new(self.min.x + x + 1, self.max.y),
            }
        } else {
            Self::default()
        }
    }

    /// Reduces the range to **relative** columns `[x0, x1)` (0 = first column).
    ///
    /// Returns the intersection with the original range.
    /// Matches Go gruid's `Range.Columns`.
    #[inline]
    pub fn columns(self, x0: i32, x1: i32) -> Self {
        let nrg = Self {
            min: Point::new(self.min.x + x0, self.min.y),
            max: Point::new(self.min.x + x1, self.max.y),
        };
        self.intersect(nrg)
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

    /// Reports whether range `self` is completely contained in range `r`.
    ///
    /// Empty ranges are always considered "in" any range.
    /// Matches Go gruid's `Range.In`.
    #[inline]
    pub fn in_range(self, r: Range) -> bool {
        if self.is_empty() {
            return true;
        }
        self.intersect(r) == self
    }

    /// Intersection of two ranges (may be empty).
    ///
    /// If the two ranges do not overlap, the zero (empty) range is returned.
    #[inline]
    pub fn intersect(self, other: Range) -> Self {
        let r = Self {
            min: Point::new(self.min.x.max(other.min.x), self.min.y.max(other.min.y)),
            max: Point::new(self.max.x.min(other.max.x), self.max.y.min(other.max.y)),
        };
        if r.is_empty() { Self::default() } else { r }
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

    /// Returns a range-relative version of a [`Msg`].
    ///
    /// For mouse messages, the position is adjusted by subtracting `self.min`,
    /// making it relative to the range. All other message variants are
    /// returned unchanged.
    ///
    /// Matches Go gruid's `Range.RelMsg`.
    pub fn rel_msg(self, msg: Msg) -> Msg {
        match msg {
            Msg::Mouse {
                action,
                pos,
                modifiers,
                time,
            } => Msg::Mouse {
                action,
                pos: pos - self.min,
                modifiers,
                time,
            },
            other => other,
        }
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
    use crate::messages::{ModMask, MouseAction};
    use std::collections::HashSet;
    use std::time::Instant;

    // -----------------------------------------------------------------------
    // Point tests (unchanged)
    // -----------------------------------------------------------------------

    #[test]
    fn point_arithmetic() {
        let a = Point::new(1, 2);
        let b = Point::new(3, 4);
        assert_eq!(a + b, Point::new(4, 6));
        assert_eq!(b - a, Point::new(2, 2));
        assert_eq!(a * 3, Point::new(3, 6));
        assert_eq!(b / 2, Point::new(1, 2));
    }

    // -----------------------------------------------------------------------
    // Range basics (kept + extended)
    // -----------------------------------------------------------------------

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
    fn range_intersect_no_overlap_returns_empty() {
        let a = Range::new(0, 0, 2, 2);
        let b = Range::new(5, 5, 7, 7);
        let c = a.intersect(b);
        assert!(c.is_empty());
        // Normalized to the zero range.
        assert_eq!(c, Range::default());
    }

    #[test]
    fn empty_range_iter() {
        let r = Range::new(0, 0, 0, 0);
        assert!(r.is_empty());
        assert_eq!(r.iter().count(), 0);
    }

    // -----------------------------------------------------------------------
    // Task 1: Range::add / Range::sub — Translation
    // -----------------------------------------------------------------------

    #[test]
    fn range_add() {
        let r = Range::new(1, 2, 4, 5);
        let p = Point::new(10, 20);
        let t = r.add(p);
        assert_eq!(t.min, Point::new(11, 22));
        assert_eq!(t.max, Point::new(14, 25));
    }

    #[test]
    fn range_sub() {
        let r = Range::new(10, 20, 14, 25);
        let p = Point::new(10, 20);
        let t = r.sub(p);
        assert_eq!(t.min, Point::new(0, 0));
        assert_eq!(t.max, Point::new(4, 5));
    }

    #[test]
    fn range_add_sub_roundtrip() {
        let r = Range::new(3, 4, 8, 9);
        let p = Point::new(7, -3);
        assert_eq!(r.add(p).sub(p), r);
    }

    // -----------------------------------------------------------------------
    // Task 2: Range::in_range — Containment check
    // -----------------------------------------------------------------------

    #[test]
    fn range_in_range_contained() {
        let inner = Range::new(2, 2, 4, 4);
        let outer = Range::new(0, 0, 6, 6);
        assert!(inner.in_range(outer));
    }

    #[test]
    fn range_in_range_same() {
        let r = Range::new(0, 0, 5, 5);
        assert!(r.in_range(r));
    }

    #[test]
    fn range_in_range_not_contained() {
        let a = Range::new(0, 0, 5, 5);
        let b = Range::new(3, 3, 8, 8);
        assert!(!a.in_range(b));
    }

    #[test]
    fn range_in_range_empty_always_in() {
        let empty = Range::default();
        let r = Range::new(3, 3, 5, 5);
        assert!(empty.in_range(r));
        // Even "in" another empty range.
        assert!(empty.in_range(Range::default()));
    }

    #[test]
    fn range_in_range_empty_weird() {
        // An empty range with non-zero coordinates is still "in" any range.
        let empty = Range {
            min: Point::new(99, 99),
            max: Point::new(99, 99),
        };
        let r = Range::new(0, 0, 5, 5);
        assert!(empty.in_range(r));
    }

    // -----------------------------------------------------------------------
    // Task 3: Range::line/lines/column/columns — Relative coordinates
    // -----------------------------------------------------------------------

    #[test]
    fn range_line_relative() {
        let r = Range::new(2, 3, 6, 8); // 4 wide, 5 tall
        // line(0) = first line
        let l0 = r.line(0);
        assert_eq!(l0.min, Point::new(2, 3));
        assert_eq!(l0.max, Point::new(6, 4));
        // line(4) = last line
        let l4 = r.line(4);
        assert_eq!(l4.min, Point::new(2, 7));
        assert_eq!(l4.max, Point::new(6, 8));
    }

    #[test]
    fn range_line_out_of_bounds() {
        let r = Range::new(2, 3, 6, 8);
        assert!(r.line(-1).is_empty());
        assert!(r.line(5).is_empty());
    }

    #[test]
    fn range_lines_relative() {
        let r = Range::new(2, 3, 6, 8);
        let sub = r.lines(1, 3);
        assert_eq!(sub.min, Point::new(2, 4));
        assert_eq!(sub.max, Point::new(6, 6));
    }

    #[test]
    fn range_lines_clamped() {
        let r = Range::new(0, 0, 5, 5);
        // Request more lines than available — intersects to what exists.
        let sub = r.lines(3, 10);
        assert_eq!(sub.min, Point::new(0, 3));
        assert_eq!(sub.max, Point::new(5, 5));
    }

    #[test]
    fn range_lines_empty_oob() {
        let r = Range::new(0, 0, 5, 5);
        assert!(r.lines(5, 7).is_empty());
    }

    #[test]
    fn range_column_relative() {
        let r = Range::new(2, 3, 6, 8);
        // column(0) = first column
        let c0 = r.column(0);
        assert_eq!(c0.min, Point::new(2, 3));
        assert_eq!(c0.max, Point::new(3, 8));
        // column(3) = last column
        let c3 = r.column(3);
        assert_eq!(c3.min, Point::new(5, 3));
        assert_eq!(c3.max, Point::new(6, 8));
    }

    #[test]
    fn range_column_out_of_bounds() {
        let r = Range::new(2, 3, 6, 8);
        assert!(r.column(-1).is_empty());
        assert!(r.column(4).is_empty());
    }

    #[test]
    fn range_columns_relative() {
        let r = Range::new(2, 3, 6, 8);
        let sub = r.columns(1, 3);
        assert_eq!(sub.min, Point::new(3, 3));
        assert_eq!(sub.max, Point::new(5, 8));
    }

    #[test]
    fn range_columns_clamped() {
        let r = Range::new(0, 0, 5, 5);
        let sub = r.columns(2, 20);
        assert_eq!(sub.min, Point::new(2, 0));
        assert_eq!(sub.max, Point::new(5, 5));
    }

    #[test]
    fn range_columns_empty_oob() {
        let r = Range::new(0, 0, 5, 5);
        assert!(r.columns(5, 7).is_empty());
    }

    // -----------------------------------------------------------------------
    // Task 4: Range::shift — Empty-range safety
    // -----------------------------------------------------------------------

    #[test]
    fn range_shift_normal() {
        let r = Range::new(1, 1, 5, 5);
        let s = r.shift(1, 1, -1, -1);
        assert_eq!(s, Range::new(2, 2, 4, 4));
    }

    #[test]
    fn range_shift_returns_empty_when_result_is_empty() {
        let r = Range::new(1, 1, 3, 3);
        // Shrink beyond zero.
        let s = r.shift(5, 0, 0, 0);
        assert!(s.is_empty());
        assert_eq!(s, Range::default());
    }

    #[test]
    fn range_shift_collapses_to_empty() {
        let r = Range::new(0, 0, 4, 4);
        // Make min.x == max.x
        let s = r.shift(2, 0, -2, 0);
        assert!(s.is_empty());
        assert_eq!(s, Range::default());
    }

    // -----------------------------------------------------------------------
    // Task 5: PartialEq — Normalize empties
    // -----------------------------------------------------------------------

    #[test]
    fn empty_ranges_compare_equal() {
        let a = Range::default(); // (0,0)-(0,0)
        let b = Range {
            min: Point::new(5, 5),
            max: Point::new(5, 5),
        };
        let c = Range {
            min: Point::new(3, 0),
            max: Point::new(1, 0),
        };
        assert_eq!(a, b);
        assert_eq!(a, c);
        assert_eq!(b, c);
    }

    #[test]
    fn non_empty_ranges_compare_normally() {
        let a = Range::new(0, 0, 3, 3);
        let b = Range::new(0, 0, 3, 3);
        let c = Range::new(0, 0, 4, 4);
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn empty_range_ne_non_empty() {
        let empty = Range::default();
        let non_empty = Range::new(0, 0, 1, 1);
        assert_ne!(empty, non_empty);
    }

    #[test]
    fn empty_ranges_hash_same() {
        let a = Range::default();
        let b = Range {
            min: Point::new(5, 5),
            max: Point::new(5, 5),
        };
        let mut set = HashSet::new();
        set.insert(a);
        // b is a different empty range but should be "equal" and hash the same.
        assert!(set.contains(&b));
    }

    // -----------------------------------------------------------------------
    // Task 6: Range::rel_msg
    // -----------------------------------------------------------------------

    #[test]
    fn rel_msg_adjusts_mouse_position() {
        let r = Range::new(5, 10, 20, 30);
        let msg = Msg::Mouse {
            action: MouseAction::Main,
            pos: Point::new(7, 12),
            modifiers: ModMask::NONE,
            time: Instant::now(),
        };
        let rel = r.rel_msg(msg);
        match rel {
            Msg::Mouse { pos, .. } => {
                assert_eq!(pos, Point::new(2, 2));
            }
            _ => panic!("expected Mouse variant"),
        }
    }

    #[test]
    fn rel_msg_preserves_non_mouse() {
        let r = Range::new(5, 10, 20, 30);
        let msg = Msg::Init;
        let rel = r.rel_msg(msg);
        match rel {
            Msg::Init => {} // ok
            _ => panic!("expected Init variant"),
        }
    }

    #[test]
    fn rel_msg_preserves_key_down() {
        let r = Range::new(5, 10, 20, 30);
        let msg = Msg::key(crate::messages::Key::Enter);
        let rel = r.rel_msg(msg);
        match rel {
            Msg::KeyDown { key, .. } => {
                assert_eq!(key, crate::messages::Key::Enter);
            }
            _ => panic!("expected KeyDown variant"),
        }
    }
}

impl Add<Point> for Range {
    type Output = Range;

    #[inline]
    fn add(self, p: Point) -> Range {
        Range {
            min: self.min + p,
            max: self.max + p,
        }
    }
}

impl Sub<Point> for Range {
    type Output = Range;

    #[inline]
    fn sub(self, p: Point) -> Range {
        Range {
            min: self.min - p,
            max: self.max - p,
        }
    }
}
