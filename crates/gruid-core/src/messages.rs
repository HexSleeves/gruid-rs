//! Input events: [`Msg`], [`Key`], [`MouseAction`], [`ModMask`].

use std::time::Instant;

use crate::geom::Point;

// ---------------------------------------------------------------------------
// Key
// ---------------------------------------------------------------------------

/// A keyboard key.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Key {
    ArrowUp,
    ArrowDown,
    ArrowLeft,
    ArrowRight,
    Escape,
    Enter,
    Tab,
    Space,
    Backspace,
    Delete,
    Home,
    End,
    PageUp,
    PageDown,
    Insert,
    /// A printable character.
    Char(char),
}

// ---------------------------------------------------------------------------
// ModMask
// ---------------------------------------------------------------------------

/// Bitmask of modifier keys held during an input event.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ModMask(pub u8);

impl ModMask {
    pub const NONE: Self = Self(0);
    pub const SHIFT: Self = Self(1 << 0);
    pub const CTRL: Self = Self(1 << 1);
    pub const ALT: Self = Self(1 << 2);
    pub const META: Self = Self(1 << 3);

    /// Whether this mask contains all bits of `other`.
    #[inline]
    pub const fn contains(self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    #[inline]
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }
}

impl std::ops::BitOr for ModMask {
    type Output = Self;
    #[inline]
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

impl std::ops::BitAnd for ModMask {
    type Output = Self;
    #[inline]
    fn bitand(self, rhs: Self) -> Self {
        Self(self.0 & rhs.0)
    }
}

// ---------------------------------------------------------------------------
// MouseAction
// ---------------------------------------------------------------------------

/// A mouse action.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum MouseAction {
    /// Primary (left) button pressed.
    Main,
    /// Middle button pressed.
    Auxiliary,
    /// Secondary (right) button pressed.
    Secondary,
    WheelUp,
    WheelDown,
    /// Button released.
    Release,
    /// Mouse moved (no button state change).
    Move,
}

// ---------------------------------------------------------------------------
// Msg
// ---------------------------------------------------------------------------

/// An input message delivered to the application.
#[derive(Clone, Debug)]
pub enum Msg {
    /// A key was pressed.
    KeyDown {
        key: Key,
        modifiers: ModMask,
        time: Instant,
    },
    /// A mouse event.
    Mouse {
        action: MouseAction,
        pos: Point,
        modifiers: ModMask,
        time: Instant,
    },
    /// The screen / terminal was resized.
    Screen {
        width: i32,
        height: i32,
        time: Instant,
    },
    /// Sent once when the application starts.
    Init,
    /// Request to quit.
    Quit,
}

impl Msg {
    /// Convenience: create a `KeyDown` with no modifiers.
    pub fn key(key: Key) -> Self {
        Self::KeyDown {
            key,
            modifiers: ModMask::NONE,
            time: Instant::now(),
        }
    }

    /// Convenience: create a `KeyDown` with modifiers.
    pub fn key_mod(key: Key, modifiers: ModMask) -> Self {
        Self::KeyDown {
            key,
            modifiers,
            time: Instant::now(),
        }
    }
}
