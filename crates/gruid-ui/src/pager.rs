use gruid_core::{Cell, Grid, Point, Style};
use gruid_core::messages::{Key, Msg};

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
    pub page_up: Vec<Key>,
    pub page_down: Vec<Key>,
    pub quit: Vec<Key>,
}

impl Default for PagerKeys {
    fn default() -> Self {
        Self {
            up: vec![Key::ArrowUp, Key::Char('k')],
            down: vec![Key::ArrowDown, Key::Char('j')],
            page_up: vec![Key::PageUp],
            page_down: vec![Key::PageDown],
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
    scroll: i32,
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
            scroll: 0,
            action: PagerAction::Pass,
        }
    }

    /// Process an input message and return the resulting action.
    pub fn update(&mut self, msg: Msg) -> PagerAction {
        self.action = PagerAction::Pass;
        let page_size = self.visible_height();

        match msg {
            Msg::KeyDown { ref key, .. } => {
                if self.keys.up.contains(key) {
                    self.scroll_by(-1);
                    self.action = PagerAction::Scroll;
                } else if self.keys.down.contains(key) {
                    self.scroll_by(1);
                    self.action = PagerAction::Scroll;
                } else if self.keys.page_up.contains(key) {
                    self.scroll_by(-page_size);
                    self.action = PagerAction::Scroll;
                } else if self.keys.page_down.contains(key) {
                    self.scroll_by(page_size);
                    self.action = PagerAction::Scroll;
                } else if self.keys.quit.contains(key) {
                    self.action = PagerAction::Quit;
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
            grid.bounds()
        };

        let start = inner_range.min;
        let vis_h = (inner_range.max.y - inner_range.min.y) as usize;
        let vis_w = (inner_range.max.x - inner_range.min.x) as usize;

        for row in 0..vis_h {
            let line_idx = self.scroll as usize + row;
            let y = start.y + row as i32;

            if line_idx < self.lines.len() {
                let text = self.lines[line_idx].content();
                let style = self.lines[line_idx].style();
                let mut x = start.x;
                for ch in text.chars() {
                    if (x - start.x) as usize >= vis_w {
                        break;
                    }
                    let p = Point::new(x, y);
                    if grid.contains(p) {
                        grid.set(p, Cell::default().with_char(ch).with_style(style));
                    }
                    x += 1;
                }
            }
        }
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

    fn scroll_by(&mut self, delta: i32) {
        let max_scroll = (self.lines.len() as i32 - self.visible_height()).max(0);
        self.scroll = (self.scroll + delta).clamp(0, max_scroll);
    }
}
