//! Visual styling: [`Color`], [`AttrMask`], and [`Style`].

use std::ops::{BitAnd, BitOr};

// ---------------------------------------------------------------------------
// Color
// ---------------------------------------------------------------------------

/// An RGB colour packed into a `u32` (0x00RRGGBB).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Color(pub u32);

impl Color {
    /// The default / unset colour (0).
    pub const DEFAULT: Self = Self(0);

    /// Construct from individual RGB components.
    #[inline]
    pub const fn from_rgb(r: u8, g: u8, b: u8) -> Self {
        Self(((r as u32) << 16) | ((g as u32) << 8) | (b as u32))
    }

    /// Red component.
    #[inline]
    pub const fn r(self) -> u8 {
        ((self.0 >> 16) & 0xFF) as u8
    }

    /// Green component.
    #[inline]
    pub const fn g(self) -> u8 {
        ((self.0 >> 8) & 0xFF) as u8
    }

    /// Blue component.
    #[inline]
    pub const fn b(self) -> u8 {
        (self.0 & 0xFF) as u8
    }
}

// ---------------------------------------------------------------------------
// AttrMask
// ---------------------------------------------------------------------------

/// Bitmask of text attributes.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct AttrMask(pub u32);

impl AttrMask {
    pub const NONE: Self = Self(0);
    pub const BOLD: Self = Self(1 << 0);
    pub const ITALIC: Self = Self(1 << 1);
    pub const UNDERLINE: Self = Self(1 << 2);
    pub const BLINK: Self = Self(1 << 3);
    pub const REVERSE: Self = Self(1 << 4);
    pub const DIM: Self = Self(1 << 5);

    /// Whether this mask contains all the bits from `other`.
    #[inline]
    pub const fn contains(self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    /// Whether the mask is empty.
    #[inline]
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }
}

impl BitOr for AttrMask {
    type Output = Self;
    #[inline]
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

impl BitAnd for AttrMask {
    type Output = Self;
    #[inline]
    fn bitand(self, rhs: Self) -> Self {
        Self(self.0 & rhs.0)
    }
}

// ---------------------------------------------------------------------------
// Style
// ---------------------------------------------------------------------------

/// Complete visual style for a single cell.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Style {
    pub fg: Color,
    pub bg: Color,
    pub attrs: AttrMask,
}

impl Style {
    /// Set the foreground colour (builder).
    #[inline]
    pub const fn with_fg(mut self, fg: Color) -> Self {
        self.fg = fg;
        self
    }

    /// Set the background colour (builder).
    #[inline]
    pub const fn with_bg(mut self, bg: Color) -> Self {
        self.bg = bg;
        self
    }

    /// Set the attribute mask (builder).
    #[inline]
    pub const fn with_attrs(mut self, attrs: AttrMask) -> Self {
        self.attrs = attrs;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn color_round_trip() {
        let c = Color::from_rgb(0xAB, 0xCD, 0xEF);
        assert_eq!(c.r(), 0xAB);
        assert_eq!(c.g(), 0xCD);
        assert_eq!(c.b(), 0xEF);
    }

    #[test]
    fn attr_mask_ops() {
        let m = AttrMask::BOLD | AttrMask::ITALIC;
        assert!(m.contains(AttrMask::BOLD));
        assert!(m.contains(AttrMask::ITALIC));
        assert!(!m.contains(AttrMask::UNDERLINE));
        assert_eq!(m & AttrMask::BOLD, AttrMask::BOLD);
    }

    #[test]
    fn style_builder() {
        let s = Style::default()
            .with_fg(Color::from_rgb(255, 0, 0))
            .with_bg(Color::from_rgb(0, 0, 0))
            .with_attrs(AttrMask::BOLD);
        assert_eq!(s.fg.r(), 255);
        assert!(s.attrs.contains(AttrMask::BOLD));
    }
}
