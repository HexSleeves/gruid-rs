use gruid_core::{Cell, Grid, Range};

use crate::{BoxDecor, StyledText};

/// A simple label widget that displays styled text, optionally inside a box.
///
/// When drawn, the label fills its area with the base style's background
/// before rendering content — matching Go gruid's `Label.Draw` behaviour.
#[derive(Debug, Clone)]
pub struct Label {
    /// The content to display.
    pub content: StyledText,
    /// Optional box decoration.
    pub box_: Option<BoxDecor>,
    /// Whether to adjust the label width to fit the content.
    ///
    /// When `true` (the default) the returned draw area is shrunk to the
    /// actual content width (plus box borders if present). When `false`
    /// the full grid width is used.
    pub adjust_width: bool,
}

impl Label {
    /// Create a new label with the given content (`adjust_width` is `true` by
    /// default).
    pub fn new(content: StyledText) -> Self {
        Self {
            content,
            box_: None,
            adjust_width: true,
        }
    }

    /// Set the text content (preserves other settings).
    pub fn set_text(&mut self, text: &str) {
        self.content = self.content.clone().with_text(text);
    }

    /// Compute the grid range for drawing, matching Go's `drawGrid`.
    fn draw_range(&self, grid: &Grid) -> Range {
        let content_size = self.content.size();
        let mut w = content_size.x;
        let mut h = content_size.y;

        // Title may widen the label.
        if let Some(ref box_decor) = self.box_ {
            let ts = box_decor.title.size();
            if w < ts.x {
                w = ts.x;
            }
        }

        if self.box_.is_some() {
            h += 2; // border top + bottom
            w += 2; // border left + right
        }

        if !self.adjust_width {
            w = grid.width();
        }

        Range::new(0, 0, w, h)
    }

    /// Draw the label into the grid.  Returns the range of cells drawn.
    ///
    /// The entire label area is first filled with the content's base style
    /// background, then the styled text is rendered on top. If `adjust_width`
    /// is true the returned range is shrunk to the content width.
    pub fn draw(&self, grid: &Grid) -> Range {
        let draw_rg = self.draw_range(grid);
        let draw_grid = grid.slice(draw_rg);

        let content_grid = if let Some(ref box_decor) = self.box_ {
            box_decor.draw(&draw_grid);
            let rg = draw_grid.range_();
            draw_grid.slice(rg.shift(1, 1, -1, -1))
        } else {
            draw_grid.clone()
        };

        // Fill the content area with the base style background.
        content_grid.fill(Cell::default().with_char(' ').with_style(self.content.style()));

        self.content.draw(&content_grid);
        draw_rg
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gruid_core::{Point, Style};
    use gruid_core::style::{AttrMask, Color};

    #[test]
    fn background_fill() {
        let style = Style { fg: Color(1), bg: Color(2), attrs: AttrMask(0) };
        let label = Label {
            content: StyledText::new("Hi", style),
            box_: None,
            adjust_width: false,
        };
        let grid = Grid::new(5, 1);
        label.draw(&grid);
        // Cells under the text should have the content.
        assert_eq!(grid.at(Point::new(0, 0)).ch, 'H');
        assert_eq!(grid.at(Point::new(1, 0)).ch, 'i');
        // Cells past the text should be space-filled with the base style.
        assert_eq!(grid.at(Point::new(2, 0)).ch, ' ');
        assert_eq!(grid.at(Point::new(2, 0)).style, style);
        assert_eq!(grid.at(Point::new(4, 0)).ch, ' ');
        assert_eq!(grid.at(Point::new(4, 0)).style, style);
    }

    #[test]
    fn adjust_width_true_shrinks_range() {
        let label = Label {
            content: StyledText::new("Hi", Style::default()),
            box_: None,
            adjust_width: true,
        };
        let grid = Grid::new(10, 3);
        let rg = label.draw(&grid);
        // "Hi" is 2 chars wide, 1 line tall.
        assert_eq!(rg, Range::new(0, 0, 2, 1));
    }

    #[test]
    fn adjust_width_false_uses_full_width() {
        let label = Label {
            content: StyledText::new("Hi", Style::default()),
            box_: None,
            adjust_width: false,
        };
        let grid = Grid::new(10, 3);
        let rg = label.draw(&grid);
        // Full grid width, content height.
        assert_eq!(rg, Range::new(0, 0, 10, 1));
    }

    #[test]
    fn adjust_width_with_box() {
        let label = Label {
            content: StyledText::new("Hi", Style::default()),
            box_: Some(BoxDecor::new()),
            adjust_width: true,
        };
        let grid = Grid::new(20, 10);
        let rg = label.draw(&grid);
        // 2 (content) + 2 (borders) = 4 wide, 1 + 2 = 3 tall
        assert_eq!(rg, Range::new(0, 0, 4, 3));
    }
}
