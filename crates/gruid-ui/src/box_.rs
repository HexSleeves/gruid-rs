use gruid_core::{Cell, Grid, Point, Range, Style};

use crate::StyledText;

/// Alignment for box title and footer text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Alignment {
    #[default]
    Center,
    Left,
    Right,
}

/// Decoration for a box drawn around a widget.
#[derive(Debug, Clone)]
pub struct BoxDecor {
    /// Style for the box border characters.
    pub style: Style,
    /// Title text drawn on the top border.
    pub title: StyledText,
    /// Footer text drawn on the bottom border.
    pub footer: StyledText,
    /// Alignment of the title on the top border.
    pub align_title: Alignment,
    /// Alignment of the footer on the bottom border.
    pub align_footer: Alignment,
}

impl BoxDecor {
    /// Create a new box decoration with default style and no title/footer.
    pub fn new() -> Self {
        Self {
            style: Style::default(),
            title: StyledText::text(""),
            footer: StyledText::text(""),
            align_title: Alignment::Center,
            align_footer: Alignment::Center,
        }
    }

    /// Draw the box border into the grid, using relative coordinates.
    /// Returns the inner range (relative, the area inside the border).
    pub fn draw(&self, grid: &Grid) -> Range {
        let w = grid.width();
        let h = grid.height();
        if w < 2 || h < 2 {
            return grid.range_();
        }

        let x1 = w;
        let y1 = h;
        let s = self.style;

        // Corners
        set(grid, Point::new(0, 0), '\u{250c}', s);
        set(grid, Point::new(x1 - 1, 0), '\u{2510}', s);
        set(grid, Point::new(0, y1 - 1), '\u{2514}', s);
        set(grid, Point::new(x1 - 1, y1 - 1), '\u{2518}', s);

        // Top and bottom borders
        for x in 1..(x1 - 1) {
            set(grid, Point::new(x, 0), '\u{2500}', s);
            set(grid, Point::new(x, y1 - 1), '\u{2500}', s);
        }

        // Left and right borders
        for y in 1..(y1 - 1) {
            set(grid, Point::new(0, y), '\u{2502}', s);
            set(grid, Point::new(x1 - 1, y), '\u{2502}', s);
        }

        // Draw title on top border
        let title_text = self.title.content();
        if !title_text.is_empty() {
            let title_len = title_text.chars().count() as i32;
            let available = (w - 2).max(0);
            if title_len > 0 && available > 0 {
                let start_x = match self.align_title {
                    Alignment::Left => 1,
                    Alignment::Right => (x1 - 1 - title_len).max(1),
                    Alignment::Center => ((w - title_len) / 2).max(1),
                };
                let mut x = start_x;
                for ch in title_text.chars() {
                    if x >= x1 - 1 {
                        break;
                    }
                    set(grid, Point::new(x, 0), ch, self.title.style());
                    x += 1;
                }
            }
        }

        // Draw footer on bottom border
        let footer_text = self.footer.content();
        if !footer_text.is_empty() {
            let footer_len = footer_text.chars().count() as i32;
            let available = (w - 2).max(0);
            if footer_len > 0 && available > 0 {
                let start_x = match self.align_footer {
                    Alignment::Left => 1,
                    Alignment::Right => (x1 - 1 - footer_len).max(1),
                    Alignment::Center => ((w - footer_len) / 2).max(1),
                };
                let mut x = start_x;
                for ch in footer_text.chars() {
                    if x >= x1 - 1 {
                        break;
                    }
                    set(grid, Point::new(x, y1 - 1), ch, self.footer.style());
                    x += 1;
                }
            }
        }

        // Inner range (relative)
        Range::new(1, 1, x1 - 1, y1 - 1)
    }
}

impl Default for BoxDecor {
    fn default() -> Self {
        Self::new()
    }
}

fn set(grid: &Grid, p: Point, ch: char, style: Style) {
    if grid.contains(p) {
        grid.set(p, Cell::default().with_char(ch).with_style(style));
    }
}
