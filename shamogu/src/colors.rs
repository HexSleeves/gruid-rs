//! Color palette — RGB approximations of ANSI colors matching Go shamogu.
//!
//! The Go game uses terminal ANSI colors. We use RGB approximations
//! since gruid-rs Color is RGB-based.

use gruid_core::style::Color;

/// Terminal default (reset).
pub const BG: Color = Color::DEFAULT;
/// Dark background for lit FOV areas.
pub const BG_SECONDARY: Color = Color::from_rgb(30, 30, 30);
/// Terminal default foreground.
pub const FG: Color = Color::DEFAULT;
/// Dimmed foreground for out-of-FOV.
pub const FG_SECONDARY: Color = Color::from_rgb(128, 128, 128);
/// Bright white emphasis.
pub const FG_EMPH: Color = Color::from_rgb(255, 255, 255);
/// Red.
pub const RED: Color = Color::from_rgb(255, 85, 85);
/// Green.
pub const GREEN: Color = Color::from_rgb(0, 170, 0);
/// Yellow.
pub const YELLOW: Color = Color::from_rgb(170, 170, 0);
/// Blue.
pub const BLUE: Color = Color::from_rgb(85, 85, 255);
/// Magenta.
pub const MAGENTA: Color = Color::from_rgb(170, 0, 170);
/// Cyan.
pub const CYAN: Color = Color::from_rgb(0, 170, 170);
/// Orange (red-ish).
pub const ORANGE: Color = Color::from_rgb(170, 85, 0);
/// Violet (bright blue).
pub const VIOLET: Color = Color::from_rgb(85, 85, 255);

/// Get color for a monster entity based on its display character.
pub fn monster_color(ch: char) -> Color {
    match ch {
        'r' | 'h' | 'B' | 'H' => RED,
        's' | 'p' | 'n' | 'c' => YELLOW,
        'e' | 'L' | 'C' | 'M' => MAGENTA,
        'a' | 'v' | 'T' | 'O' => GREEN,
        'l' | 'P' | 'G' => ORANGE,
        'F' | 'D' | 'K' | 'I' => BLUE,
        'b' | 'w' | 'W' | 'f' => CYAN,
        _ => FG,
    }
}
