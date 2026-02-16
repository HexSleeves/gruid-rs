//! Input events: [`Msg`], [`Key`], [`MouseAction`], [`ModMask`].

use std::any::Any;
use std::sync::Arc;
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

impl std::fmt::Display for Key {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ArrowUp => write!(f, "ArrowUp"),
            Self::ArrowDown => write!(f, "ArrowDown"),
            Self::ArrowLeft => write!(f, "ArrowLeft"),
            Self::ArrowRight => write!(f, "ArrowRight"),
            Self::Escape => write!(f, "Escape"),
            Self::Enter => write!(f, "Enter"),
            Self::Tab => write!(f, "Tab"),
            Self::Space => write!(f, "Space"),
            Self::Backspace => write!(f, "Backspace"),
            Self::Delete => write!(f, "Delete"),
            Self::Home => write!(f, "Home"),
            Self::End => write!(f, "End"),
            Self::PageUp => write!(f, "PageUp"),
            Self::PageDown => write!(f, "PageDown"),
            Self::Insert => write!(f, "Insert"),
            Self::Char(c) => write!(f, "Char({})", c),
        }
    }
}

// ---------------------------------------------------------------------------
// ModMask
// ---------------------------------------------------------------------------

/// Bitmask of modifier keys held during an input event.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ModMask(pub u8);

impl std::fmt::Display for ModMask {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_empty() {
            return write!(f, "None");
        }
        let mut first = true;
        let mut addmod = |name: &str| -> std::fmt::Result {
            if !first {
                write!(f, "+")?;
            }
            first = false;
            write!(f, "{}", name)
        };
        if self.contains(Self::CTRL) {
            addmod("Ctrl")?;
        }
        if self.contains(Self::ALT) {
            addmod("Alt")?;
        }
        if self.contains(Self::META) {
            addmod("Meta")?;
        }
        if self.contains(Self::SHIFT) {
            addmod("Shift")?;
        }
        Ok(())
    }
}

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

impl std::fmt::Display for MouseAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Main => write!(f, "Main"),
            Self::Auxiliary => write!(f, "Auxiliary"),
            Self::Secondary => write!(f, "Secondary"),
            Self::WheelUp => write!(f, "WheelUp"),
            Self::WheelDown => write!(f, "WheelDown"),
            Self::Release => write!(f, "Release"),
            Self::Move => write!(f, "Move"),
        }
    }
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
    /// User-defined message.
    ///
    /// Go gruid uses `interface{}` for `Msg`, so any type can be a message.
    /// In Rust the core variants cover driver events; use `Custom` for
    /// application-specific messages (timers, animation ticks, async
    /// results, etc.).
    ///
    /// Downcast with `msg.downcast_ref::<YourType>()`.
    Custom(Arc<dyn Any + Send + Sync>),
}

impl std::fmt::Display for Msg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Init => write!(f, "Init"),
            Self::Quit => write!(f, "Quit"),
            Self::KeyDown {
                key,
                modifiers,
                time,
            } => write!(
                f,
                "KeyDown {{ key: {}, modifiers: {}, time: {} }}",
                key,
                modifiers,
                time.elapsed().as_secs()
            ),
            Self::Mouse {
                action,
                pos,
                modifiers,
                time,
            } => write!(
                f,
                "Mouse {{ action: {}, pos: {}, modifiers: {}, time: {} }}",
                action,
                pos,
                modifiers,
                time.elapsed().as_secs()
            ),
            Self::Screen {
                width,
                height,
                time,
            } => write!(
                f,
                "Screen {{ width: {}, height: {}, time: {} }}",
                width,
                height,
                time.elapsed().as_secs()
            ),
            Self::Custom(_) => write!(f, "Custom(..)"),
        }
    }
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

    /// Create a custom message wrapping any `Send + Sync + 'static` value.
    ///
    /// This is the Rust equivalent of Go gruid's `Msg = interface{}`.
    /// Use it for application-specific messages: timers, animation ticks,
    /// async results, inter-widget communication, etc.
    ///
    /// ```ignore
    /// struct MyTick(u32);
    /// let msg = Msg::custom(MyTick(42));
    /// ```
    pub fn custom<T: Any + Send + Sync>(value: T) -> Self {
        Self::Custom(Arc::new(value))
    }

    /// Try to downcast a `Custom` payload to a concrete type.
    ///
    /// Returns `None` if this is not a `Custom` variant or the type
    /// doesn't match.
    ///
    /// ```ignore
    /// if let Some(tick) = msg.downcast_ref::<MyTick>() {
    ///     println!("tick {}", tick.0);
    /// }
    /// ```
    pub fn downcast_ref<T: Any>(&self) -> Option<&T> {
        match self {
            Self::Custom(arc) => arc.downcast_ref::<T>(),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modmask_display_none() {
        assert_eq!(ModMask::NONE.to_string(), "None");
    }

    #[test]
    fn modmask_display_single() {
        assert_eq!(ModMask::SHIFT.to_string(), "Shift");
        assert_eq!(ModMask::CTRL.to_string(), "Ctrl");
        assert_eq!(ModMask::ALT.to_string(), "Alt");
        assert_eq!(ModMask::META.to_string(), "Meta");
    }

    #[test]
    fn modmask_display_combo() {
        assert_eq!((ModMask::CTRL | ModMask::SHIFT).to_string(), "Ctrl+Shift");
        assert_eq!(
            (ModMask::CTRL | ModMask::ALT | ModMask::SHIFT).to_string(),
            "Ctrl+Alt+Shift"
        );
        assert_eq!(
            (ModMask::CTRL | ModMask::ALT | ModMask::META | ModMask::SHIFT).to_string(),
            "Ctrl+Alt+Meta+Shift"
        );
    }
}
