//! Prefabricated room/level sections built from text.
//!
//! A [`Vault`] parses an ASCII art string into a grid overlay that can be
//! drawn, reflected, and rotated. This matches Go gruid's `rl.Vault`.

use crate::grid::{Cell, Grid};
use gruid_core::Point;
use std::fmt;

/// A prefabricated room or level section built from text.
///
/// Each character in the content maps to a position. Lines are separated
/// by `'\n'` and must all have the same width.
#[derive(Debug, Clone)]
pub struct Vault {
    content: String,
    runes: String,
    size: Point,
}

impl Vault {
    /// Create a new vault by parsing the given string.
    ///
    /// See [`parse`](Self::parse) for format requirements.
    pub fn new(s: &str) -> Result<Self, VaultError> {
        let mut v = Self {
            content: String::new(),
            runes: String::new(),
            size: Point::new(0, 0),
        };
        v.parse(s)?;
        Ok(v)
    }

    /// Return the vault's textual content.
    pub fn content(&self) -> &str {
        &self.content
    }

    /// Return the (width, height) size of the vault in cells.
    pub fn size(&self) -> Point {
        self.size
    }

    /// Set the permitted characters. If non-empty, [`parse`](Self::parse)
    /// will reject any character not in this string. Empty means any
    /// character is allowed.
    pub fn set_runes(&mut self, s: &str) {
        self.runes = s.to_string();
    }

    /// Return the currently permitted characters.
    pub fn runes(&self) -> &str {
        &self.runes
    }

    /// Parse (or re-parse) the vault content.
    ///
    /// Each line must have the same width. Leading/trailing whitespace
    /// is trimmed from the whole string but not from individual lines.
    /// Only characters in [`runes`](Self::runes) are allowed (if set).
    pub fn parse(&mut self, s: &str) -> Result<(), VaultError> {
        let s = s.trim();
        let mut x: i32 = 0;
        let mut y: i32 = 0;
        let mut w: i32 = -1;

        for ch in s.chars() {
            if ch == '\n' {
                if x > w {
                    if w > 0 {
                        return Err(VaultError::InconsistentSize(s.to_string()));
                    }
                    w = x;
                }
                x = 0;
                y += 1;
                continue;
            }
            if !self.runes.is_empty() && !self.runes.contains(ch) {
                return Err(VaultError::InvalidRune {
                    ch,
                    pos: Point::new(x, y),
                    content: s.to_string(),
                });
            }
            x += 1;
        }
        if x > w {
            if w > 0 {
                return Err(VaultError::InconsistentSize(s.to_string()));
            }
            w = x;
        }
        if w > 0 || y > 0 {
            y += 1; // at least one line
        }
        self.content = s.to_string();
        self.size = Point::new(x, y);
        Ok(())
    }

    /// Iterate over all positions and their characters.
    pub fn iter(&self, mut f: impl FnMut(Point, char)) {
        let mut x: i32 = 0;
        let mut y: i32 = 0;
        for ch in self.content.chars() {
            if ch == '\n' {
                x = 0;
                y += 1;
                continue;
            }
            f(Point::new(x, y), ch);
            x += 1;
        }
    }

    /// Draw the vault into a grid using a mapping from characters to cells.
    ///
    /// Returns a sub-grid slice covering the vault's extent.
    pub fn draw(&self, grid: &Grid, f: impl Fn(char) -> Cell) -> Grid {
        let mut x: i32 = 0;
        let mut y: i32 = 0;
        for ch in self.content.chars() {
            if ch == '\n' {
                x = 0;
                y += 1;
                continue;
            }
            grid.set(Point::new(x, y), f(ch));
            x += 1;
        }
        grid.slice(gruid_core::Range::new(0, 0, self.size.x, self.size.y))
    }

    /// Reflect the content horizontally (reverse each line).
    pub fn reflect(&mut self) {
        let mut result = String::with_capacity(self.content.len());
        let mut line: Vec<char> = Vec::with_capacity(self.size.x as usize);

        for ch in self.content.chars() {
            if ch == '\n' {
                for &c in line.iter().rev() {
                    result.push(c);
                }
                result.push('\n');
                line.clear();
                continue;
            }
            line.push(ch);
        }
        // Last line (no trailing newline)
        for &c in line.iter().rev() {
            result.push(c);
        }

        self.content = result;
    }

    /// Rotate the content n×90° counter-clockwise.
    /// Negative n rotates clockwise.
    pub fn rotate(&mut self, n: i32) {
        let mut n = n % 4;
        if n < 0 {
            n += 4;
        }
        match n {
            1 => self.rotate90(),
            2 => self.rotate180(),
            3 => {
                self.rotate180();
                self.rotate90();
            }
            _ => {}
        }
    }

    fn rotate90(&mut self) {
        let lines: Vec<Vec<char>> = self
            .content
            .split('\n')
            .map(|l| l.chars().collect())
            .collect();
        let max = self.size;
        let mut result = String::with_capacity(self.content.len());

        for x in 0..max.x {
            for y in 0..max.y {
                result.push(lines[y as usize][(max.x - x - 1) as usize]);
            }
            if x < max.x - 1 {
                result.push('\n');
            }
        }

        self.content = result;
        self.size = Point::new(max.y, max.x);
    }

    fn rotate180(&mut self) {
        let reversed: String = self.content.chars().rev().collect();
        self.content = reversed;
    }
}

/// Errors that can occur when parsing a vault.
#[derive(Debug, Clone)]
pub enum VaultError {
    /// Lines have inconsistent widths.
    InconsistentSize(String),
    /// A character not in the allowed set was found.
    InvalidRune {
        ch: char,
        pos: Point,
        content: String,
    },
}

impl fmt::Display for VaultError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InconsistentSize(s) => write!(f, "vault: inconsistent size:\n{s}"),
            Self::InvalidRune { ch, pos, content } => {
                write!(
                    f,
                    "vault contains invalid rune \u{201c}{ch}\u{201d} at ({}, {}):\n{content}",
                    pos.x, pos.y
                )
            }
        }
    }
}

impl std::error::Error for VaultError {}

#[cfg(test)]
mod tests {
    use super::*;

    const ROOM: &str = "\
####
#..#
#..#
####";

    #[test]
    fn parse_and_size() {
        let v = Vault::new(ROOM).unwrap();
        assert_eq!(v.size(), Point::new(4, 4));
    }

    #[test]
    fn iter_positions() {
        let v = Vault::new(ROOM).unwrap();
        let mut cells = Vec::new();
        v.iter(|p, ch| cells.push((p, ch)));
        assert_eq!(cells.len(), 16);
        assert_eq!(cells[0], (Point::new(0, 0), '#'));
        assert_eq!(cells[5], (Point::new(1, 1), '.'));
    }

    #[test]
    fn draw_into_grid() {
        let v = Vault::new(ROOM).unwrap();
        let grid = Grid::new(10, 10);
        let sub = v.draw(&grid, |ch| if ch == '#' { Cell(1) } else { Cell(0) });
        assert_eq!(sub.size(), Point::new(4, 4));
        assert_eq!(grid.at(Point::new(0, 0)), Some(Cell(1)));
        assert_eq!(grid.at(Point::new(1, 1)), Some(Cell(0)));
    }

    #[test]
    fn reflect() {
        let mut v = Vault::new("AB\nCD").unwrap();
        v.reflect();
        assert_eq!(v.content(), "BA\nDC");
        assert_eq!(v.size(), Point::new(2, 2));
    }

    #[test]
    fn rotate_90() {
        let mut v = Vault::new("AB\nCD").unwrap();
        v.rotate(1); // 90° CCW
        // Original:      Rotated 90° CCW:
        //   AB              BD
        //   CD              AC
        assert_eq!(v.content(), "BD\nAC");
        assert_eq!(v.size(), Point::new(2, 2));
    }

    #[test]
    fn rotate_180() {
        let mut v = Vault::new("AB\nCD").unwrap();
        v.rotate(2);
        assert_eq!(v.content(), "DC\nBA");
    }

    #[test]
    fn rotate_270() {
        let mut v = Vault::new("AB\nCD").unwrap();
        v.rotate(3); // 270° CCW = 90° CW
        assert_eq!(v.content(), "CA\nDB");
        assert_eq!(v.size(), Point::new(2, 2));
    }

    #[test]
    fn rotate_full_circle() {
        let mut v = Vault::new(ROOM).unwrap();
        let original = v.content().to_string();
        v.rotate(4);
        assert_eq!(v.content(), original);
    }

    #[test]
    fn set_runes_validation() {
        let mut v = Vault {
            content: String::new(),
            runes: String::new(),
            size: Point::new(0, 0),
        };
        v.set_runes("#.");
        assert!(v.parse(ROOM).is_ok());
        assert!(v.parse("AB").is_err());
    }

    #[test]
    fn inconsistent_size_error() {
        let result = Vault::new("AB\nCDE");
        assert!(result.is_err());
    }
}
