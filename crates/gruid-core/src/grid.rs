//! The [`Grid`] type — a 2D grid of [`Cell`]s with slice semantics.
//!
//! A `Grid` is a *view* into a shared backing buffer. Cloning a `Grid` yields
//! another view of the **same** storage (like Go's slices). Use [`slice`](Grid::slice)
//! to obtain sub-grid views.
//!
//! All public methods use **relative** coordinates (0-based within the grid
//! view), matching Go gruid's semantics. After `grid.slice(Range::new(5,5,10,10))`,
//! `grid.set(Point::new(0,0), c)` writes to position (5,5) in the underlying buffer.

use std::cell::RefCell;
use std::fmt;
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
/// All position arguments are **relative** to this grid view's origin.
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

    /// The bounding range of this grid slice within the underlying buffer
    /// (absolute coordinates).
    #[inline]
    pub fn bounds(&self) -> Range {
        self.bounds
    }

    /// Convenience range with min at (0,0) and max at size.
    #[inline]
    pub fn range_(&self) -> Range {
        Range::new(0, 0, self.bounds.width(), self.bounds.height())
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

    /// Whether relative point `p` is inside this grid.
    #[inline]
    pub fn contains(&self, p: Point) -> bool {
        let q = Point::new(p.x + self.bounds.min.x, p.y + self.bounds.min.y);
        self.bounds.contains(q)
    }

    /// Get a sub-grid view. `rg` is a **relative** range within this grid.
    /// The returned `Grid` shares the same backing buffer.
    ///
    /// Like Go's `Grid.Slice`, the range is clamped to this grid's size.
    pub fn slice(&self, rg: Range) -> Grid {
        let max = self.size();
        let min_x = rg.min.x.max(0);
        let min_y = rg.min.y.max(0);
        let max_x = rg.max.x.min(max.x);
        let max_y = rg.max.y.min(max.y);
        // Offset to absolute coords in the underlying buffer.
        let abs_min = Point::new(min_x + self.bounds.min.x, min_y + self.bounds.min.y);
        let abs_max = Point::new(max_x + self.bounds.min.x, max_y + self.bounds.min.y);
        Grid {
            buffer: Rc::clone(&self.buffer),
            bounds: Range {
                min: abs_min,
                max: abs_max,
            },
        }
    }

    /// Read the cell at relative position `p`. Returns `Cell::default()` if
    /// `p` is outside bounds.
    pub fn at(&self, p: Point) -> Cell {
        let q = Point::new(p.x + self.bounds.min.x, p.y + self.bounds.min.y);
        if !self.bounds.contains(q) {
            return Cell::default();
        }
        let buf = self.buffer.borrow();
        buf.index(q.x, q.y)
            .map(|i| buf.cells[i])
            .unwrap_or_default()
    }

    /// Set the cell at relative position `p`. No-op if `p` is outside bounds.
    pub fn set(&self, p: Point, cell: Cell) {
        let q = Point::new(p.x + self.bounds.min.x, p.y + self.bounds.min.y);
        if !self.bounds.contains(q) {
            return;
        }
        let mut buf = self.buffer.borrow_mut();
        if let Some(i) = buf.index(q.x, q.y) {
            buf.cells[i] = cell;
        }
    }

    /// Fill every cell in the grid with `cell`.
    pub fn fill(&self, cell: Cell) {
        let mut buf = self.buffer.borrow_mut();
        for abs_p in self.bounds.iter() {
            if let Some(i) = buf.index(abs_p.x, abs_p.y) {
                buf.cells[i] = cell;
            }
        }
    }

    /// Apply `f` to every cell in the grid, replacing each with the return
    /// value. The callback receives **relative** coordinates.
    pub fn map_cells<F: Fn(Point, Cell) -> Cell>(&self, f: F) {
        let mut buf = self.buffer.borrow_mut();
        let min = self.bounds.min;
        for abs_p in self.bounds.iter() {
            if let Some(i) = buf.index(abs_p.x, abs_p.y) {
                let rel = Point::new(abs_p.x - min.x, abs_p.y - min.y);
                buf.cells[i] = f(rel, buf.cells[i]);
            }
        }
    }

    /// Copy cells from `src` into `self`, aligning origins. Returns the
    /// size actually copied (min of both grids on each axis).
    pub fn copy_from(&self, src: &Grid) -> Point {
        let sw = src.bounds.width().min(self.bounds.width());
        let sh = src.bounds.height().min(self.bounds.height());
        if Rc::ptr_eq(&self.buffer, &src.buffer) && self.bounds == src.bounds {
            return Point::new(sw, sh);
        }
        let src_buf = src.buffer.borrow();
        let mut dst_buf = self.buffer.borrow_mut();
        for dy in 0..sh {
            for dx in 0..sw {
                let sp = Point::new(src.bounds.min.x + dx, src.bounds.min.y + dy);
                let dp = Point::new(self.bounds.min.x + dx, self.bounds.min.y + dy);
                if let (Some(si), Some(di)) = (src_buf.index(sp.x, sp.y), dst_buf.index(dp.x, dp.y))
                {
                    dst_buf.cells[di] = src_buf.cells[si];
                }
            }
        }
        Point::new(sw, sh)
    }

    /// Resize the grid to the given dimensions.
    ///
    /// Creates a new backing buffer of the requested size, copies cells from
    /// the old grid that fall within the overlapping region, and fills any
    /// newly-exposed cells with `Cell::default()` (a space). If the existing
    /// backing buffer is large enough, it is reused. The grid's slice offset
    /// and bounds are reset to cover the full new grid.
    ///
    /// Matches Go gruid's `Grid.Resize` semantics.
    pub fn resize(&mut self, width: i32, height: i32) {
        let old_size = self.size();
        let ow = old_size.x;
        let oh = old_size.y;
        if ow == width && oh == height {
            return;
        }
        if width <= 0 || height <= 0 {
            self.bounds.max = self.bounds.min;
            return;
        }
        let new_w = width as usize;
        let new_h = height as usize;

        // Check if the underlying buffer is large enough to reuse.
        {
            let buf = self.buffer.borrow();
            let need_w = (self.bounds.min.x as usize) + new_w;
            let need_h = (self.bounds.min.y as usize) + new_h;
            if need_w <= buf.width && need_h <= buf.height {
                // Buffer is large enough — just adjust bounds.
                drop(buf);
                self.bounds.max = self.bounds.min.shift(width, height);
                return;
            }
        }

        // Need a new buffer. Create one, copy overlapping content.
        let mut new_buf = GridBuffer::new(new_w, new_h);
        {
            let old_buf = self.buffer.borrow();
            let copy_w = (ow.min(width)) as usize;
            let copy_h = (oh.min(height)) as usize;
            let old_width = old_buf.width;
            let old_min_x = self.bounds.min.x as usize;
            let old_min_y = self.bounds.min.y as usize;
            for dy in 0..copy_h {
                let src_start = (old_min_y + dy) * old_width + old_min_x;
                let dst_start = dy * new_w;
                new_buf.cells[dst_start..dst_start + copy_w]
                    .copy_from_slice(&old_buf.cells[src_start..src_start + copy_w]);
            }
        }

        self.buffer = Rc::new(RefCell::new(new_buf));
        self.bounds = Range::new(0, 0, width, height);
    }

    /// Row-major iterator over `(Point, Cell)` pairs with **relative**
    /// coordinates.
    pub fn iter(&self) -> GridIter<'_> {
        GridIter {
            grid: self,
            rel_iter: self.range_().iter(),
        }
    }

    /// Row-major iterator over the [`Point`]s in the grid (relative
    /// coordinates, no cells).
    ///
    /// This is a convenience wrapper around iterating over `self.range_()`.
    pub fn points(&self) -> impl Iterator<Item = Point> {
        self.range_().iter()
    }
}

// ---------------------------------------------------------------------------
// Display
// ---------------------------------------------------------------------------

impl fmt::Display for Grid {
    /// Render the grid as a string of characters, one row per line.
    ///
    /// Each cell's `ch` is emitted, and rows are separated by `'\n'`.
    /// Matches Go gruid's `Grid.String()` output.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let size = self.size();
        let buf = self.buffer.borrow();
        for y in 0..size.y {
            for x in 0..size.x {
                let abs_x = self.bounds.min.x + x;
                let abs_y = self.bounds.min.y + y;
                if let Some(i) = buf.index(abs_x, abs_y) {
                    write!(f, "{}", buf.cells[i].ch)?;
                }
            }
            writeln!(f)?;
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// GridIter
// ---------------------------------------------------------------------------

/// Iterator over `(Point, Cell)` pairs in a [`Grid`] with relative coords.
pub struct GridIter<'a> {
    grid: &'a Grid,
    rel_iter: crate::geom::RangeIter,
}

impl<'a> Iterator for GridIter<'a> {
    type Item = (Point, Cell);

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let rel_p = self.rel_iter.next()?;
        Some((rel_p, self.grid.at(rel_p)))
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.rel_iter.size_hint()
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
    /// Milliseconds since recording start (used for replay timing).
    pub time_ms: u64,
}

/// Compute the difference between two same-sized grids.
///
/// Returns a [`Frame`] containing only the cells that differ.
/// Positions in the frame are **relative** (0-based).
pub fn compute_frame(prev: &Grid, curr: &Grid) -> Frame {
    let bounds = curr.bounds();
    let min = bounds.min;
    let mut cells = Vec::new();
    for abs_p in bounds.iter() {
        // Read using absolute coords from buffer directly.
        let pc = prev.at(Point::new(abs_p.x - min.x, abs_p.y - min.y));
        let cc = curr.at(Point::new(abs_p.x - min.x, abs_p.y - min.y));
        if pc != cc {
            let rel_p = Point::new(abs_p.x - min.x, abs_p.y - min.y);
            cells.push(FrameCell {
                cell: cc,
                pos: rel_p,
            });
        }
    }
    Frame {
        cells,
        width: bounds.width(),
        height: bounds.height(),
        time_ms: 0,
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
    fn grid_slice_relative_coords() {
        let g = Grid::new(10, 10);
        let c = Cell::default().with_char('#');
        // Slice a 5x5 region starting at (2,3).
        let s = g.slice(Range::new(2, 3, 7, 8));
        assert_eq!(s.size(), Point::new(5, 5));
        // set at relative (0,0) in the slice -> absolute (2,3)
        s.set(Point::new(0, 0), c);
        // read from the parent grid at relative (2,3)
        assert_eq!(g.at(Point::new(2, 3)).ch, '#');
        // read from the slice at relative (0,0)
        assert_eq!(s.at(Point::new(0, 0)).ch, '#');
    }

    #[test]
    fn grid_slice_shares_buffer() {
        let g = Grid::new(4, 3);
        let s = g.slice(Range::new(1, 1, 3, 3));
        let c = Cell::default().with_char('#');
        // Set at relative (0,0) of the slice -> absolute (1,1).
        s.set(Point::new(0, 0), c);
        assert_eq!(g.at(Point::new(1, 1)).ch, '#');
    }

    #[test]
    fn grid_nested_slice() {
        let g = Grid::new(20, 20);
        let s1 = g.slice(Range::new(5, 5, 15, 15));
        let s2 = s1.slice(Range::new(2, 2, 5, 5));
        let c = Cell::default().with_char('Z');
        // Set at relative (0,0) of s2 -> absolute (7,7)
        s2.set(Point::new(0, 0), c);
        assert_eq!(g.at(Point::new(7, 7)).ch, 'Z');
        assert_eq!(s1.at(Point::new(2, 2)).ch, 'Z');
        assert_eq!(s2.at(Point::new(0, 0)).ch, 'Z');
    }

    #[test]
    fn grid_contains_relative() {
        let g = Grid::new(10, 10);
        let s = g.slice(Range::new(5, 5, 8, 8));
        assert!(s.contains(Point::new(0, 0)));
        assert!(s.contains(Point::new(2, 2)));
        assert!(!s.contains(Point::new(3, 0))); // outside slice width
        assert!(!s.contains(Point::new(-1, 0)));
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
    fn grid_iter_relative_coords() {
        let g = Grid::new(10, 10);
        let s = g.slice(Range::new(3, 3, 6, 6));
        let positions: Vec<Point> = s.iter().map(|(p, _)| p).collect();
        assert_eq!(positions[0], Point::new(0, 0));
        assert_eq!(positions.last(), Some(&Point::new(2, 2)));
        assert_eq!(positions.len(), 9); // 3x3
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

    #[test]
    fn compute_frame_relative_positions() {
        // Even for sliced grids, frame positions should be relative.
        let a = Grid::new(10, 10);
        let b = Grid::new(10, 10);
        b.set(Point::new(5, 5), Cell::default().with_char('X'));
        let frame = compute_frame(&a, &b);
        assert_eq!(frame.cells.len(), 1);
        assert_eq!(frame.cells[0].pos, Point::new(5, 5));
    }

    #[test]
    fn grid_map_cells_relative() {
        let g = Grid::new(10, 10);
        let s = g.slice(Range::new(5, 5, 8, 8));
        s.map_cells(|p, _| Cell::default().with_char(if p.x == 0 && p.y == 0 { 'O' } else { '.' }));
        assert_eq!(s.at(Point::new(0, 0)).ch, 'O');
        assert_eq!(s.at(Point::new(1, 0)).ch, '.');
    }

    // -----------------------------------------------------------------------
    // resize tests
    // -----------------------------------------------------------------------

    #[test]
    fn resize_same_size_is_noop() {
        let mut g = Grid::new(4, 3);
        g.set(Point::new(1, 1), Cell::default().with_char('A'));
        g.resize(4, 3);
        assert_eq!(g.size(), Point::new(4, 3));
        assert_eq!(g.at(Point::new(1, 1)).ch, 'A');
    }

    #[test]
    fn resize_grow_preserves_content() {
        let mut g = Grid::new(3, 2);
        g.set(Point::new(0, 0), Cell::default().with_char('X'));
        g.set(Point::new(2, 1), Cell::default().with_char('Y'));
        g.resize(5, 4);
        assert_eq!(g.size(), Point::new(5, 4));
        // Old content preserved.
        assert_eq!(g.at(Point::new(0, 0)).ch, 'X');
        assert_eq!(g.at(Point::new(2, 1)).ch, 'Y');
        // New cells are default (space).
        assert_eq!(g.at(Point::new(3, 0)).ch, ' ');
        assert_eq!(g.at(Point::new(0, 3)).ch, ' ');
    }

    #[test]
    fn resize_shrink_preserves_overlap() {
        let mut g = Grid::new(5, 5);
        g.set(Point::new(1, 1), Cell::default().with_char('K'));
        g.set(Point::new(4, 4), Cell::default().with_char('Z'));
        g.resize(3, 3);
        assert_eq!(g.size(), Point::new(3, 3));
        // (1,1) is still inside the new grid.
        assert_eq!(g.at(Point::new(1, 1)).ch, 'K');
        // (4,4) is outside bounds now.
        assert_eq!(g.at(Point::new(4, 4)), Cell::default());
    }

    #[test]
    fn resize_to_zero_produces_empty() {
        let mut g = Grid::new(3, 3);
        g.resize(0, 0);
        assert_eq!(g.size(), Point::new(0, 0));
    }

    #[test]
    fn resize_grow_width_only() {
        let mut g = Grid::new(2, 2);
        g.set(Point::new(0, 0), Cell::default().with_char('A'));
        g.set(Point::new(1, 1), Cell::default().with_char('B'));
        g.resize(4, 2);
        assert_eq!(g.size(), Point::new(4, 2));
        assert_eq!(g.at(Point::new(0, 0)).ch, 'A');
        assert_eq!(g.at(Point::new(1, 1)).ch, 'B');
        assert_eq!(g.at(Point::new(2, 0)).ch, ' ');
    }

    #[test]
    fn resize_grow_height_only() {
        let mut g = Grid::new(2, 2);
        g.set(Point::new(0, 0), Cell::default().with_char('A'));
        g.set(Point::new(1, 1), Cell::default().with_char('B'));
        g.resize(2, 5);
        assert_eq!(g.size(), Point::new(2, 5));
        assert_eq!(g.at(Point::new(0, 0)).ch, 'A');
        assert_eq!(g.at(Point::new(1, 1)).ch, 'B');
        assert_eq!(g.at(Point::new(0, 4)).ch, ' ');
    }

    // -----------------------------------------------------------------------
    // Display test
    // -----------------------------------------------------------------------

    #[test]
    fn display_output() {
        let g = Grid::new(3, 2);
        g.set(Point::new(0, 0), Cell::default().with_char('A'));
        g.set(Point::new(1, 0), Cell::default().with_char('B'));
        g.set(Point::new(2, 0), Cell::default().with_char('C'));
        g.set(Point::new(0, 1), Cell::default().with_char('D'));
        g.set(Point::new(1, 1), Cell::default().with_char('E'));
        g.set(Point::new(2, 1), Cell::default().with_char('F'));
        let s = format!("{}", g);
        assert_eq!(s, "ABC\nDEF\n");
    }

    #[test]
    fn display_slice() {
        let g = Grid::new(4, 4);
        g.fill(Cell::default().with_char('.'));
        g.set(Point::new(1, 1), Cell::default().with_char('X'));
        let s = g.slice(Range::new(1, 1, 3, 3));
        let out = format!("{}", s);
        assert_eq!(out, "X.\n..\n");
    }

    // -----------------------------------------------------------------------
    // points() test
    // -----------------------------------------------------------------------

    #[test]
    fn points_iterator() {
        let g = Grid::new(3, 2);
        let pts: Vec<Point> = g.points().collect();
        assert_eq!(pts.len(), 6);
        assert_eq!(pts[0], Point::new(0, 0));
        assert_eq!(pts[1], Point::new(1, 0));
        assert_eq!(pts[5], Point::new(2, 1));
    }

    #[test]
    fn points_on_slice() {
        let g = Grid::new(10, 10);
        let s = g.slice(Range::new(3, 3, 6, 5));
        let pts: Vec<Point> = s.points().collect();
        // 3x2 slice -> 6 points, all relative
        assert_eq!(pts.len(), 6);
        assert_eq!(pts[0], Point::new(0, 0));
        assert_eq!(pts[5], Point::new(2, 1));
    }
}
