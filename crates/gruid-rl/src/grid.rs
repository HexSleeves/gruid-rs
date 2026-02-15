//! An integer-cell grid for map representation.
//!
//! [`Cell`] is a newtype over `i32`, analogous to Go's `rl.Cell = int`.
//! [`Grid`] provides a 2D grid of such cells with slice semantics via
//! shared backing storage (`Rc<RefCell<...>>`).

use gruid_core::{Point, Range};
use std::cell::RefCell;
use std::rc::Rc;

/// A map cell value, wrapping an `i32`.
///
/// Different integer values represent different terrain types
/// (walls, floors, doors, etc.) as determined by the user.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Cell(pub i32);

impl Cell {
    /// Create a new cell with the given value.
    pub const fn new(value: i32) -> Self {
        Self(value)
    }

    /// Get the underlying integer value.
    pub const fn value(self) -> i32 {
        self.0
    }
}

impl From<i32> for Cell {
    fn from(v: i32) -> Self {
        Self(v)
    }
}

impl From<Cell> for i32 {
    fn from(c: Cell) -> Self {
        c.0
    }
}

/// Shared backing buffer for grid data.
#[derive(Debug, Clone)]
struct GridBuffer {
    cells: Vec<Cell>,
    /// Full buffer width (not the slice width).
    width: i32,
    /// Full buffer height (retained for completeness).
    #[allow(dead_code)]
    height: i32,
}

impl GridBuffer {
    fn index(&self, p: Point) -> usize {
        (p.y * self.width + p.x) as usize
    }
}

/// A 2D grid of [`Cell`] values with slice semantics.
///
/// Multiple `Grid` values can share the same backing buffer,
/// each viewing a different rectangular sub-region.
#[derive(Debug, Clone)]
pub struct Grid {
    buf: Rc<RefCell<GridBuffer>>,
    bounds: Range,
}

impl Grid {
    /// Create a new grid filled with `Cell(0)`.
    pub fn new(width: i32, height: i32) -> Self {
        let cells = vec![Cell::default(); (width * height) as usize];
        let buf = Rc::new(RefCell::new(GridBuffer {
            cells,
            width,
            height,
        }));
        Self {
            buf,
            bounds: Range::new(0, 0, width, height),
        }
    }

    /// Returns the bounding range of this grid view.
    pub fn bounds(&self) -> Range {
        self.bounds
    }

    /// Returns the size as a Point (width = x, height = y).
    pub fn size(&self) -> Point {
        self.bounds.size()
    }

    /// Width of this grid view.
    pub fn width(&self) -> i32 {
        self.bounds.width()
    }

    /// Height of this grid view.
    pub fn height(&self) -> i32 {
        self.bounds.height()
    }

    /// Whether this grid view contains the given point.
    pub fn contains(&self, p: Point) -> bool {
        self.bounds.contains(p)
    }

    /// Create a sub-grid view (slice) restricted to the intersection
    /// of this view's bounds and the given range.
    pub fn slice(&self, rng: Range) -> Grid {
        let bounds = self.bounds.intersect(rng);
        Grid {
            buf: Rc::clone(&self.buf),
            bounds,
        }
    }

    /// Get the cell at a point, or `None` if out of bounds.
    pub fn at(&self, p: Point) -> Option<Cell> {
        if !self.bounds.contains(p) {
            return None;
        }
        let buf = self.buf.borrow();
        let idx = buf.index(p);
        Some(buf.cells[idx])
    }

    /// Set the cell at a point. Does nothing if out of bounds.
    pub fn set(&self, p: Point, cell: Cell) {
        if !self.bounds.contains(p) {
            return;
        }
        let mut buf = self.buf.borrow_mut();
        let idx = buf.index(p);
        buf.cells[idx] = cell;
    }

    /// Fill the entire grid view with the given cell.
    pub fn fill(&self, cell: Cell) {
        let mut buf = self.buf.borrow_mut();
        for p in self.bounds.iter() {
            let idx = buf.index(p);
            buf.cells[idx] = cell;
        }
    }

    /// Fill the grid view using a function that takes each point.
    pub fn fill_fn(&self, mut f: impl FnMut(Point) -> Cell) {
        let mut buf = self.buf.borrow_mut();
        for p in self.bounds.iter() {
            let idx = buf.index(p);
            buf.cells[idx] = f(p);
        }
    }

    /// Apply a transformation to every cell in the grid view.
    pub fn map_cells(&self, mut f: impl FnMut(Point, Cell) -> Cell) {
        let mut buf = self.buf.borrow_mut();
        for p in self.bounds.iter() {
            let idx = buf.index(p);
            buf.cells[idx] = f(p, buf.cells[idx]);
        }
    }

    /// Copy cells from `other` into `self`. Only the overlapping region
    /// (by coordinate) is copied.
    pub fn copy_from(&self, other: &Grid) {
        let overlap = self.bounds.intersect(other.bounds);
        if overlap.is_empty() {
            return;
        }
        // We need to borrow both buffers. If they are the same Rc, skip
        // (self-copy within same buffer is a no-op for identical ranges).
        if Rc::ptr_eq(&self.buf, &other.buf) {
            return;
        }
        let src = other.buf.borrow();
        let mut dst = self.buf.borrow_mut();
        for p in overlap.iter() {
            let si = src.index(p);
            let di = dst.index(p);
            dst.cells[di] = src.cells[si];
        }
    }

    /// Count how many cells in the view equal the given cell.
    pub fn count(&self, cell: Cell) -> usize {
        let buf = self.buf.borrow();
        let mut n = 0;
        for p in self.bounds.iter() {
            let idx = buf.index(p);
            if buf.cells[idx] == cell {
                n += 1;
            }
        }
        n
    }

    /// Count how many cells satisfy a predicate.
    pub fn count_fn(&self, mut f: impl FnMut(Point, Cell) -> bool) -> usize {
        let buf = self.buf.borrow();
        let mut n = 0;
        for p in self.bounds.iter() {
            let idx = buf.index(p);
            if f(p, buf.cells[idx]) {
                n += 1;
            }
        }
        n
    }

    /// Iterate over `(Point, Cell)` pairs in row-major order.
    pub fn iter(&self) -> GridIter {
        let buf = self.buf.borrow();
        // Collect a snapshot so we don't hold the borrow across the iterator.
        let items: Vec<(Point, Cell)> = self
            .bounds
            .iter()
            .map(|p| {
                let idx = buf.index(p);
                (p, buf.cells[idx])
            })
            .collect();
        GridIter {
            items,
            pos: 0,
        }
    }
}

/// Iterator over (Point, Cell) pairs of a Grid snapshot.
pub struct GridIter {
    items: Vec<(Point, Cell)>,
    pos: usize,
}

impl Iterator for GridIter {
    type Item = (Point, Cell);

    fn next(&mut self) -> Option<Self::Item> {
        if self.pos < self.items.len() {
            let item = self.items[self.pos];
            self.pos += 1;
            Some(item)
        } else {
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.items.len() - self.pos;
        (remaining, Some(remaining))
    }
}

impl ExactSizeIterator for GridIter {}

impl IntoIterator for &Grid {
    type Item = (Point, Cell);
    type IntoIter = GridIter;

    fn into_iter(self) -> GridIter {
        self.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_and_size() {
        let g = Grid::new(10, 5);
        assert_eq!(g.size(), Point::new(10, 5));
        assert_eq!(g.width(), 10);
        assert_eq!(g.height(), 5);
    }

    #[test]
    fn test_set_and_at() {
        let g = Grid::new(4, 4);
        let p = Point::new(2, 3);
        g.set(p, Cell(42));
        assert_eq!(g.at(p), Some(Cell(42)));
        assert_eq!(g.at(Point::new(0, 0)), Some(Cell(0)));
        assert_eq!(g.at(Point::new(10, 10)), None);
    }

    #[test]
    fn test_slice_shares_buffer() {
        let g = Grid::new(10, 10);
        let s = g.slice(Range::new(2, 2, 5, 5));
        g.set(Point::new(3, 3), Cell(99));
        assert_eq!(s.at(Point::new(3, 3)), Some(Cell(99)));
    }

    #[test]
    fn test_fill_and_count() {
        let g = Grid::new(5, 5);
        g.fill(Cell(1));
        assert_eq!(g.count(Cell(1)), 25);
        g.set(Point::new(0, 0), Cell(2));
        assert_eq!(g.count(Cell(1)), 24);
        assert_eq!(g.count(Cell(2)), 1);
    }

    #[test]
    fn test_iter() {
        let g = Grid::new(3, 2);
        g.set(Point::new(1, 0), Cell(5));
        let items: Vec<_> = g.iter().collect();
        assert_eq!(items.len(), 6);
        assert_eq!(items[1], (Point::new(1, 0), Cell(5)));
    }
}
