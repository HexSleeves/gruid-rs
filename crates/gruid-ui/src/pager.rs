//! Scrollable text pager widget with keyboard, mouse, and seeking support.

use gruid_core::messages::{Key, MouseAction, Msg};
use gruid_core::{Cell, Grid, Point, Range, Style};

use crate::{BoxDecor, StyledText};

/// Horizontal scroll step (columns per left/right key press), matching Go gruid.
const SCROLL_STEP_X: i32 = 8;

/// Configuration for a [`Pager`] widget.
#[derive(Debug, Clone)]
pub struct PagerConfig {
    /// The text content to page through.
    pub content: StyledText,
    /// Grid to draw into.
    pub grid: Grid,
    /// Key bindings.
    pub keys: PagerKeys,
    /// Optional box decoration.
    pub box_: Option<BoxDecor>,
    /// Visual style.
    pub style: PagerStyle,
}

/// Key bindings for pager navigation.
#[derive(Debug, Clone)]
pub struct PagerKeys {
    pub up: Vec<Key>,
    pub down: Vec<Key>,
    pub left: Vec<Key>,
    pub right: Vec<Key>,
    /// Keys that reset horizontal scroll to 0 (Go default: `0`, `^`).
    pub start: Vec<Key>,
    pub page_up: Vec<Key>,
    pub page_down: Vec<Key>,
    pub half_page_up: Vec<Key>,
    pub half_page_down: Vec<Key>,
    pub top: Vec<Key>,
    pub bottom: Vec<Key>,
    pub quit: Vec<Key>,
}

impl Default for PagerKeys {
    fn default() -> Self {
        Self {
            up: vec![Key::ArrowUp, Key::Char('k')],
            down: vec![Key::ArrowDown, Key::Char('j')],
            left: vec![Key::ArrowLeft, Key::Char('h')],
            right: vec![Key::ArrowRight, Key::Char('l')],
            start: vec![Key::Char('^'), Key::Char('0')],
            page_up: vec![Key::PageUp, Key::Char('b')],
            page_down: vec![Key::PageDown, Key::Char('f')],
            half_page_up: vec![Key::Char('u')],
            half_page_down: vec![Key::Char('d')],
            top: vec![Key::Home, Key::Char('g')],
            bottom: vec![Key::End, Key::Char('G')],
            quit: vec![Key::Escape, Key::Char('q')],
        }
    }
}

/// Visual style for a pager.
#[derive(Debug, Clone, Default)]
pub struct PagerStyle {
    /// Style for the page number indicator.
    pub page_num: Style,
}

/// Actions returned by [`Pager::update`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PagerAction {
    /// No meaningful action.
    Pass,
    /// The scroll position changed.
    Scroll,
    /// The user requested to quit/close the pager.
    Quit,
}

/// A scrollable text pager widget.
#[derive(Debug, Clone)]
pub struct Pager {
    lines: Vec<StyledText>,
    grid: Grid,
    keys: PagerKeys,
    box_: Option<BoxDecor>,
    _style: PagerStyle,
    scroll_y: i32,
    scroll_x: i32,
    action: PagerAction,
}

impl Pager {
    /// Create a new pager from the given configuration.
    pub fn new(config: PagerConfig) -> Self {
        let width = config.grid.width().max(0) as usize;
        let formatted = config.content.format(width.saturating_sub(2).max(1));
        let lines = formatted.lines();
        Self {
            lines,
            grid: config.grid,
            keys: config.keys,
            box_: config.box_,
            _style: config.style,
            scroll_y: 0,
            scroll_x: 0,
            action: PagerAction::Pass,
        }
    }

    /// Process an input message and return the resulting action.
    pub fn update(&mut self, msg: Msg) -> PagerAction {
        self.action = PagerAction::Pass;
        let nlines = self.visible_height();

        match msg {
            Msg::KeyDown { ref key, .. } => {
                if self.keys.up.contains(key) {
                    self.up(1);
                } else if self.keys.down.contains(key) {
                    self.down(1);
                } else if self.keys.left.contains(key) {
                    self.left();
                } else if self.keys.right.contains(key) {
                    self.right();
                } else if self.keys.start.contains(key) {
                    self.line_start();
                } else if self.keys.page_down.contains(key) || self.keys.half_page_down.contains(key) {
                    let mut shift = nlines - 1;
                    if self.keys.half_page_down.contains(key) {
                        shift /= 2;
                    }
                    self.down(shift);
                } else if self.keys.page_up.contains(key) || self.keys.half_page_up.contains(key) {
                    let mut shift = nlines - 1;
                    if self.keys.half_page_up.contains(key) {
                        shift /= 2;
                    }
                    self.up(shift);
                } else if self.keys.top.contains(key) {
                    self.go_top();
                } else if self.keys.bottom.contains(key) {
                    self.go_bottom();
                } else if self.keys.quit.contains(key) {
                    self.action = PagerAction::Quit;
                }
            }
            Msg::Mouse {
                action, pos, ..
            } => {
                let (h, bh) = self.height();
                let nlines_vis = h - bh;
                let grid_range = self.grid.range_().lines(0, h);

                if !grid_range.contains(pos) {
                    // Click outside the pager area.
                    if action == MouseAction::Main {
                        self.action = PagerAction::Quit;
                    }
                } else {
                    match action {
                        MouseAction::Main => {
                            let rel_y = pos.y - self.grid.bounds().min.y;
                            if rel_y > nlines_vis / 2 {
                                self.down(nlines_vis - 1);
                            } else {
                                self.up(nlines_vis - 1);
                            }
                        }
                        MouseAction::WheelUp => {
                            self.up(1);
                        }
                        MouseAction::WheelDown => {
                            self.down(1);
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }

        self.action
    }

    /// Draw the pager into the given grid.
    pub fn draw(&self, grid: &Grid) {
        let inner_range = if let Some(ref box_decor) = self.box_ {
            box_decor.draw(grid)
        } else {
            grid.range_()
        };

        let start = inner_range.min;
        let vis_h = (inner_range.max.y - inner_range.min.y) as usize;
        let vis_w = (inner_range.max.x - inner_range.min.x) as usize;

        for row in 0..vis_h {
            let line_idx = self.scroll_y as usize + row;
            let y = start.y + row as i32;

            if line_idx < self.lines.len() {
                let stt = &self.lines[line_idx];
                let style = stt.style();
                // Fill line with base style, then render styled chars.
                for col in 0..vis_w {
                    let x = start.x + col as i32;
                    let p = Point::new(x, y);
                    if grid.contains(p) {
                        grid.set(p, Cell::default().with_char(' ').with_style(style));
                    }
                }
                let scroll_x = self.scroll_x;
                stt.iter(|p, cell| {
                    let shifted_x = p.x - scroll_x;
                    if shifted_x >= 0 && shifted_x < vis_w as i32 {
                        let gp = Point::new(start.x + shifted_x, y);
                        if grid.contains(gp) {
                            grid.set(gp, cell);
                        }
                    }
                });
            } else {
                // Clear rows past the content
                for col in 0..vis_w {
                    let x = start.x + col as i32;
                    let p = Point::new(x, y);
                    if grid.contains(p) {
                        grid.set(p, Cell::default());
                    }
                }
            }
        }
    }

    /// Return the last action.
    pub fn action(&self) -> PagerAction {
        self.action
    }

    /// Return the total number of content lines.
    pub fn lines(&self) -> usize {
        self.lines.len()
    }

    /// Replace the lines.
    pub fn set_lines(&mut self, lines: Vec<StyledText>) {
        let nlines = self.visible_height();
        self.lines = lines;
        if self.scroll_y + nlines - 1 >= self.lines.len() as i32 {
            self.scroll_y = self.lines.len() as i32 - nlines;
            if self.scroll_y < 0 {
                self.scroll_y = 0;
            }
        }
    }

    /// Set both horizontal and vertical scroll position.
    ///
    /// `pos.x` sets the horizontal column offset and `pos.y` sets the
    /// line index of the top visible row.
    pub fn set_cursor(&mut self, pos: Point) {
        self.scroll_x = pos.x.max(0);
        let nlines = self.visible_height();
        self.scroll_y = pos.y;
        if self.scroll_y + nlines - 1 >= self.lines.len() as i32 {
            self.scroll_y = self.lines.len() as i32 - nlines;
        }
        if self.scroll_y < 0 {
            self.scroll_y = 0;
        }
    }

    /// Replace the box decoration.
    pub fn set_box(&mut self, box_: Option<BoxDecor>) {
        self.box_ = box_;
    }

    /// Return a [`Range`] representing the currently visible viewport.
    ///
    /// `min` is `(scroll_x, scroll_y)` and `max` is the exclusive far
    /// corner, matching Go gruid's `Pager.View()`.
    pub fn view(&self) -> Range {
        let size = self.grid.size();
        let bh = if self.box_.is_some() { 2 } else { 0 };
        let mut h = size.y;
        if h > bh + self.lines.len() as i32 {
            h = bh + self.lines.len() as i32;
        }
        if h - bh <= 0 {
            return Range::new(0, 0, 0, 0);
        }
        Range::new(
            self.scroll_x,
            self.scroll_y,
            self.scroll_x + size.x,
            self.scroll_y + h - bh,
        )
    }

    // -- private helpers matching Go gruid's Pager methods --

    fn visible_height(&self) -> i32 {
        let (h, bh) = self.height();
        h - bh
    }

    fn height(&self) -> (i32, i32) {
        let mut h = self.grid.height();
        let bh = if self.box_.is_some() { 2 } else { 0 };
        if h > bh + self.lines.len() as i32 {
            h = bh + self.lines.len() as i32;
        }
        (h, bh)
    }

    fn down(&mut self, mut shift: i32) {
        let nlines = self.visible_height();
        if self.scroll_y + nlines + shift - 1 >= self.lines.len() as i32 {
            shift = self.lines.len() as i32 - self.scroll_y - nlines;
        }
        if shift > 0 {
            self.action = PagerAction::Scroll;
            self.scroll_y += shift;
        }
    }

    fn up(&mut self, mut shift: i32) {
        if self.scroll_y - shift < 0 {
            shift = self.scroll_y;
        }
        if shift > 0 {
            self.action = PagerAction::Scroll;
            self.scroll_y -= shift;
        }
    }

    fn right(&mut self) {
        self.action = PagerAction::Scroll;
        self.scroll_x += SCROLL_STEP_X;
    }

    fn left(&mut self) {
        if self.scroll_x > 0 {
            self.action = PagerAction::Scroll;
            self.scroll_x -= SCROLL_STEP_X;
            if self.scroll_x < 0 {
                self.scroll_x = 0;
            }
        }
    }

    fn line_start(&mut self) {
        if self.scroll_x > 0 {
            self.action = PagerAction::Scroll;
            self.scroll_x = 0;
        }
    }

    fn go_top(&mut self) {
        if self.scroll_y != 0 {
            self.scroll_y = 0;
            self.action = PagerAction::Scroll;
        }
    }

    fn go_bottom(&mut self) {
        let nlines = self.visible_height();
        let target = self.lines.len() as i32 - nlines;
        if self.scroll_y != target {
            self.scroll_y = target;
            self.action = PagerAction::Scroll;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_pager(num_lines: usize, height: i32) -> Pager {
        let content_str: String = (0..num_lines)
            .map(|i| format!("Line {i}"))
            .collect::<Vec<_>>()
            .join("\n");
        Pager::new(PagerConfig {
            content: StyledText::new(&content_str, Style::default()),
            grid: Grid::new(20, height),
            keys: PagerKeys::default(),
            box_: None,
            style: PagerStyle::default(),
        })
    }

    #[test]
    fn scroll_down_up() {
        let mut pager = make_pager(20, 5);
        assert_eq!(pager.view().min.y, 0);

        pager.update(Msg::key(Key::ArrowDown));
        assert_eq!(pager.view().min.y, 1);

        pager.update(Msg::key(Key::ArrowUp));
        assert_eq!(pager.view().min.y, 0);

        // Can't scroll above 0
        pager.update(Msg::key(Key::ArrowUp));
        assert_eq!(pager.view().min.y, 0);
    }

    #[test]
    fn page_up_down() {
        let mut pager = make_pager(30, 5);
        pager.update(Msg::key(Key::PageDown));
        assert_eq!(pager.view().min.y, 4); // nlines-1 = 5-1 = 4

        pager.update(Msg::key(Key::PageUp));
        assert_eq!(pager.view().min.y, 0);
    }

    #[test]
    fn top_bottom() {
        let mut pager = make_pager(30, 5);
        pager.update(Msg::key(Key::End));
        assert_eq!(pager.view().min.y, 25); // 30 - 5

        pager.update(Msg::key(Key::Home));
        assert_eq!(pager.view().min.y, 0);
    }

    #[test]
    fn horizontal_scroll() {
        // Use a narrow grid (5 chars) with long lines so horizontal scroll works.
        let long_content = "This is a very long line that exceeds the grid width";
        let mut pager = Pager::new(PagerConfig {
            content: StyledText::new(long_content, Style::default()),
            grid: Grid::new(5, 3),
            keys: PagerKeys::default(),
            box_: None,
            style: PagerStyle::default(),
        });
        pager.update(Msg::key(Key::ArrowRight));
        assert_eq!(pager.view().min.x, SCROLL_STEP_X);

        pager.update(Msg::key(Key::ArrowLeft));
        assert_eq!(pager.view().min.x, 0);
    }

    #[test]
    fn horizontal_scroll_step_is_8() {
        let long_content = "A very long line of text that is wider than the pager grid width";
        let mut pager = Pager::new(PagerConfig {
            content: StyledText::new(long_content, Style::default()),
            grid: Grid::new(10, 3),
            keys: PagerKeys::default(),
            box_: None,
            style: PagerStyle::default(),
        });
        pager.update(Msg::key(Key::ArrowRight));
        assert_eq!(pager.view().min.x, 8);
        pager.update(Msg::key(Key::ArrowRight));
        assert_eq!(pager.view().min.x, 16);
        pager.update(Msg::key(Key::ArrowLeft));
        assert_eq!(pager.view().min.x, 8);
    }

    #[test]
    fn start_key_resets_x() {
        let long_content = "A very long line of text";
        let mut pager = Pager::new(PagerConfig {
            content: StyledText::new(long_content, Style::default()),
            grid: Grid::new(5, 3),
            keys: PagerKeys::default(),
            box_: None,
            style: PagerStyle::default(),
        });
        pager.update(Msg::key(Key::ArrowRight));
        pager.update(Msg::key(Key::ArrowRight));
        assert!(pager.view().min.x > 0);
        pager.update(Msg::key(Key::Char('0')));
        assert_eq!(pager.view().min.x, 0);
    }

    #[test]
    fn mouse_wheel() {
        let mut pager = make_pager(30, 5);
        pager.update(Msg::Mouse {
            action: MouseAction::WheelDown,
            pos: Point::new(0, 0),
            modifiers: Default::default(),
            time: std::time::Instant::now(),
        });
        assert_eq!(pager.view().min.y, 1);
    }

    #[test]
    fn mouse_click_page_down() {
        let mut pager = make_pager(30, 6);
        // Click in the bottom half (y = 4, nlines = 6, half = 3)
        pager.update(Msg::Mouse {
            action: MouseAction::Main,
            pos: Point::new(5, 4),
            modifiers: Default::default(),
            time: std::time::Instant::now(),
        });
        assert_eq!(pager.action(), PagerAction::Scroll);
        assert_eq!(pager.view().min.y, 5); // nlines-1 = 5
    }

    #[test]
    fn mouse_click_page_up() {
        let mut pager = make_pager(30, 6);
        // First scroll down.
        pager.set_cursor(Point::new(0, 10));
        // Click in the top half (y = 1, nlines = 6, half = 3)
        pager.update(Msg::Mouse {
            action: MouseAction::Main,
            pos: Point::new(5, 1),
            modifiers: Default::default(),
            time: std::time::Instant::now(),
        });
        assert_eq!(pager.action(), PagerAction::Scroll);
        assert_eq!(pager.view().min.y, 5); // 10 - 5
    }

    #[test]
    fn quit_action() {
        let mut pager = make_pager(5, 5);
        let action = pager.update(Msg::key(Key::Escape));
        assert_eq!(action, PagerAction::Quit);
    }

    #[test]
    fn lines_count() {
        let pager = make_pager(15, 5);
        assert_eq!(pager.lines(), 15);
    }

    #[test]
    fn view_returns_range() {
        let pager = make_pager(10, 5);
        let v = pager.view();
        assert_eq!(v.min, Point::new(0, 0));
        assert_eq!(v.max.y, 5);
        assert_eq!(v.max.x, 20); // grid width
    }

    #[test]
    fn set_cursor_sets_both_x_and_y() {
        let mut pager = make_pager(30, 5);
        pager.set_cursor(Point::new(4, 10));
        let v = pager.view();
        assert_eq!(v.min.x, 4);
        assert_eq!(v.min.y, 10);
    }

    #[test]
    fn set_cursor_clamps() {
        let mut pager = make_pager(20, 6);
        // Set beyond max
        pager.set_cursor(Point::new(0, 100));
        let v = pager.view();
        assert!(v.min.y >= 0);
        assert!(v.max.y <= 20);
        // Negative x clamped to 0
        pager.set_cursor(Point::new(-5, 3));
        assert_eq!(pager.view().min.x, 0);
    }

    #[test]
    fn pager_go_test() {
        // Port of the Go TestPager.
        let content_str = "line 0\nline 1\nline 2\nline 3";
        let mut pager = Pager::new(PagerConfig {
            content: StyledText::new(content_str, Style::default()),
            grid: Grid::new(10, 6),
            keys: PagerKeys::default(),
            box_: None,
            style: PagerStyle::default(),
        });
        assert_eq!(pager.action(), PagerAction::Pass);
        assert_eq!(pager.lines(), 4);
        assert_eq!(pager.view().size().y, 4);

        pager.update(Msg::key(Key::Escape));
        assert_eq!(pager.action(), PagerAction::Quit);

        // Double the lines
        let double_content = "line 0\nline 1\nline 2\nline 3\nline 0\nline 1\nline 2\nline 3";
        let double_formatted = StyledText::new(double_content, Style::default());
        pager.set_lines(double_formatted.lines());
        assert_eq!(pager.view().size().y, 6);
        assert_eq!(pager.view().max.y, 6);
        assert_eq!(pager.lines(), 8);

        pager.update(Msg::key(Key::ArrowDown));
        assert_eq!(pager.action(), PagerAction::Scroll);
        assert_eq!(pager.view().size().y, 6);
        assert_eq!(pager.view().max.y, 7);

        pager.set_lines(vec![]);
        assert_eq!(pager.view().size().y, 0);
        pager.update(Msg::key(Key::ArrowDown));
        assert_eq!(pager.action(), PagerAction::Pass);
    }

    #[test]
    fn pager_set_cursor_go_test() {
        // Port of the Go TestPagerSetCursor.
        let content_str: String = (0..20)
            .map(|i| format!("{i}"))
            .collect::<Vec<_>>()
            .join("\n");
        let mut pager = Pager::new(PagerConfig {
            content: StyledText::new(&content_str, Style::default()),
            grid: Grid::new(10, 6),
            keys: PagerKeys::default(),
            box_: None,
            style: PagerStyle::default(),
        });

        for i in -1..=20 {
            pager.set_cursor(Point::new(0, i));
            let view = pager.view();
            assert!(
                view.min.y >= 0,
                "view min y: {} ({})",
                view.min.y,
                i
            );
            assert!(
                view.max.y <= 20,
                "view max y: {} ({})",
                view.max.y,
                i
            );
            if i >= 0 && i <= 14 {
                assert_eq!(
                    view.max.y,
                    i + 6,
                    "view max y: {} ({})",
                    view.max.y,
                    i
                );
            }
            if i == 14 {
                assert_eq!(
                    view.min.y, i,
                    "view min y: {} ({})",
                    view.min.y, i
                );
            }
            if i == 15 {
                assert_ne!(
                    view.min.y, i,
                    "view min y should not be {} ({})",
                    view.min.y, i
                );
            }
        }
    }
}
