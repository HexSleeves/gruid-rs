//! Color palette matching Go shamogu's ANSI terminal look.
//!
//! The original uses the terminal's 16-color ANSI palette. We approximate
//! with RGB values chosen to look similar on a dark terminal background.

use gruid_core::style::Color;

// -- Backgrounds --

/// Default terminal background (reset).
pub const BG: Color = Color::DEFAULT;
/// Lit FOV background — a visible-but-subtle dark shade.
pub const BG_LIT: Color = Color::from_rgb(40, 42, 54);

// -- Foregrounds --

/// Default terminal foreground (reset).
pub const FG: Color = Color::DEFAULT;
/// Dimmed foreground for out-of-FOV explored terrain.
pub const FG_DIM: Color = Color::from_rgb(98, 100, 106);
/// Bright white for emphasis / player.
pub const FG_EMPH: Color = Color::from_rgb(248, 248, 242);

// -- Terrain-specific foregrounds (in FOV) --

/// Wall '#' in FOV — light blue-grey, bold.
pub const WALL_FG: Color = Color::from_rgb(150, 155, 170);
/// Floor '.' in FOV — medium grey.
pub const FLOOR_FG: Color = Color::from_rgb(110, 115, 125);
/// Foliage '"' in FOV — green.
pub const FOLIAGE_FG: Color = Color::from_rgb(80, 160, 80);
/// Rubble '^' in FOV — brownish-yellow.
pub const RUBBLE_FG: Color = Color::from_rgb(170, 140, 80);
/// Translucent wall '◊' in FOV — cyan-ish.
pub const TLWALL_FG: Color = Color::from_rgb(100, 160, 180);

// -- Named palette colours --

pub const RED: Color = Color::from_rgb(255, 85, 85);
pub const GREEN: Color = Color::from_rgb(80, 200, 80);
pub const YELLOW: Color = Color::from_rgb(220, 200, 60);
pub const BLUE: Color = Color::from_rgb(100, 130, 255);
pub const MAGENTA: Color = Color::from_rgb(210, 100, 210);
pub const CYAN: Color = Color::from_rgb(80, 210, 210);
pub const ORANGE: Color = Color::from_rgb(220, 140, 50);
pub const VIOLET: Color = Color::from_rgb(140, 120, 255);

// -- Status bar colours --

pub const HP_GOOD: Color = GREEN;
pub const HP_WARN: Color = YELLOW;
pub const HP_CRIT: Color = ORANGE;
pub const STAT_BLUE: Color = BLUE;

/// Player '@' colour.
pub const PLAYER_FG: Color = Color::from_rgb(100, 160, 255);

/// Get colour for a monster entity based on its display character.
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
