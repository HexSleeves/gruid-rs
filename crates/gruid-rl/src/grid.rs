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

    /// Iterate all cells, calling `f` with the relative position and a
    /// mutable reference to each cell.
    ///
    /// This is the mutable-iteration counterpart of `iter()`, matching
    /// Go's `GridIterator.SetCell` pattern.
    pub fn for_each_mut(&self, mut f: impl FnMut(Point, &mut Cell)) {
        let mut buf = self.buf.borrow_mut();
        let min = self.bounds.min;
        for abs_p in self.bounds.iter() {
            if let Some(idx) = buf.index(abs_p.x, abs_p.y) {
                let rel = Point::new(abs_p.x - min.x, abs_p.y - min.y);
                f(rel, &mut buf.cells[idx]);
            }
        }
    }

    /// Replace each cell with the return value of `f`, which receives the
    /// relative position and the current cell value.
    ///
    /// This is equivalent to `map_cells` but named to make the mutation
    /// explicit.
    pub fn map_cells_mut(&self, mut f: impl FnMut(Point, Cell) -> Cell) {
        let mut buf = self.buf.borrow_mut();
        let min = self.bounds.min;
        for abs_p in self.bounds.iter() {
            if let Some(idx) = buf.index(abs_p.x, abs_p.y) {
                let rel = Point::new(abs_p.x - min.x, abs_p.y - min.y);
                buf.cells[idx] = f(rel, buf.cells[idx]);
            }
        }
    }

    /// Return the cell at relative position `p` without checking grid-slice
    /// bounds.
    ///
    /// If `p` is outside this grid slice but within the underlying buffer,
    /// the corresponding underlying cell is returned.  If also outside the
    /// underlying buffer, returns `Cell(0)`.
    ///
    /// This matches Go's `Grid.AtU`.
    ///
    /// # Safety note
    ///
    /// This method is safe (no `unsafe` code) but can return surprising
    /// values for out-of-slice positions.  Prefer [`at`](Self::at) unless
    /// you need the performance in a tight loop.
    pub fn at_unchecked(&self, p: Point) -> Cell {
        let q = Point::new(p.x + self.bounds.min.x, p.y + self.bounds.min.y);
        let buf = self.buf.borrow();
        let i = q.y as isize * buf.width as isize + q.x as isize;
        if i < 0 || i as usize >= buf.cells.len() {
            Cell(0)
        } else {
            buf.cells[i as usize]
        }
    }

    /// Resize the grid to new dimensions, preserving existing content.
    ///
    /// If the new size is larger, the underlying buffer grows and new cells
    /// are initialised to `Cell(0)`.  If the new size is smaller (or zero),
    /// the grid view shrinks.  Matches Go's `Grid.Resize`.
    pub fn resize(&mut self, w: i32, h: i32) {
        let cur = self.size();
        if cur.x == w && cur.y == h {
            return;
        }
        if w <= 0 || h <= 0 {
            self.bounds.max = self.bounds.min;
            return;
        }
        // Update the view's max bound.
        self.bounds.max = Point::new(self.bounds.min.x + w, self.bounds.min.y + h);

        let mut buf = self.buf.borrow_mut();
        let old_w = buf.width;
        let old_h = buf.height;
        let need_w = self.bounds.max.x;
        let need_h = self.bounds.max.y;

        if need_w > old_w || need_h > old_h {
            let nw = need_w.max(old_w);
            let nh = need_h.max(old_h);
            let mut new_cells = vec![Cell::default(); (nw * nh) as usize];
            // Copy old data row by row.
            for y in 0..old_h {
                let src_start = (y * old_w) as usize;
                let dst_start = (y * nw) as usize;
                let row_len = old_w as usize;
                new_cells[dst_start..dst_start + row_len]
                    .copy_from_slice(&buf.cells[src_start..src_start + row_len]);
            }
            buf.cells = new_cells;
            buf.width = nw;
            buf.height = nh;
        }
    }

    /// Copy cells from `other` into `self`, aligning origins.
    ///
    /// Returns the size of the copied region as a `Point`, which is the
    /// component-wise minimum of both grids' dimensions (matching Go's
    /// `Grid.Copy` return value).
    pub fn copy_from(&self, other: &Grid) -> Point {
        let sw = other.bounds.width().min(self.bounds.width());
        let sh = other.bounds.height().min(self.bounds.height());
        let copied = Point::new(sw, sh);
        if Rc::ptr_eq(&self.buf, &other.buf) && self.bounds == other.bounds {
            return self.bounds.size();
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
        copied
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

    #[test]
    fn test_for_each_mut() {
        let g = Grid::new(3, 3);
        g.fill(Cell(1));
        g.for_each_mut(|p, cell| {
            *cell = Cell(p.x + p.y);
        });
        assert_eq!(g.at(Point::new(0, 0)), Some(Cell(0)));
        assert_eq!(g.at(Point::new(2, 1)), Some(Cell(3)));
        assert_eq!(g.at(Point::new(1, 2)), Some(Cell(3)));
        assert_eq!(g.at(Point::new(2, 2)), Some(Cell(4)));
    }

    #[test]
    fn test_for_each_mut_on_slice() {
        let g = Grid::new(10, 10);
        g.fill(Cell(0));
        let s = g.slice(Range::new(2, 2, 5, 5));
        s.for_each_mut(|_p, cell| {
            *cell = Cell(cell.0 + 10);
        });
        // Inside slice: was 0, now 10.
        assert_eq!(g.at(Point::new(3, 3)), Some(Cell(10)));
        // Outside slice: unchanged.
        assert_eq!(g.at(Point::new(0, 0)), Some(Cell(0)));
    }

    #[test]
    fn test_map_cells_mut() {
        let g = Grid::new(4, 4);
        g.fill(Cell(1));
        g.map_cells_mut(|p, c| Cell(c.0 + p.x * 10));
        assert_eq!(g.at(Point::new(0, 0)), Some(Cell(1)));
        assert_eq!(g.at(Point::new(3, 0)), Some(Cell(31)));
        assert_eq!(g.at(Point::new(2, 3)), Some(Cell(21)));
    }

    #[test]
    fn test_at_unchecked() {
        let g = Grid::new(5, 5);
        g.fill(Cell(7));
        // Normal in-bounds access.
        assert_eq!(g.at_unchecked(Point::new(2, 2)), Cell(7));
        // Completely out of underlying buffer returns Cell(0).
        assert_eq!(g.at_unchecked(Point::new(100, 100)), Cell(0));
        assert_eq!(g.at_unchecked(Point::new(-1, -1)), Cell(0));
    }

    #[test]
    fn test_at_unchecked_skips_slice_bounds() {
        let g = Grid::new(10, 10);
        g.set(Point::new(0, 0), Cell(42));
        let s = g.slice(Range::new(5, 5, 8, 8));
        // Relative (-5, -5) is outside the slice, but at_unchecked
        // skips slice bounds and hits absolute (0, 0).
        assert_eq!(s.at_unchecked(Point::new(-5, -5)), Cell(42));
        // Normal at() would return None for the same point.
        assert_eq!(s.at(Point::new(-5, -5)), None);
    }

    #[test]
    fn test_resize_same_size() {
        let mut g = Grid::new(10, 10);
        g.fill(Cell(5));
        g.resize(10, 10);
        assert_eq!(g.size(), Point::new(10, 10));
        assert_eq!(g.at(Point::new(5, 5)), Some(Cell(5)));
    }

    #[test]
    fn test_resize_grow() {
        let mut g = Grid::new(5, 5);
        g.fill(Cell(3));
        g.resize(10, 10);
        assert_eq!(g.size(), Point::new(10, 10));
        // Old content preserved.
        assert_eq!(g.at(Point::new(2, 2)), Some(Cell(3)));
        // New area is zero.
        assert_eq!(g.at(Point::new(7, 7)), Some(Cell(0)));
    }

    #[test]
    fn test_resize_shrink_to_zero() {
        let mut g = Grid::new(10, 10);
        g.resize(-1, 5);
        assert_eq!(g.size(), Point::new(0, 0));
    }

    #[test]
    fn test_resize_preserves_content() {
        // Matches Go TestResize behavior.
        let mut g = Grid::new(20, 10);
        g.fill(Cell(2));
        g.resize(20, 30);
        assert_eq!(g.at_unchecked(Point::new(10, 5)), Cell(2));
        assert_eq!(g.at_unchecked(Point::new(10, 25)), Cell(0));
    }

    #[test]
    fn test_copy_from_returns_size() {
        let g1 = Grid::new(10, 10);
        let g2 = Grid::new(5, 8);
        let copied = g1.copy_from(&g2);
        assert_eq!(copied, Point::new(5, 8));

        let g3 = Grid::new(3, 3);
        let copied2 = g3.copy_from(&g1);
        assert_eq!(copied2, Point::new(3, 3));
    }

    #[test]
    fn test_copy_from_same_grid_returns_size() {
        let g = Grid::new(10, 10);
        let copied = g.copy_from(&g);
        assert_eq!(copied, Point::new(10, 10));
    }
}
