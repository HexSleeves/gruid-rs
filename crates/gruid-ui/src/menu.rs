//! Interactive menu widget with keyboard and mouse support.

use std::collections::HashMap;

use gruid_core::messages::{Key, MouseAction, Msg};
use gruid_core::{Cell, Grid, Point, Range, Style};

use crate::{BoxDecor, StyledText};

/// An item placed in the 2D table. Maps a logical grid position to an entry.
#[derive(Debug, Clone)]
struct Item {
    /// Grid slice for this item (shares backing buffer with menu grid).
    bounds: Range,
    /// Index into `Menu::entries`.
    i: usize,
    /// Page coordinate (x-page, y-page).
    page: Point,
}

/// Internal layout classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LayoutKind {
    Column,
    Line,
    Table,
}

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
    /// 2D logical position of the active entry.
    active: Point,
    action: MenuAction,
    /// Maps logical 2D positions to items.
    table: HashMap<Point, Item>,
    /// Entry index → logical 2D position.
    points: Vec<Point>,
    /// Maximum page numbers (x, y) across all items.
    pages: Point,
    /// Computed layout (clamped copy of style.layout).
    layout: Point,
}

impl Menu {
    /// Create a new menu from the given configuration.
    pub fn new(config: MenuConfig) -> Self {
        let mut m = Self {
            grid: config.grid,
            entries: config.entries,
            keys: config.keys,
            box_: config.box_,
            style: config.style,
            active: Point::ZERO,
            action: MenuAction::Pass,
            table: HashMap::new(),
            points: Vec::new(),
            pages: Point::ZERO,
            layout: Point::ZERO,
        };
        m.place_items();
        m.cursor_at_first_choice();
        m
    }

    /// Process an input message and return the resulting action.
    pub fn update(&mut self, msg: Msg) -> MenuAction {
        self.action = MenuAction::Pass;
        if self.entries.is_empty() {
            return MenuAction::Pass;
        }

        match msg {
            Msg::KeyDown { ref key, .. } => {
                if self.keys.quit.contains(key) {
                    self.action = MenuAction::Quit;
                } else if self.keys.down.contains(key) {
                    self.move_to(Point::new(0, 1));
                } else if self.keys.up.contains(key) {
                    self.move_to(Point::new(0, -1));
                } else if self.keys.right.contains(key) {
                    self.move_to(Point::new(1, 0));
                } else if self.keys.left.contains(key) {
                    self.move_to(Point::new(-1, 0));
                } else if self.keys.page_down.contains(key) {
                    self.page_down();
                } else if self.keys.page_up.contains(key) {
                    self.page_up();
                } else if self.keys.invoke.contains(key) && self.contains_pos(self.active) {
                    if !self.current_disabled() {
                        self.action = MenuAction::Invoke;
                    }
                } else {
                    // Check per-entry shortcut keys.
                    for (i, entry) in self.entries.iter().enumerate() {
                        if !entry.disabled && entry.keys.contains(key) {
                            self.active = self.idx_to_pos(i);
                            self.action = MenuAction::Invoke;
                            break;
                        }
                    }
                }
            }
            Msg::Mouse { action, pos, .. } => {
                let outer = self.visible_range();
                let inner = self.content_range();
                let p = pos;
                match action {
                    MouseAction::Move => {
                        if inner.contains(p) {
                            self.move_to_point(p);
                        }
                    }
                    MouseAction::WheelDown => {
                        if inner.contains(p) {
                            self.page_down();
                        }
                    }
                    MouseAction::WheelUp => {
                        if inner.contains(p) {
                            self.page_up();
                        }
                    }
                    MouseAction::Main => {
                        if !outer.contains(p) {
                            self.action = MenuAction::Quit;
                        } else if inner.contains(p) {
                            self.invoke_point(p);
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }

        self.action
    }

    /// Draw the menu into its grid. Returns the visible sub-grid range.
    pub fn draw(&self) -> Range {
        let pgr = self.page_grid_range();
        let grid = self.grid.slice(pgr);

        // Draw box if present, with page number footer.
        if let Some(ref box_decor) = self.box_ {
            let pg = self.table.get(&self.active).map_or(Point::ZERO, |it| it.page);
            let lnumtext = if self.pages.x == 0 && self.pages.y == 0 {
                String::new()
            } else if self.pages.x == 0 {
                format!("{}/{}", pg.y, self.pages.y)
            } else if self.pages.y == 0 {
                format!("{}/{}", pg.x, self.pages.x)
            } else {
                format!("{},{}/{},{}", pg.x, pg.y, self.pages.x, self.pages.y)
            };
            if !lnumtext.is_empty() && box_decor.footer.content().is_empty() {
                let mut bd = box_decor.clone();
                bd.footer = StyledText::new(&lnumtext, self.style.page_num);
                bd.draw(&grid);
            } else {
                box_decor.draw(&grid);
            }
        }

        let active_item = self.table.get(&self.active);
        let active_page = active_item.map_or(Point::ZERO, |it| it.page);

        for (&pos, it) in &self.table {
            if it.page != active_page { continue; }
            let entry = &self.entries[it.i];
            let mut st = entry.text.style();
            let is_active = pos == self.active && !entry.disabled;

            if is_active {
                if self.style.active.fg != gruid_core::Color::DEFAULT {
                    st.fg = self.style.active.fg;
                }
                if self.style.active.bg != gruid_core::Color::DEFAULT {
                    st.bg = self.style.active.bg;
                }
                if self.style.active.attrs != gruid_core::AttrMask::NONE {
                    st.attrs = self.style.active.attrs;
                }
            }

            // Fill the item area and draw text into it.
            let item_grid = grid.slice(it.bounds);
            let fill_cell = Cell::default().with_char(' ').with_style(st);
            item_grid.fill(fill_cell);
            if is_active {
                entry.text.clone().with_style(st).draw(&item_grid);
            } else {
                entry.text.draw(&item_grid);
            }
        }

        pgr
    }

    /// Draw the menu into an externally-provided grid (legacy API).
    /// Draw the menu into an externally-provided grid (legacy API).
    pub fn draw_into(&self, _grid: &Grid) {
        self.draw();
    }

    /// Return the raw index of the currently active entry (counting disabled entries).
    pub fn active(&self) -> usize {
        self.table.get(&self.active).map_or(0, |it| it.i)
    }

    /// Set the active entry by raw index (counting disabled entries).
    pub fn set_active(&mut self, i: usize) {
        if i < self.entries.len() {
            self.active = self.idx_to_pos(i);
        }
    }

    /// Return the index of the currently active entry among only invokable
    /// (non-disabled) entries. Returns `None` if the active entry is disabled.
    ///
    /// This matches Go's `Menu.ActiveInvokable()`.
    pub fn active_invokable(&self) -> Option<usize> {
        let raw = self.active();
        if raw >= self.entries.len() || self.entries[raw].disabled {
            return None;
        }
        let mut count = 0usize;
        for e in &self.entries[..=raw] {
            if !e.disabled {
                count += 1;
            }
        }
        Some(count - 1)
    }

    /// Set the active entry to the `i`-th invokable (non-disabled) entry.
    ///
    /// The index ignores disabled entries. For example, `set_active_invokable(0)`
    /// activates the first non-disabled entry.
    ///
    /// This matches Go's `Menu.SetActiveInvokable(i)`.
    pub fn set_active_invokable(&mut self, i: usize) {
        let mut n: isize = -1;
        for (j, e) in self.entries.iter().enumerate() {
            if !e.disabled {
                n += 1;
            }
            if n == i as isize {
                self.active = self.idx_to_pos(j);
                return;
            }
        }
    }

    /// Return the last action.
    pub fn action(&self) -> MenuAction {
        self.action
    }

    /// Replace the entries.
    pub fn set_entries(&mut self, entries: Vec<MenuEntry>) {
        self.entries = entries;
        self.place_items();
        if !self.table.contains_key(&self.active) {
            self.cursor_at_last_choice();
        }
    }

    /// Replace the box decoration.
    pub fn set_box(&mut self, box_: Option<BoxDecor>) {
        self.box_ = box_;
        self.place_items();
    }

    /// Bounding range of the active entry (relative to the grid).
    pub fn active_bounds(&self) -> Range {
        self.table.get(&self.active).map_or(Range::default(), |it| it.bounds)
    }

    /// Bounding range of the visible menu area (including box).
    pub fn bounds(&self) -> Range {
        self.page_grid_range()
    }

    /// Current page number (0-based, Y-page for column/table, X-page for line).
    pub fn page(&self) -> usize {
        let pg = self.cur_page();
        if self.pages.y > 0 { pg.y as usize } else { pg.x as usize }
    }

    /// Total number of pages.
    pub fn page_count(&self) -> usize {
        let p = if self.pages.y > 0 { self.pages.y } else { self.pages.x };
        (p + 1) as usize
    }

    // ---------------------------------------------------------------
    // Private helpers: index/position conversion
    // ---------------------------------------------------------------

    fn idx_to_pos(&self, i: usize) -> Point {
        if i < self.points.len() { self.points[i] } else { Point::ZERO }
    }

    fn contains_pos(&self, p: Point) -> bool {
        self.table.contains_key(&p)
    }

    fn current_disabled(&self) -> bool {
        self.table.get(&self.active).map_or(true, |it| self.entries[it.i].disabled)
    }

    fn cur_page(&self) -> Point {
        self.table.get(&self.active).map_or(Point::ZERO, |it| it.page)
    }

    // ---------------------------------------------------------------
    // Cursor placement
    // ---------------------------------------------------------------

    fn cursor_at_first_choice(&mut self) {
        let mut j = 0;
        for (i, e) in self.entries.iter().enumerate() {
            if !e.disabled { j = i; break; }
        }
        self.active = self.idx_to_pos(j);
    }

    fn cursor_at_last_choice(&mut self) {
        let mut j = self.entries.len().saturating_sub(1);
        for (i, e) in self.entries.iter().enumerate() {
            if !e.disabled { j = i; }
        }
        self.active = self.idx_to_pos(j);
    }

    // ---------------------------------------------------------------
    // Movement
    // ---------------------------------------------------------------

    /// Move in direction `d` (unit vector), skipping disabled entries.
    /// Wraps at boundaries via page changes or cycling to start/end.
    fn move_to(&mut self, d: Point) {
        let old = self.active;
        let mut q = self.active;
        loop {
            q = q + d;
            match self.table.get(&q) {
                None => break,
                Some(it) if !self.entries[it.i].disabled => break,
                _ => {} // disabled: keep going
            }
        }
        if self.contains_pos(q) {
            self.active = q;
        } else if let Some(nq) = self.find_next_page(d) {
            self.active = nq;
        } else {
            // wrap
            match (d.x, d.y) {
                (0, 1) | (1, 0) => self.cursor_at_first_choice(),
                (0, -1) | (-1, 0) => self.cursor_at_last_choice(),
                _ => {}
            }
        }
        if self.active != old {
            self.action = MenuAction::Move;
        }
    }

    /// Find the first entry on the next page in direction `d`.
    fn find_next_page(&self, d: Point) -> Option<Point> {
        let it = self.table.get(&self.active)?;
        let cur_page = it.page;
        let cur_i = it.i;
        match (d.x, d.y) {
            (0, 1) => {
                for i in (cur_i + 1)..self.entries.len() {
                    let q = self.idx_to_pos(i);
                    if self.table[&q].page.y > cur_page.y { return Some(q); }
                }
            }
            (1, 0) => {
                for i in (cur_i + 1)..self.entries.len() {
                    let q = self.idx_to_pos(i);
                    if self.table[&q].page.x > cur_page.x { return Some(q); }
                }
            }
            (0, -1) => {
                for i in (0..cur_i).rev() {
                    let q = self.idx_to_pos(i);
                    if self.table[&q].page.y < cur_page.y { return Some(q); }
                }
            }
            (-1, 0) => {
                for i in (0..cur_i).rev() {
                    let q = self.idx_to_pos(i);
                    if self.table[&q].page.x < cur_page.x { return Some(q); }
                }
            }
            _ => {}
        }
        None
    }

    fn page_down(&mut self) {
        let d = if self.pages.y > 0 { Point::new(0, 1) } else { Point::new(1, 0) };
        if let Some(q) = self.find_next_page(d) {
            self.active = q;
            self.action = MenuAction::Move;
        }
    }

    fn page_up(&mut self) {
        let d = if self.pages.y > 0 { Point::new(0, -1) } else { Point::new(-1, 0) };
        if let Some(q) = self.find_next_page(d) {
            // find_next_page returns the last entry on the prev page;
            // walk forward to find the first entry on that same page.
            let target_page = self.table[&q].page;
            let mut first = q;
            for i in 0..self.entries.len() {
                let p = self.idx_to_pos(i);
                if self.table[&p].page == target_page {
                    first = p;
                    break;
                }
            }
            self.active = first;
            self.action = MenuAction::Move;
        }
    }



    // ---------------------------------------------------------------
    // Mouse helpers
    // ---------------------------------------------------------------

    /// Visible range of the menu for the current page (including box).
    fn page_grid_range(&self) -> Range {
        let active_page = self.table.get(&self.active).map_or(Point::ZERO, |it| it.page);

        if self.layout.y > 0 && self.layout.x == 0 {
            // Column with row-limit: union of bounds on this page
            let mut rg = Range::default();
            for it in self.table.values() {
                if it.page == active_page {
                    rg = rg.union(it.bounds);
                }
            }
            if self.box_.is_some() {
                rg = rg.shift(-1, -1, 1, 1);
            }
            return rg;
        }

        // Count visible rows on current page (column 0 only)
        let mut h = 0i32;
        for (p, it) in &self.table {
            if p.x > 0 { continue; }
            if it.page != active_page { continue; }
            h += 1;
        }
        if self.box_.is_some() { h += 2; }
        let max = self.grid.size();
        Range::new(0, 0, max.x, h)
    }

    fn visible_range(&self) -> Range {
        self.page_grid_range()
    }

    fn content_range(&self) -> Range {
        let outer = self.visible_range();
        if self.box_.is_some() {
            outer.shift(1, 1, -1, -1)
        } else {
            outer
        }
    }

    fn move_to_point(&mut self, p: Point) {
        let page = self.table.get(&self.active).map_or(Point::ZERO, |it| it.page);
        for (&q, it) in &self.table {
            if it.page == page && it.bounds.contains(p) {
                if q == self.active { return; }
                self.active = q;
                self.action = MenuAction::Move;
                return;
            }
        }
    }

    fn invoke_point(&mut self, p: Point) {
        let page = self.table.get(&self.active).map_or(Point::ZERO, |it| it.page);
        for (&q, it) in &self.table {
            if it.page == page && it.bounds.contains(p) {
                self.active = q;
                if self.entries[it.i].disabled {
                    self.action = MenuAction::Move;
                } else {
                    self.action = MenuAction::Invoke;
                }
                return;
            }
        }
    }

    // ---------------------------------------------------------------
    // Layout engine
    // ---------------------------------------------------------------

    fn update_layout(&mut self) {
        self.layout = self.style.layout;
        let gs = self.grid.size();
        let n = self.entries.len() as i32;
        if self.layout.y > gs.y { self.layout.y = gs.y; }
        if self.layout.y > n   { self.layout.y = n; }
        if self.layout.x > n   { self.layout.x = n; }
    }

    fn get_layout(&self, w: i32, _h: i32) -> (LayoutKind, i32, i32) {
        let n = self.entries.len() as i32;
        let mut lines = self.layout.y;
        let nw = w;
        if lines <= 0 { lines = n; }
        let mut columns = self.layout.x;
        if columns <= 0 {
            columns = if lines == n { 1 } else { n };
        }
        if lines * columns > n {
            columns = if lines > 0 { n / lines } else { 1 };
        }
        if columns < 1 { columns = 1; }
        if columns > 1 && lines > 1 {
            (LayoutKind::Table, nw / columns, columns)
        } else if columns > 1 {
            (LayoutKind::Line, nw, columns)
        } else {
            (LayoutKind::Column, nw, 1)
        }
    }

    fn place_items(&mut self) {
        self.update_layout();

        // Compute draw-grid height.
        let mut h = self.entries.len() as i32;
        if self.layout.y > 0 { h = self.layout.y; }
        if self.box_.is_some() { h += 2; }
        let gs = self.grid.size();
        let draw_h = h.min(gs.y);

        // Inner dimensions (inside box).
        let (inner_x, inner_y, inner_w, inner_h) = if self.box_.is_some() {
            (1, 1, (gs.x - 2).max(0), (draw_h - 2).max(0))
        } else {
            (0, 0, gs.x, draw_h)
        };

        let (kind, col_w, columns) = self.get_layout(inner_w, inner_h);

        self.table.clear();
        self.points.clear();
        self.pages = Point::ZERO;

        let cw = col_w.max(1);
        let ch = inner_h.max(1);

        match kind {
            LayoutKind::Column => {
                for i in 0..self.entries.len() {
                    let row_in_page = (i as i32) % ch;
                    let page_y = (i as i32) / ch;
                    let pos = Point::new(0, i as i32);
                    let bounds = Range::new(
                        inner_x, inner_y + row_in_page,
                        inner_x + cw, inner_y + row_in_page + 1,
                    );
                    self.table.insert(pos, Item { bounds, i, page: Point::new(0, page_y) });
                    self.points.push(pos);
                }
            }
            LayoutKind::Line => {
                let mut to = 0i32;
                let mut hpage = 0i32;
                for i in 0..self.entries.len() {
                    let from = to;
                    let tw = self.entries[i].text.size().x;
                    to += tw;
                    let (from, new_to) = if from > 0 && to > inner_w {
                        hpage += 1;
                        (0i32, tw)
                    } else {
                        (from, to)
                    };
                    to = new_to;
                    let pos = Point::new(i as i32, 0);
                    let bounds = Range::new(
                        inner_x + from, inner_y,
                        inner_x + to, inner_y + 1,
                    );
                    self.table.insert(pos, Item { bounds, i, page: Point::new(hpage, 0) });
                    self.points.push(pos);
                }
            }
            LayoutKind::Table => {
                let h = ch;
                for i in 0..self.entries.len() {
                    let page = (i as i32) / (columns * h);
                    let pageidx = (i as i32) % (columns * h);
                    let ln = pageidx % h;
                    let col = pageidx / h;
                    let pos = Point::new(col, ln + page * h);
                    let bounds = Range::new(
                        inner_x + col * cw, inner_y + ln,
                        inner_x + (col + 1) * cw, inner_y + ln + 1,
                    );
                    self.table.insert(pos, Item { bounds, i, page: Point::new(0, page) });
                    self.points.push(pos);
                }
            }
        }

        // Update max pages.
        for it in self.table.values() {
            if it.page.x > self.pages.x { self.pages.x = it.page.x; }
            if it.page.y > self.pages.y { self.pages.y = it.page.y; }
        }
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

        menu.set_entries(vec![MenuEntry::new(StyledText::new(
            "New",
            Style::default(),
        ))]);
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

    #[test]
    fn active_invokable_basic() {
        let mut entries: Vec<MenuEntry> = (0..5)
            .map(|i| MenuEntry::new(StyledText::new(&format!("Item {i}"), Style::default())))
            .collect();
        entries[1].disabled = true;
        entries[3].disabled = true;

        let mut menu = Menu::new(MenuConfig {
            grid: Grid::new(20, 10),
            entries,
            keys: MenuKeys::default(),
            box_: None,
            style: MenuStyle::default(),
        });

        // active=0 ("Item 0", enabled) → invokable index 0
        assert_eq!(menu.active(), 0);
        assert_eq!(menu.active_invokable(), Some(0));

        // move to index 2 ("Item 2", enabled, skipping disabled 1) → invokable index 1
        menu.set_active(2);
        assert_eq!(menu.active_invokable(), Some(1));

        // move to index 4 ("Item 4", enabled, skipping disabled 1,3) → invokable index 2
        menu.set_active(4);
        assert_eq!(menu.active_invokable(), Some(2));

        // move to disabled entry → None
        menu.set_active(1);
        assert_eq!(menu.active_invokable(), None);

        menu.set_active(3);
        assert_eq!(menu.active_invokable(), None);
    }

    #[test]
    fn set_active_invokable_basic() {
        let mut entries: Vec<MenuEntry> = (0..5)
            .map(|i| MenuEntry::new(StyledText::new(&format!("Item {i}"), Style::default())))
            .collect();
        entries[0].disabled = true;
        entries[2].disabled = true;
        // Invokable entries: 1, 3, 4 (indices 0, 1, 2 among invokable)

        let mut menu = Menu::new(MenuConfig {
            grid: Grid::new(20, 10),
            entries,
            keys: MenuKeys::default(),
            box_: None,
            style: MenuStyle::default(),
        });

        menu.set_active_invokable(0);
        assert_eq!(menu.active(), 1);

        menu.set_active_invokable(1);
        assert_eq!(menu.active(), 3);

        menu.set_active_invokable(2);
        assert_eq!(menu.active(), 4);

        // out of range: no change
        menu.set_active_invokable(10);
        assert_eq!(menu.active(), 4);
    }

    fn mouse_msg(action: MouseAction, x: i32, y: i32) -> Msg {
        Msg::Mouse {
            action,
            pos: Point::new(x, y),
            modifiers: Default::default(),
            time: Instant::now(),
        }
    }

    #[test]
    fn mouse_click_outside_quits() {
        let mut menu = make_menu(3, 5);
        // Click at y=10 which is outside the 5-row grid
        let action = menu.update(mouse_msg(MouseAction::Main, 0, 10));
        assert_eq!(action, MenuAction::Quit);
    }

    #[test]
    fn mouse_wheel_pages() {
        let mut menu = make_menu(10, 3); // 3 visible rows, ceil(10/3) = 4 pages
        assert_eq!(menu.page(), 0);
        assert_eq!(menu.active(), 0);

        // Wheel down inside content area
        let action = menu.update(mouse_msg(MouseAction::WheelDown, 1, 1));
        assert_eq!(action, MenuAction::Move);
        assert_eq!(menu.page(), 1);
        assert_eq!(menu.active(), 3);

        // Wheel up
        let action = menu.update(mouse_msg(MouseAction::WheelUp, 1, 1));
        assert_eq!(action, MenuAction::Move);
        assert_eq!(menu.page(), 0);
        assert_eq!(menu.active(), 0);

        // Wheel up at page 0 → no change (Pass)
        let action = menu.update(mouse_msg(MouseAction::WheelUp, 1, 1));
        assert_eq!(action, MenuAction::Pass);
    }

    #[test]
    fn mouse_move_highlights() {
        let mut menu = make_menu(5, 10);
        assert_eq!(menu.active(), 0);

        let action = menu.update(mouse_msg(MouseAction::Move, 5, 3));
        assert_eq!(action, MenuAction::Move);
        assert_eq!(menu.active(), 3);

        // Moving to same entry → Pass
        let action = menu.update(mouse_msg(MouseAction::Move, 2, 3));
        assert_eq!(action, MenuAction::Pass);
    }

    #[test]
    fn mouse_click_invokes() {
        let mut menu = make_menu(5, 10);
        let action = menu.update(mouse_msg(MouseAction::Main, 5, 2));
        assert_eq!(action, MenuAction::Invoke);
        assert_eq!(menu.active(), 2);
    }

    #[test]
    fn mouse_click_disabled_entry() {
        let mut entries: Vec<MenuEntry> = (0..5)
            .map(|i| MenuEntry::new(StyledText::new(&format!("Item {i}"), Style::default())))
            .collect();
        entries[2].disabled = true;

        let mut menu = Menu::new(MenuConfig {
            grid: Grid::new(20, 10),
            entries,
            keys: MenuKeys::default(),
            box_: None,
            style: MenuStyle::default(),
        });

        // Click on disabled entry → Move (not Invoke)
        let action = menu.update(mouse_msg(MouseAction::Main, 5, 2));
        assert_eq!(action, MenuAction::Move);
        assert_eq!(menu.active(), 2);
    }

    #[test]
    fn mouse_click_on_box_border() {
        let mut menu = Menu::new(MenuConfig {
            grid: Grid::new(20, 10),
            entries: (0..3)
                .map(|i| MenuEntry::new(StyledText::new(&format!("Item {i}"), Style::default())))
                .collect(),
            keys: MenuKeys::default(),
            box_: Some(BoxDecor::new()),
            style: MenuStyle::default(),
        });

        // Click on top border (y=0) - inside outer but outside inner → Pass (not Quit)
        let action = menu.update(mouse_msg(MouseAction::Main, 5, 0));
        assert_eq!(action, MenuAction::Pass);

        // Click outside the box entirely
        let action = menu.update(mouse_msg(MouseAction::Main, 5, 20));
        assert_eq!(action, MenuAction::Quit);
    }
}
