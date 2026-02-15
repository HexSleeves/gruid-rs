//! The [`Cell`] type — a single character with styling.

use crate::style::Style;

/// A styled character cell.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Cell {
    pub ch: char,
    pub style: Style,
}

impl Cell {
    /// Set the character (builder).
    #[inline]
    pub const fn with_char(mut self, ch: char) -> Self {
        self.ch = ch;
        self
    }

    /// Set the style (builder).
    #[inline]
    pub const fn with_style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }
}

impl Default for Cell {
    #[inline]
    fn default() -> Self {
        Self {
            ch: ' ',
            style: Style::default(),
        }
    }
}
