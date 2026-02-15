//! An integer-cell grid for map representation.
//!
//! [`Cell`] is a newtype over `i32`, analogous to Go's `rl.Cell = int`.
//! [`Grid`] provides a 2D grid of such cells with slice semantics via
//! shared backing storage (`Rc<RefCell<...>>`).
//!
//! All public methods use **relative** coordinates (0-based within the grid
//! view), matching Go gruid's semantics.

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
    /// Full buffer height.
    #[allow(dead_code)]
    height: i32,
}

impl GridBuffer {
    fn index(&self, x: i32, y: i32) -> Option<usize> {
        if x >= 0 && y >= 0 && x < self.width && y < self.height {
            Some((y * self.width + x) as usize)
        } else {
            None
        }
    }
}

/// A 2D grid of [`Cell`] values with slice semantics.
///
/// Multiple `Grid` values can share the same backing buffer,
/// each viewing a different rectangular sub-region.
/// All position arguments are **relative** to this grid view's origin.
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

    /// Returns the bounding range (absolute coords in underlying buffer).
    pub fn bounds(&self) -> Range {
        self.bounds
    }

    /// Returns a relative range: min at (0,0), max at size.
    pub fn range_(&self) -> Range {
        Range::new(0, 0, self.bounds.width(), self.bounds.height())
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

    /// Whether relative point `p` is inside this grid.
    pub fn contains(&self, p: Point) -> bool {
        let q = Point::new(p.x + self.bounds.min.x, p.y + self.bounds.min.y);
        self.bounds.contains(q)
    }

    /// Create a sub-grid view. `rng` is a **relative** range within this grid.
    /// Clamped to this grid's size.
    pub fn slice(&self, rng: Range) -> Grid {
        let max = self.size();
        let min_x = rng.min.x.max(0);
        let min_y = rng.min.y.max(0);
        let max_x = rng.max.x.min(max.x);
        let max_y = rng.max.y.min(max.y);
        let abs_min = Point::new(min_x + self.bounds.min.x, min_y + self.bounds.min.y);
        let abs_max = Point::new(max_x + self.bounds.min.x, max_y + self.bounds.min.y);
        Grid {
            buf: Rc::clone(&self.buf),
            bounds: Range {
                min: abs_min,
                max: abs_max,
            },
        }
    }

    /// Get the cell at relative position `p`, or `None` if out of bounds.
    pub fn at(&self, p: Point) -> Option<Cell> {
        let q = Point::new(p.x + self.bounds.min.x, p.y + self.bounds.min.y);
        if !self.bounds.contains(q) {
            return None;
        }
        let buf = self.buf.borrow();
        buf.index(q.x, q.y).map(|idx| buf.cells[idx])
    }

    /// Set the cell at relative position `p`. Does nothing if out of bounds.
    pub fn set(&self, p: Point, cell: Cell) {
        let q = Point::new(p.x + self.bounds.min.x, p.y + self.bounds.min.y);
        if !self.bounds.contains(q) {
            return;
        }
        let mut buf = self.buf.borrow_mut();
        if let Some(idx) = buf.index(q.x, q.y) {
            buf.cells[idx] = cell;
        }
    }

    /// Fill the entire grid view with the given cell.
    pub fn fill(&self, cell: Cell) {
        let mut buf = self.buf.borrow_mut();
        for abs_p in self.bounds.iter() {
            if let Some(idx) = buf.index(abs_p.x, abs_p.y) {
                buf.cells[idx] = cell;
            }
        }
    }

    /// Fill the grid view using a function (no arguments).
    pub fn fill_fn(&self, mut f: impl FnMut() -> Cell) {
        let mut buf = self.buf.borrow_mut();
        for abs_p in self.bounds.iter() {
            if let Some(idx) = buf.index(abs_p.x, abs_p.y) {
                buf.cells[idx] = f();
            }
        }
    }

    /// Apply a transformation to every cell. Callback receives **relative**
    /// coordinates.
    pub fn map_cells(&self, mut f: impl FnMut(Point, Cell) -> Cell) {
        let mut buf = self.buf.borrow_mut();
        let min = self.bounds.min;
        for abs_p in self.bounds.iter() {
            if let Some(idx) = buf.index(abs_p.x, abs_p.y) {
                let rel = Point::new(abs_p.x - min.x, abs_p.y - min.y);
                buf.cells[idx] = f(rel, buf.cells[idx]);
            }
        }
    }

    /// Copy cells from `other` into `self`, aligning origins.
    pub fn copy_from(&self, other: &Grid) {
        let sw = other.bounds.width().min(self.bounds.width());
        let sh = other.bounds.height().min(self.bounds.height());
        if Rc::ptr_eq(&self.buf, &other.buf) && self.bounds == other.bounds {
            return;
        }
        if Rc::ptr_eq(&self.buf, &other.buf) {
            // Same underlying buffer but different slices - need temp copy.
            let buf = self.buf.borrow();
            let mut tmp = Vec::with_capacity((sw * sh) as usize);
            for dy in 0..sh {
                for dx in 0..sw {
                    let sp = Point::new(other.bounds.min.x + dx, other.bounds.min.y + dy);
                    if let Some(si) = buf.index(sp.x, sp.y) {
                        tmp.push(buf.cells[si]);
                    }
                }
            }
            drop(buf);
            let mut buf = self.buf.borrow_mut();
            let mut ti = 0;
            for dy in 0..sh {
                for dx in 0..sw {
                    let dp = Point::new(self.bounds.min.x + dx, self.bounds.min.y + dy);
                    if let Some(di) = buf.index(dp.x, dp.y) {
                        buf.cells[di] = tmp[ti];
                    }
                    ti += 1;
                }
            }
        } else {
            let src = other.buf.borrow();
            let mut dst = self.buf.borrow_mut();
            for dy in 0..sh {
                for dx in 0..sw {
                    let sp = Point::new(other.bounds.min.x + dx, other.bounds.min.y + dy);
                    let dp = Point::new(self.bounds.min.x + dx, self.bounds.min.y + dy);
                    if let (Some(si), Some(di)) = (src.index(sp.x, sp.y), dst.index(dp.x, dp.y)) {
                        dst.cells[di] = src.cells[si];
                    }
                }
            }
        }
    }

    /// Count how many cells in the view equal the given cell.
    pub fn count(&self, cell: Cell) -> usize {
        let buf = self.buf.borrow();
        let mut n = 0;
        for abs_p in self.bounds.iter() {
            if let Some(idx) = buf.index(abs_p.x, abs_p.y) {
                if buf.cells[idx] == cell {
                    n += 1;
                }
            }
        }
        n
    }

    /// Count how many cells satisfy a predicate. Callback receives **relative** coords.
    pub fn count_fn(&self, mut f: impl FnMut(Point, Cell) -> bool) -> usize {
        let buf = self.buf.borrow();
        let min = self.bounds.min;
        let mut n = 0;
        for abs_p in self.bounds.iter() {
            if let Some(idx) = buf.index(abs_p.x, abs_p.y) {
                let rel = Point::new(abs_p.x - min.x, abs_p.y - min.y);
                if f(rel, buf.cells[idx]) {
                    n += 1;
                }
            }
        }
        n
    }

    /// Iterate over `(Point, Cell)` pairs in row-major order with **relative** coords.
    pub fn iter(&self) -> GridIter {
        let buf = self.buf.borrow();
        let min = self.bounds.min;
        let items: Vec<(Point, Cell)> = self
            .bounds
            .iter()
            .map(|abs_p| {
                let rel = Point::new(abs_p.x - min.x, abs_p.y - min.y);
                let idx = (abs_p.y * buf.width + abs_p.x) as usize;
                (rel, buf.cells[idx])
            })
            .collect();
        GridIter { items, pos: 0 }
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
    fn test_slice_relative_coords() {
        let g = Grid::new(10, 10);
        g.set(Point::new(5, 5), Cell(99));
        let s = g.slice(Range::new(3, 3, 8, 8));
        // relative (2,2) in slice = absolute (5,5)
        assert_eq!(s.at(Point::new(2, 2)), Some(Cell(99)));
        // set at relative (0,0) in slice = absolute (3,3)
        s.set(Point::new(0, 0), Cell(77));
        assert_eq!(g.at(Point::new(3, 3)), Some(Cell(77)));
    }

    #[test]
    fn test_slice_shares_buffer() {
        let g = Grid::new(10, 10);
        let s = g.slice(Range::new(2, 2, 5, 5));
        // Set in parent, read from slice
        g.set(Point::new(3, 3), Cell(99));
        // relative (1,1) in slice = absolute (3,3)
        assert_eq!(s.at(Point::new(1, 1)), Some(Cell(99)));
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
    fn test_iter_relative() {
        let g = Grid::new(3, 2);
        g.set(Point::new(1, 0), Cell(5));
        let items: Vec<_> = g.iter().collect();
        assert_eq!(items.len(), 6);
        assert_eq!(items[0].0, Point::new(0, 0));
        assert_eq!(items[1], (Point::new(1, 0), Cell(5)));
    }

    #[test]
    fn test_nested_slice() {
        let g = Grid::new(20, 20);
        let s1 = g.slice(Range::new(5, 5, 15, 15));
        let s2 = s1.slice(Range::new(2, 2, 5, 5));
        s2.set(Point::new(0, 0), Cell(42));
        // abs (7,7)
        assert_eq!(g.at(Point::new(7, 7)), Some(Cell(42)));
        assert_eq!(s1.at(Point::new(2, 2)), Some(Cell(42)));
        assert_eq!(s2.at(Point::new(0, 0)), Some(Cell(42)));
    }

    #[test]
    fn test_contains_relative() {
        let g = Grid::new(10, 10);
        let s = g.slice(Range::new(5, 5, 8, 8));
        assert!(s.contains(Point::new(0, 0)));
        assert!(s.contains(Point::new(2, 2)));
        assert!(!s.contains(Point::new(3, 0)));
        assert!(!s.contains(Point::new(-1, 0)));
    }
}
