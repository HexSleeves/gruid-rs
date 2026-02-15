//! The [`Grid`] type — a 2D grid of [`Cell`]s with slice semantics.
//!
//! A `Grid` is a *view* into a shared backing buffer. Cloning a `Grid` yields
//! another view of the **same** storage (like Go’s slices). Use [`slice`](Grid::slice)
//! to obtain sub-grid views.

use std::cell::RefCell;
use std::rc::Rc;

use crate::cell::Cell;
use crate::geom::{Point, Range};

// ---------------------------------------------------------------------------
// Internal shared buffer
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
struct GridBuffer {
    cells: Vec<Cell>,
    width: usize,
    height: usize,
}

impl GridBuffer {
    fn new(width: usize, height: usize) -> Self {
        Self {
            cells: vec![Cell::default(); width * height],
            width,
            height,
        }
    }

    #[inline]
    fn index(&self, x: i32, y: i32) -> Option<usize> {
        if x >= 0 && y >= 0 && (x as usize) < self.width && (y as usize) < self.height {
            Some((y as usize) * self.width + (x as usize))
        } else {
            None
        }
    }
}

// ---------------------------------------------------------------------------
// Grid
// ---------------------------------------------------------------------------

/// A 2D grid of [`Cell`]s backed by shared storage.
///
/// Cloning produces another view into the same buffer (slice semantics).
#[derive(Debug, Clone)]
pub struct Grid {
    buffer: Rc<RefCell<GridBuffer>>,
    bounds: Range,
}

impl Grid {
    /// Create a new grid of the given dimensions, filled with default cells.
    pub fn new(width: i32, height: i32) -> Self {
        let w = width.max(0) as usize;
        let h = height.max(0) as usize;
        Self {
            buffer: Rc::new(RefCell::new(GridBuffer::new(w, h))),
            bounds: Range::new(0, 0, width.max(0), height.max(0)),
        }
    }

    /// The bounding range of this grid / sub-grid.
    #[inline]
    pub fn bounds(&self) -> Range {
        self.bounds
    }

    /// Alias for [`bounds`](Grid::bounds).
    #[inline]
    pub fn range_(&self) -> Range {
        self.bounds
    }

    /// Size of the grid as a `Point`.
    #[inline]
    pub fn size(&self) -> Point {
        self.bounds.size()
    }

    /// Width.
    #[inline]
    pub fn width(&self) -> i32 {
        self.bounds.width()
    }

    /// Height.
    #[inline]
    pub fn height(&self) -> i32 {
        self.bounds.height()
    }

    /// Whether `p` is inside this grid’s bounds.
    #[inline]
    pub fn contains(&self, p: Point) -> bool {
        self.bounds.contains(p)
    }

    /// Get a sub-grid view. The returned `Grid` shares the same backing
    /// buffer but has narrower bounds (the intersection of the requested
    /// range and the current bounds).
    pub fn slice(&self, r: Range) -> Grid {
        Grid {
            buffer: Rc::clone(&self.buffer),
            bounds: self.bounds.intersect(r),
        }
    }

    /// Read the cell at `p`. Returns `Cell::default()` if `p` is outside
    /// bounds.
    pub fn at(&self, p: Point) -> Cell {
        if !self.bounds.contains(p) {
            return Cell::default();
        }
        let buf = self.buffer.borrow();
        buf.index(p.x, p.y)
            .map(|i| buf.cells[i])
            .unwrap_or_default()
    }

    /// Set the cell at `p`. No-op if `p` is outside bounds.
    pub fn set(&self, p: Point, cell: Cell) {
        if !self.bounds.contains(p) {
            return;
        }
        let mut buf = self.buffer.borrow_mut();
        if let Some(i) = buf.index(p.x, p.y) {
            buf.cells[i] = cell;
        }
    }

    /// Fill every cell in the grid with `cell`.
    pub fn fill(&self, cell: Cell) {
        let mut buf = self.buffer.borrow_mut();
        for p in self.bounds.iter() {
            if let Some(i) = buf.index(p.x, p.y) {
                buf.cells[i] = cell;
            }
        }
    }

    /// Apply `f` to every cell in the grid, replacing each with the return
    /// value.
    pub fn map_cells<F: Fn(Point, Cell) -> Cell>(&self, f: F) {
        let mut buf = self.buffer.borrow_mut();
        for p in self.bounds.iter() {
            if let Some(i) = buf.index(p.x, p.y) {
                buf.cells[i] = f(p, buf.cells[i]);
            }
        }
    }

    /// Copy cells from `src` into `self`, aligning `src.bounds.min` with
    /// `self.bounds.min`. Returns the size actually copied.
    pub fn copy_from(&self, src: &Grid) -> Point {
        let sw = src.bounds.width().min(self.bounds.width());
        let sh = src.bounds.height().min(self.bounds.height());
        let src_buf = src.buffer.borrow();
        let mut dst_buf = self.buffer.borrow_mut();
        for dy in 0..sh {
            for dx in 0..sw {
                let sp = Point::new(src.bounds.min.x + dx, src.bounds.min.y + dy);
                let dp = Point::new(self.bounds.min.x + dx, self.bounds.min.y + dy);
                if let (Some(si), Some(di)) =
                    (src_buf.index(sp.x, sp.y), dst_buf.index(dp.x, dp.y))
                {
                    dst_buf.cells[di] = src_buf.cells[si];
                }
            }
        }
        Point::new(sw, sh)
    }

    /// Row-major iterator over `(Point, Cell)` pairs.
    pub fn iter(&self) -> GridIter<'_> {
        GridIter {
            grid: self,
            inner: self.bounds.iter(),
        }
    }
}

// ---------------------------------------------------------------------------
// GridIter
// ---------------------------------------------------------------------------

/// Iterator over `(Point, Cell)` pairs in a [`Grid`].
pub struct GridIter<'a> {
    grid: &'a Grid,
    inner: crate::geom::RangeIter,
}

impl<'a> Iterator for GridIter<'a> {
    type Item = (Point, Cell);

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let p = self.inner.next()?;
        Some((p, self.grid.at(p)))
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

// ---------------------------------------------------------------------------
// Frame / FrameCell / compute_frame
// ---------------------------------------------------------------------------

/// A single cell that changed between frames.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FrameCell {
    pub cell: Cell,
    pub pos: Point,
}

/// A set of cell changes (a diff frame).
#[derive(Clone, Debug, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Frame {
    pub cells: Vec<FrameCell>,
    pub width: i32,
    pub height: i32,
}

/// Compute the difference between two same-sized grids.
///
/// Returns a [`Frame`] containing only the cells that differ.
pub fn compute_frame(prev: &Grid, curr: &Grid) -> Frame {
    let bounds = curr.bounds();
    let mut cells = Vec::new();
    for p in bounds.iter() {
        let pc = prev.at(p);
        let cc = curr.at(p);
        if pc != cc {
            cells.push(FrameCell { cell: cc, pos: p });
        }
    }
    Frame {
        cells,
        width: bounds.width(),
        height: bounds.height(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grid_new_and_at() {
        let g = Grid::new(4, 3);
        assert_eq!(g.size(), Point::new(4, 3));
        assert_eq!(g.at(Point::new(0, 0)), Cell::default());
    }

    #[test]
    fn grid_set_and_get() {
        let g = Grid::new(4, 3);
        let c = Cell::default().with_char('X');
        g.set(Point::new(2, 1), c);
        assert_eq!(g.at(Point::new(2, 1)).ch, 'X');
        // out of bounds returns default
        assert_eq!(g.at(Point::new(10, 10)), Cell::default());
    }

    #[test]
    fn grid_slice_shares_buffer() {
        let g = Grid::new(4, 3);
        let s = g.slice(Range::new(1, 1, 3, 3));
        let c = Cell::default().with_char('#');
        s.set(Point::new(1, 1), c);
        assert_eq!(g.at(Point::new(1, 1)).ch, '#');
    }

    #[test]
    fn grid_fill() {
        let g = Grid::new(3, 2);
        let c = Cell::default().with_char('.');
        g.fill(c);
        for (_, cell) in g.iter() {
            assert_eq!(cell.ch, '.');
        }
    }

    #[test]
    fn compute_frame_diff() {
        let a = Grid::new(3, 2);
        let b = Grid::new(3, 2);
        b.set(Point::new(1, 0), Cell::default().with_char('A'));
        let frame = compute_frame(&a, &b);
        assert_eq!(frame.cells.len(), 1);
        assert_eq!(frame.cells[0].pos, Point::new(1, 0));
        assert_eq!(frame.cells[0].cell.ch, 'A');
    }
}
