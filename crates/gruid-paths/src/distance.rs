use gruid_core::Point;

/// Manhattan (L1) distance between two points.
#[inline]
pub fn manhattan(a: Point, b: Point) -> i32 {
    (a.x - b.x).abs() + (a.y - b.y).abs()
}

/// Chebyshev (L∞) distance between two points.
#[inline]
pub fn chebyshev(a: Point, b: Point) -> i32 {
    (a.x - b.x).abs().max((a.y - b.y).abs())
}
