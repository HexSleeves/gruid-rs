use gruid_core::Point;

/// Minimal pathfinding interface — provides neighbor enumeration.
pub trait Pather {
    /// Append neighbors of `p` into `buf`. The caller clears `buf` before calling.
    fn neighbors(&self, p: Point, buf: &mut Vec<Point>);
}

/// Pather with weighted (positive-cost) edges.
pub trait WeightedPather: Pather {
    /// Cost of moving from `from` to adjacent `to`. Must be > 0.
    fn cost(&self, from: Point, to: Point) -> i32;
}

/// Full A* pather with an admissible heuristic.
pub trait AstarPather: WeightedPather {
    /// Heuristic estimate of distance from `from` to `to`.
    /// Must never overestimate the true cost (admissible).
    fn estimate(&self, from: Point, to: Point) -> i32;
}
