//! Interactive menu widget with keyboard and mouse support.

use gruid_core::messages::{Key, MouseAction, Msg};
use gruid_core::{Cell, Grid, Point, Range, Style};

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
    pub page_up: Vec<Key>,
    pub page_down: Vec<Key>,
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
            page_up: vec![Key::PageUp],
            page_down: vec![Key::PageDown],
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
    /// Style for disabled entries.
    pub disabled: Style,
    /// Style for page number indicator.
    pub page_num: Style,
}

impl Default for MenuStyle {
    fn default() -> Self {
        Self {
            layout: Point::new(1, 0),
            active: Style::default(),
            disabled: Style::default(),
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
    grid: Grid,
    entries: Vec<MenuEntry>,
    keys: MenuKeys,
    box_: Option<BoxDecor>,
    style: MenuStyle,
    active: usize,
    page: usize,
    action: MenuAction,
}

impl Menu {
    /// Create a new menu from the given configuration.
    pub fn new(config: MenuConfig) -> Self {
        Self {
            grid: config.grid,
            entries: config.entries,
            keys: config.keys,
            box_: config.box_,
            style: config.style,
            active: 0,
            page: 0,
            action: MenuAction::Pass,
        }
    }

    /// Process an input message and return the resulting action.
    pub fn update(&mut self, msg: Msg) -> MenuAction {
        self.action = MenuAction::Pass;
        if self.entries.is_empty() {
            return MenuAction::Pass;
        }

        match msg {
            Msg::KeyDown { ref key, .. } => {
                if self.keys.up.contains(key) {
                    self.move_active(-1);
                    self.action = MenuAction::Move;
                } else if self.keys.down.contains(key) {
                    self.move_active(1);
                    self.action = MenuAction::Move;
                } else if self.keys.page_up.contains(key) {
                    self.prev_page();
                    self.action = MenuAction::Move;
                } else if self.keys.page_down.contains(key) {
                    self.next_page();
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
                            self.update_page_for_active();
                            self.action = MenuAction::Invoke;
                            break;
                        }
                    }
                }
            }
            Msg::Mouse {
                action, pos, ..
            } => {
                let inner = self.inner_range();
                let rel = Point::new(pos.x - inner.min.x, pos.y - inner.min.y);
                if inner.contains(pos) {
                    let row = rel.y as usize;
                    let idx = self.page_start() + row;
                    if idx < self.entries.len() {
                        match action {
                            MouseAction::Move | MouseAction::Main => {
                                self.active = idx;
                            }
                            _ => {}
                        }
                        if action == MouseAction::Main && !self.entries[idx].disabled {
                            self.action = MenuAction::Invoke;
                        } else if action == MouseAction::Move {
                            self.action = MenuAction::Move;
                        }
                    }
                }
            }
            _ => {}
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
        let vis_h = (inner_range.max.y - inner_range.min.y) as usize;
        let page_start = self.page_start();

        for row in 0..vis_h {
            let idx = page_start + row;
            let y = start.y + row as i32;

            if idx >= self.entries.len() {
                break;
            }

            let entry = &self.entries[idx];
            let is_active = idx == self.active;
            let base_style = if is_active {
                self.style.active
            } else if entry.disabled {
                self.style.disabled
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
            self.update_page_for_active();
        }
    }

    /// Return the last action.
    pub fn action(&self) -> MenuAction {
        self.action
    }

    /// Replace the entries.
    pub fn set_entries(&mut self, entries: Vec<MenuEntry>) {
        self.entries = entries;
        self.active = 0;
        self.page = 0;
    }

    /// Replace the box decoration.
    pub fn set_box(&mut self, box_: Option<BoxDecor>) {
        self.box_ = box_;
    }

    /// Bounding range of the active entry (relative to the grid).
    pub fn active_bounds(&self) -> Range {
        let inner = self.inner_range();
        let row = self.active.saturating_sub(self.page_start()) as i32;
        let y = inner.min.y + row;
        Range::new(inner.min.x, y, inner.max.x, y + 1)
    }

    /// Bounding range of the entire menu content area.
    pub fn bounds(&self) -> Range {
        self.grid.range_()
    }

    /// Current page number (0-based).
    pub fn page(&self) -> usize {
        self.page
    }

    /// Total number of pages.
    pub fn page_count(&self) -> usize {
        let ps = self.page_size();
        if ps == 0 {
            return 1;
        }
        (self.entries.len() + ps - 1) / ps
    }

    // -- private helpers --

    fn inner_range(&self) -> Range {
        if self.box_.is_some() {
            let r = self.grid.range_();
            Range::new(r.min.x + 1, r.min.y + 1, r.max.x - 1, r.max.y - 1)
        } else {
            self.grid.range_()
        }
    }

    fn page_size(&self) -> usize {
        let inner = self.inner_range();
        (inner.max.y - inner.min.y).max(1) as usize
    }

    fn page_start(&self) -> usize {
        self.page * self.page_size()
    }

    fn update_page_for_active(&mut self) {
        let ps = self.page_size();
        if ps > 0 {
            self.page = self.active / ps;
        }
    }

    fn move_active(&mut self, delta: i32) {
        let len = self.entries.len() as i32;
        if len == 0 {
            return;
        }
        let mut idx = self.active as i32 + delta;

        // Wrap around
        if idx < 0 {
            idx = len - 1;
        } else if idx >= len {
            idx = 0;
        }

        // Skip disabled entries
        let start = idx;
        loop {
            if !self.entries[idx as usize].disabled {
                break;
            }
            idx += delta.signum();
            if idx < 0 {
                idx = len - 1;
            } else if idx >= len {
                idx = 0;
            }
            if idx == start {
                break; // all disabled
            }
        }

        self.active = idx as usize;
        self.update_page_for_active();
    }

    fn next_page(&mut self) {
        let max_page = self.page_count().saturating_sub(1);
        if self.page < max_page {
            self.page += 1;
            self.active = self.page_start();
            self.skip_disabled_forward();
        }
    }

    fn prev_page(&mut self) {
        if self.page > 0 {
            self.page -= 1;
            self.active = self.page_start();
            self.skip_disabled_forward();
        }
    }

    fn skip_disabled_forward(&mut self) {
        let len = self.entries.len();
        let start = self.active;
        while self.active < len && self.entries[self.active].disabled {
            self.active += 1;
        }
        if self.active >= len {
            self.active = start;
        }
    }

    fn current_disabled(&self) -> bool {
        self.entries.get(self.active).is_none_or(|e| e.disabled)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    fn make_menu(n: usize, height: i32) -> Menu {
        let entries: Vec<MenuEntry> = (0..n)
            .map(|i| MenuEntry::new(StyledText::new(&format!("Item {i}"), Style::default())))
            .collect();
        Menu::new(MenuConfig {
            grid: Grid::new(20, height),
            entries,
            keys: MenuKeys::default(),
            box_: None,
            style: MenuStyle::default(),
        })
    }

    #[test]
    fn keyboard_navigation() {
        let mut menu = make_menu(5, 10);
        assert_eq!(menu.active(), 0);

        menu.update(Msg::key(Key::ArrowDown));
        assert_eq!(menu.active(), 1);

        menu.update(Msg::key(Key::ArrowUp));
        assert_eq!(menu.active(), 0);

        // Wrap around
        menu.update(Msg::key(Key::ArrowUp));
        assert_eq!(menu.active(), 4);
    }

    #[test]
    fn disabled_entry_skip() {
        let mut entries: Vec<MenuEntry> = (0..5)
            .map(|i| MenuEntry::new(StyledText::new(&format!("Item {i}"), Style::default())))
            .collect();
        entries[1].disabled = true;
        entries[2].disabled = true;

        let mut menu = Menu::new(MenuConfig {
            grid: Grid::new(20, 10),
            entries,
            keys: MenuKeys::default(),
            box_: None,
            style: MenuStyle::default(),
        });

        assert_eq!(menu.active(), 0);
        menu.update(Msg::key(Key::ArrowDown));
        // Should skip 1 and 2, land on 3
        assert_eq!(menu.active(), 3);
    }

    #[test]
    fn pagination() {
        let mut menu = make_menu(10, 3); // 3 rows visible
        assert_eq!(menu.page(), 0);
        assert_eq!(menu.page_count(), 4); // ceil(10/3)

        menu.update(Msg::key(Key::PageDown));
        assert_eq!(menu.page(), 1);
        assert_eq!(menu.active(), 3);

        menu.update(Msg::key(Key::PageUp));
        assert_eq!(menu.page(), 0);
        assert_eq!(menu.active(), 0);
    }

    #[test]
    fn mouse_hover_and_click() {
        let mut menu = make_menu(5, 10);
        assert_eq!(menu.active(), 0);

        // Hover over row 2
        let action = menu.update(Msg::Mouse {
            action: MouseAction::Move,
            pos: Point::new(5, 2),
            modifiers: Default::default(),
            time: Instant::now(),
        });
        assert_eq!(menu.active(), 2);
        assert_eq!(action, MenuAction::Move);

        // Click
        let action = menu.update(Msg::Mouse {
            action: MouseAction::Main,
            pos: Point::new(5, 2),
            modifiers: Default::default(),
            time: Instant::now(),
        });
        assert_eq!(action, MenuAction::Invoke);
    }

    #[test]
    fn set_entries_resets() {
        let mut menu = make_menu(5, 10);
        menu.set_active(3);
        assert_eq!(menu.active(), 3);

        menu.set_entries(vec![MenuEntry::new(StyledText::new("New", Style::default()))]);
        assert_eq!(menu.active(), 0);
        assert_eq!(menu.page(), 0);
    }

    #[test]
    fn invoke_action() {
        let mut menu = make_menu(3, 10);
        let action = menu.update(Msg::key(Key::Enter));
        assert_eq!(action, MenuAction::Invoke);
    }

    #[test]
    fn quit_action() {
        let mut menu = make_menu(3, 10);
        let action = menu.update(Msg::key(Key::Escape));
        assert_eq!(action, MenuAction::Quit);
    }
}
