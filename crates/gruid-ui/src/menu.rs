use gruid_core::messages::{Key, Msg};
use gruid_core::{Cell, Grid, Point, Style};

use crate::{BoxDecor, StyledText};

/// Configuration for a [`Menu`] widget.
#[derive(Debug, Clone)]
pub struct MenuConfig {
    /// Grid to draw into.
    pub grid: Grid,
    /// The menu entries.
    pub entries: Vec<MenuEntry>,
    /// Key bindings.
    pub keys: MenuKeys,
    /// Optional box decoration.
    pub box_: Option<BoxDecor>,
    /// Visual style.
    pub style: MenuStyle,
}

/// A single entry in a menu.
#[derive(Debug, Clone)]
pub struct MenuEntry {
    /// Display text.
    pub text: StyledText,
    /// Whether the entry is disabled (cannot be invoked).
    pub disabled: bool,
    /// Shortcut keys that invoke this entry.
    pub keys: Vec<Key>,
}

impl MenuEntry {
    /// Create a new enabled entry with the given text and no shortcut keys.
    pub fn new(text: StyledText) -> Self {
        Self {
            text,
            disabled: false,
            keys: Vec::new(),
        }
    }
}

/// Key bindings for menu navigation.
#[derive(Debug, Clone)]
pub struct MenuKeys {
    pub up: Vec<Key>,
    pub down: Vec<Key>,
    pub left: Vec<Key>,
    pub right: Vec<Key>,
    pub invoke: Vec<Key>,
    pub quit: Vec<Key>,
}

impl Default for MenuKeys {
    fn default() -> Self {
        Self {
            up: vec![Key::ArrowUp, Key::Char('k')],
            down: vec![Key::ArrowDown, Key::Char('j')],
            left: vec![Key::ArrowLeft, Key::Char('h')],
            right: vec![Key::ArrowRight, Key::Char('l')],
            invoke: vec![Key::Enter],
            quit: vec![Key::Escape, Key::Char('q')],
        }
    }
}

/// Visual style for a menu.
#[derive(Debug, Clone)]
pub struct MenuStyle {
    /// Layout size hint (columns, rows) for arranging entries.
    pub layout: Point,
    /// Style for the active (highlighted) entry.
    pub active: Style,
    /// Style for page number indicator.
    pub page_num: Style,
}

impl Default for MenuStyle {
    fn default() -> Self {
        Self {
            layout: Point::new(1, 0),
            active: Style::default(),
            page_num: Style::default(),
        }
    }
}

/// Actions returned by [`Menu::update`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MenuAction {
    /// No meaningful action occurred.
    Pass,
    /// The active entry changed.
    Move,
    /// The active entry was invoked.
    Invoke,
    /// The user requested to quit/close the menu.
    Quit,
}

/// An interactive menu widget.
#[derive(Debug, Clone)]
pub struct Menu {
    _grid: Grid,
    entries: Vec<MenuEntry>,
    keys: MenuKeys,
    box_: Option<BoxDecor>,
    style: MenuStyle,
    active: usize,
    action: MenuAction,
}

impl Menu {
    /// Create a new menu from the given configuration.
    pub fn new(config: MenuConfig) -> Self {
        Self {
            _grid: config.grid,
            entries: config.entries,
            keys: config.keys,
            box_: config.box_,
            style: config.style,
            active: 0,
            action: MenuAction::Pass,
        }
    }

    /// Process an input message and return the resulting action.
    pub fn update(&mut self, msg: Msg) -> MenuAction {
        self.action = MenuAction::Pass;
        if self.entries.is_empty() {
            return MenuAction::Pass;
        }

        if let Msg::KeyDown { ref key, .. } = msg {
            if self.keys.up.contains(key) {
                self.move_active(-1);
                self.action = MenuAction::Move;
            } else if self.keys.down.contains(key) {
                self.move_active(1);
                self.action = MenuAction::Move;
            } else if self.keys.invoke.contains(key) {
                if !self.current_disabled() {
                    self.action = MenuAction::Invoke;
                }
            } else if self.keys.quit.contains(key) {
                self.action = MenuAction::Quit;
            } else {
                // Check per-entry shortcut keys.
                for (i, entry) in self.entries.iter().enumerate() {
                    if !entry.disabled && entry.keys.contains(key) {
                        self.active = i;
                        self.action = MenuAction::Invoke;
                        break;
                    }
                }
            }
        }

        self.action
    }

    /// Draw the menu into the given grid.
    pub fn draw(&self, grid: &Grid) {
        let inner_range = if let Some(ref box_decor) = self.box_ {
            box_decor.draw(grid)
        } else {
            grid.range_()
        };

        let start = inner_range.min;
        for (i, entry) in self.entries.iter().enumerate() {
            let y = start.y + i as i32;
            if y >= inner_range.max.y {
                break;
            }

            let is_active = i == self.active;
            let base_style = if is_active {
                self.style.active
            } else {
                entry.text.style()
            };

            // Fill the row with the style background first.
            for x in start.x..inner_range.max.x {
                let p = Point::new(x, y);
                if grid.contains(p) {
                    grid.set(p, Cell::default().with_char(' ').with_style(base_style));
                }
            }

            // Draw the entry text.
            let text = entry.text.content();
            let mut x = start.x;
            for ch in text.chars() {
                if x >= inner_range.max.x {
                    break;
                }
                let p = Point::new(x, y);
                if grid.contains(p) {
                    grid.set(p, Cell::default().with_char(ch).with_style(base_style));
                }
                x += 1;
            }
        }
    }

    /// Return the currently active (highlighted) entry index.
    pub fn active(&self) -> usize {
        self.active
    }

    /// Set the active entry index.
    pub fn set_active(&mut self, i: usize) {
        if i < self.entries.len() {
            self.active = i;
        }
    }

    /// Return the last action.
    pub fn action(&self) -> MenuAction {
        self.action
    }

    // -- private helpers --

    fn move_active(&mut self, delta: i32) {
        let len = self.entries.len() as i32;
        if len == 0 {
            return;
        }
        let mut idx = self.active as i32 + delta;
        if idx < 0 {
            idx = len - 1;
        } else if idx >= len {
            idx = 0;
        }
        self.active = idx as usize;
    }

    fn current_disabled(&self) -> bool {
        self.entries.get(self.active).is_none_or(|e| e.disabled)
    }
}
