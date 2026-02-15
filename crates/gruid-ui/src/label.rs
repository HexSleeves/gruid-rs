use gruid_core::{Grid, Range};

use crate::{BoxDecor, StyledText};

/// A simple label widget that displays styled text, optionally inside a box.
#[derive(Debug, Clone)]
pub struct Label {
    /// The content to display.
    pub content: StyledText,
    /// Optional box decoration.
    pub box_: Option<BoxDecor>,
    /// Whether to adjust the label width to fit the content.
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

    /// Draw the label into the grid.  Returns the range of cells drawn.
    pub fn draw(&self, grid: &Grid) -> Range {
        if let Some(ref box_decor) = self.box_ {
            let inner_range = box_decor.draw(grid);
            let inner_grid = grid.slice(inner_range);
            self.content.draw(&inner_grid);
            grid.bounds()
        } else {
            self.content.draw(grid)
        }
    }
}
