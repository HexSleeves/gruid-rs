//! Scrollable text pager widget with keyboard, mouse, and seeking support.

use gruid_core::messages::{Key, MouseAction, Msg};
use gruid_core::{Cell, Grid, Point, Style};

use crate::{BoxDecor, StyledText};

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
        let page_size = self.visible_height();
        let half_page = (page_size / 2).max(1);

        match msg {
            Msg::KeyDown { ref key, .. } => {
                if self.keys.up.contains(key) {
                    self.scroll_by_y(-1);
                    self.action = PagerAction::Scroll;
                } else if self.keys.down.contains(key) {
                    self.scroll_by_y(1);
                    self.action = PagerAction::Scroll;
                } else if self.keys.left.contains(key) {
                    self.scroll_by_x(-1);
                    self.action = PagerAction::Scroll;
                } else if self.keys.right.contains(key) {
                    self.scroll_by_x(1);
                    self.action = PagerAction::Scroll;
                } else if self.keys.page_up.contains(key) {
                    self.scroll_by_y(-page_size);
                    self.action = PagerAction::Scroll;
                } else if self.keys.page_down.contains(key) {
                    self.scroll_by_y(page_size);
                    self.action = PagerAction::Scroll;
                } else if self.keys.half_page_up.contains(key) {
                    self.scroll_by_y(-half_page);
                    self.action = PagerAction::Scroll;
                } else if self.keys.half_page_down.contains(key) {
                    self.scroll_by_y(half_page);
                    self.action = PagerAction::Scroll;
                } else if self.keys.top.contains(key) {
                    self.scroll_y = 0;
                    self.action = PagerAction::Scroll;
                } else if self.keys.bottom.contains(key) {
                    self.scroll_y = self.max_scroll_y();
                    self.action = PagerAction::Scroll;
                } else if self.keys.quit.contains(key) {
                    self.action = PagerAction::Quit;
                }
            }
            Msg::Mouse { action, .. } => match action {
                MouseAction::WheelUp => {
                    self.scroll_by_y(-3);
                    self.action = PagerAction::Scroll;
                }
                MouseAction::WheelDown => {
                    self.scroll_by_y(3);
                    self.action = PagerAction::Scroll;
                }
                _ => {}
            },
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
                let text = self.lines[line_idx].content();
                let style = self.lines[line_idx].style();
                let chars: Vec<char> = text.chars().collect();
                let x_offset = self.scroll_x as usize;

                for col in 0..vis_w {
                    let x = start.x + col as i32;
                    let char_idx = x_offset + col;
                    let ch = if char_idx < chars.len() {
                        chars[char_idx]
                    } else {
                        ' '
                    };
                    let p = Point::new(x, y);
                    if grid.contains(p) {
                        grid.set(p, Cell::default().with_char(ch).with_style(style));
                    }
                }
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

    /// Replace the lines.
    pub fn set_lines(&mut self, lines: Vec<StyledText>) {
        self.lines = lines;
        self.scroll_y = self.scroll_y.min(self.max_scroll_y());
    }

    /// Set the vertical scroll position.
    pub fn set_cursor(&mut self, y: i32) {
        self.scroll_y = y.clamp(0, self.max_scroll_y());
    }

    /// Replace the box decoration.
    pub fn set_box(&mut self, box_: Option<BoxDecor>) {
        self.box_ = box_;
    }

    /// Return the current scroll position as (x, y).
    pub fn view(&self) -> Point {
        Point::new(self.scroll_x, self.scroll_y)
    }

    // -- private helpers --

    fn visible_height(&self) -> i32 {
        let h = self.grid.height();
        if self.box_.is_some() {
            (h - 2).max(1)
        } else {
            h.max(1)
        }
    }

    fn visible_width(&self) -> i32 {
        let w = self.grid.width();
        if self.box_.is_some() {
            (w - 2).max(1)
        } else {
            w.max(1)
        }
    }

    fn max_scroll_y(&self) -> i32 {
        (self.lines.len() as i32 - self.visible_height()).max(0)
    }

    fn max_scroll_x(&self) -> i32 {
        let max_line_len = self.lines.iter().map(|l| l.content().len()).max().unwrap_or(0) as i32;
        (max_line_len - self.visible_width()).max(0)
    }

    fn scroll_by_y(&mut self, delta: i32) {
        self.scroll_y = (self.scroll_y + delta).clamp(0, self.max_scroll_y());
    }

    fn scroll_by_x(&mut self, delta: i32) {
        self.scroll_x = (self.scroll_x + delta).clamp(0, self.max_scroll_x());
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
        assert_eq!(pager.view().y, 0);

        pager.update(Msg::key(Key::ArrowDown));
        assert_eq!(pager.view().y, 1);

        pager.update(Msg::key(Key::ArrowUp));
        assert_eq!(pager.view().y, 0);

        // Can't scroll above 0
        pager.update(Msg::key(Key::ArrowUp));
        assert_eq!(pager.view().y, 0);
    }

    #[test]
    fn page_up_down() {
        let mut pager = make_pager(30, 5);
        pager.update(Msg::key(Key::PageDown));
        assert_eq!(pager.view().y, 5);

        pager.update(Msg::key(Key::PageUp));
        assert_eq!(pager.view().y, 0);
    }

    #[test]
    fn top_bottom() {
        let mut pager = make_pager(30, 5);
        pager.update(Msg::key(Key::End));
        assert_eq!(pager.view().y, 25); // 30 - 5

        pager.update(Msg::key(Key::Home));
        assert_eq!(pager.view().y, 0);
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
        assert_eq!(pager.view().x, 1);

        pager.update(Msg::key(Key::ArrowLeft));
        assert_eq!(pager.view().x, 0);
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
        assert_eq!(pager.view().y, 3);
    }

    #[test]
    fn quit_action() {
        let mut pager = make_pager(5, 5);
        let action = pager.update(Msg::key(Key::Escape));
        assert_eq!(action, PagerAction::Quit);
    }
}
