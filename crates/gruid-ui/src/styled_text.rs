use std::collections::HashMap;

use gruid_core::{Cell, Grid, Point, Range, Style};

/// Text with optional per-character style markups.
///
/// Markup characters in the text (e.g. `@`) switch the style for the
/// following characters according to a markup map.
#[derive(Debug, Clone)]
pub struct StyledText {
    text: String,
    style: Style,
    markups: Option<HashMap<char, Style>>,
}

impl StyledText {
    // -- Constructors --

    /// Create a styled text from a plain string with default style.
    pub fn text(s: &str) -> Self {
        Self {
            text: s.to_string(),
            style: Style::default(),
            markups: None,
        }
    }

    /// Create a styled text from a formatted string with default style.
    pub fn textf(s: String) -> Self {
        Self {
            text: s,
            style: Style::default(),
            markups: None,
        }
    }

    /// Create a styled text with the given text and style.
    pub fn new(text: &str, style: Style) -> Self {
        Self {
            text: text.to_string(),
            style,
            markups: None,
        }
    }

    // -- Builders --

    /// Set the text content.
    pub fn with_text(mut self, text: &str) -> Self {
        self.text = text.to_string();
        self
    }

    /// Set the base style.
    pub fn with_style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    /// Add a single markup: when `marker` is encountered in the text, switch
    /// to `style` for subsequent characters.
    pub fn with_markup(mut self, marker: char, style: Style) -> Self {
        self.markups
            .get_or_insert_with(HashMap::new)
            .insert(marker, style);
        self
    }

    /// Set all markups at once.
    pub fn with_markups(mut self, markups: HashMap<char, Style>) -> Self {
        self.markups = if markups.is_empty() {
            None
        } else {
            Some(markups)
        };
        self
    }

    // -- Accessors --

    /// Return the raw text content.
    pub fn content(&self) -> &str {
        &self.text
    }

    /// Return the base style.
    pub fn style(&self) -> Style {
        self.style
    }

    /// Return a clone of the markups map (empty map if none set).
    pub fn markups(&self) -> HashMap<char, Style> {
        self.markups.clone().unwrap_or_default()
    }

    // -- Iteration & measurement --

    /// Iterate over styled characters, calling `callback` for each with its
    /// grid position and cell.  Returns the point one past the last character
    /// (i.e. the cursor position after the text).
    pub fn iter(&self, mut callback: impl FnMut(Point, Cell)) -> Point {
        let mut x: i32 = 0;
        let mut y: i32 = 0;
        let mut current_style = self.style;

        for ch in self.text.chars() {
            // Check if this character is a markup marker.
            if let Some(markups) = &self.markups {
                if let Some(&s) = markups.get(&ch) {
                    current_style = s;
                    continue;
                }
            }
            if ch == '\n' {
                x = 0;
                y += 1;
                continue;
            }
            let p = Point::new(x, y);
            let cell = Cell::default().with_char(ch).with_style(current_style);
            callback(p, cell);
            x += 1;
        }
        Point::new(x, y)
    }

    /// Return the minimum bounding size required to display this text.
    pub fn size(&self) -> Point {
        let mut max_x: i32 = 0;
        let mut max_y: i32 = 0;
        self.iter(|p, _| {
            if p.x + 1 > max_x {
                max_x = p.x + 1;
            }
            if p.y + 1 > max_y {
                max_y = p.y + 1;
            }
        });
        Point::new(max_x, max_y)
    }

    /// Word-wrap the text to the given width and return a new `StyledText`.
    pub fn format(&self, width: usize) -> StyledText {
        if width == 0 {
            return self.clone();
        }
        let mut result = String::new();
        for line in self.text.split('\n') {
            if !result.is_empty() {
                result.push('\n');
            }
            let mut col = 0usize;
            let mut first_word = true;
            for word in line.split(' ') {
                let word_len = visible_len(word, &self.markups);
                if !first_word && col + 1 + word_len > width {
                    result.push('\n');
                    col = 0;
                    first_word = true;
                }
                if !first_word {
                    result.push(' ');
                    col += 1;
                }
                result.push_str(word);
                col += word_len;
                first_word = false;
            }
        }
        StyledText {
            text: result,
            style: self.style,
            markups: self.markups.clone(),
        }
    }

    /// Split the text at newlines, returning a `Vec` of single-line
    /// `StyledText`s.
    pub fn lines(&self) -> Vec<StyledText> {
        self.text
            .split('\n')
            .map(|line| StyledText {
                text: line.to_string(),
                style: self.style,
                markups: self.markups.clone(),
            })
            .collect()
    }

    /// Draw the styled text into the given grid starting at (0,0).
    /// Returns the range of cells that were written (relative coords).
    pub fn draw(&self, grid: &Grid) -> Range {
        let mut min_x = i32::MAX;
        let mut min_y = i32::MAX;
        let mut max_x = i32::MIN;
        let mut max_y = i32::MIN;
        let mut any = false;
        self.iter(|p, cell| {
            if grid.contains(p) {
                grid.set(p, cell);
                if !any || p.x < min_x {
                    min_x = p.x;
                }
                if !any || p.y < min_y {
                    min_y = p.y;
                }
                if p.x + 1 > max_x {
                    max_x = p.x + 1;
                }
                if p.y + 1 > max_y {
                    max_y = p.y + 1;
                }
                any = true;
            }
        });
        if any {
            Range::new(min_x, min_y, max_x, max_y)
        } else {
            Range::new(0, 0, 0, 0)
        }
    }
}

/// Count visible (non-markup) characters in a string.
fn visible_len(s: &str, markups: &Option<HashMap<char, Style>>) -> usize {
    let mut count = 0;
    for ch in s.chars() {
        if let Some(m) = markups {
            if m.contains_key(&ch) {
                continue;
            }
        }
        count += 1;
    }
    count
}
