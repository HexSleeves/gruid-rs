use std::collections::HashMap;

use gruid_core::{Cell, Grid, Point, Range, Style};

/// Text with optional `@`-prefix style markups.
///
/// When at least one markup is registered (via [`with_markup`] or
/// [`with_markups`]), the following protocol applies to the text:
///
/// | Sequence | Effect |
/// |----------|--------|
/// | `@X`     | If `X` is a key in the markups map, switch to that style (zero-width). |
/// | `@N`     | Reset to the base style (zero-width). |
/// | `@@`     | Emit a literal `@` character. |
/// | `@?`     | For any other char `?`, the `@` is consumed and `?` is emitted in the current style. |
/// | `@` at end | Ignored. |
///
/// This markup protocol is compatible with Go gruid's `StyledText`.
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

    /// Return a derived styled text with updated text content.
    pub fn with_text(mut self, text: &str) -> Self {
        self.text = text.to_string();
        self
    }

    /// Return a derived styled text with a pre-formatted string.
    ///
    /// This is the Rust equivalent of Go gruid's `WithTextf`. Since Rust
    /// uses `format!()` instead of `fmt.Sprintf`, callers should pass the
    /// already-formatted string:
    ///
    /// ```ignore
    /// let stt = stt.with_textf(format!("HP: {}/{}", cur, max));
    /// ```
    pub fn with_textf(mut self, text: String) -> Self {
        self.text = text;
        self
    }

    /// Return a derived styled text with new text and style.
    pub fn with(mut self, text: &str, style: Style) -> Self {
        self.text = text.to_string();
        self.style = style;
        self
    }

    /// Set the base style.
    pub fn with_style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    /// Register a markup: `@marker` in the text will switch to `style`.
    ///
    /// Whitespace and newline markers are ignored to avoid conflicts with
    /// formatting. The special marker `'N'` sets the base style instead of
    /// adding to the markups map (matching Go gruid behaviour).
    pub fn with_markup(mut self, marker: char, style: Style) -> Self {
        if marker == ' ' || marker == '\n' || marker == '\r' {
            return self;
        }
        if marker == 'N' {
            // 'N' has built-in meaning: reset to base style.
            self.style = style;
            // Still ensure markups is Some so markup processing activates.
            if self.markups.is_none() {
                self.markups = Some(HashMap::new());
            }
            return self;
        }
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

    // -- Markup resolution helper --

    /// Resolve a markup rune to its style.
    fn markup_style(&self, r: char) -> Style {
        if r == 'N' {
            return self.style;
        }
        if let Some(ref m) = self.markups {
            if let Some(&s) = m.get(&r) {
                return s;
            }
        }
        self.style
    }

    // -- Iteration & measurement --

    /// Iterate over styled characters, calling `callback` for each visible
    /// character with its grid position and cell.
    ///
    /// Returns the minimum `(w, h)` size as a [`Point`] that can fit the text.
    ///
    /// The `@`-prefix markup protocol is applied when markups are registered:
    /// - `@X` where X is a markup key → switch style (zero-width)
    /// - `@N` → reset to base style (zero-width)
    /// - `@@` → emit literal `@`
    /// - `@?` → consume `@`, emit `?` in current style
    /// - `@` at end of string → ignored
    pub fn iter(&self, mut callback: impl FnMut(Point, Cell)) -> Point {
        let mut x: i32 = 0;
        let mut y: i32 = 0;
        let mut xmax: i32 = 0;
        let mut current_style = self.style;
        let markup = self.markups.is_some();
        let mut procm = false; // true when previous char was '@' and we're expecting the command char

        for ch in self.text.chars() {
            if ch == '\r' {
                continue;
            }
            if markup {
                if proc_markup(procm, ch) {
                    if procm {
                        // ch is the command char after '@': switch style.
                        // For known markups → that style; for 'N' → base;
                        // for unknown → base style (same as Go).
                        current_style = self.markup_style(ch);
                    }
                    procm = !procm;
                    continue;
                }
                // procm was true and ch == '@' → fall through to emit literal '@'
                procm = false;
            }
            if ch == '\n' {
                if x > xmax {
                    xmax = x;
                }
                x = 0;
                y += 1;
                continue;
            }
            let cell = Cell::default().with_char(ch).with_style(current_style);
            callback(Point::new(x, y), cell);
            x += 1;
        }
        if x > xmax {
            xmax = x;
        }
        if xmax > 0 || y > 0 {
            y += 1; // at least one line
        }
        Point::new(xmax, y)
    }

    /// Return the minimum bounding `(w, h)` size required to display this text.
    pub fn size(&self) -> Point {
        let mut x: i32 = 0;
        let mut y: i32 = 0;
        let mut xmax: i32 = 0;
        let markup = self.markups.is_some();
        let mut procm = false;

        for ch in self.text.chars() {
            if ch == '\r' {
                continue;
            }
            if markup {
                if proc_markup(procm, ch) {
                    procm = !procm;
                    continue;
                }
                procm = false;
            }
            if ch == '\n' {
                if x > xmax {
                    xmax = x;
                }
                x = 0;
                y += 1;
                continue;
            }
            x += 1;
        }
        if x > xmax {
            xmax = x;
        }
        if xmax > 0 || y > 0 {
            y += 1;
        }
        Point::new(xmax, y)
    }

    /// Word-wrap the text to the given width and return a new `StyledText`.
    ///
    /// Markup `@X` sequences are zero-width and do not count toward line
    /// length. `@@` counts as 1 character. Preserves leading spaces on a line.
    pub fn format(&self, width: usize) -> StyledText {
        if width == 0 {
            return self.clone();
        }
        let width = width as i32;
        let mut s = String::new();
        let mut wordbuf = String::new();
        let mut col: i32 = 0;
        let mut wantspace = false;
        let mut wlen: i32 = 0;
        let markup = self.markups.is_some();
        let mut procm = false;
        let mut start = true;

        let do_last_word =
            |s: &mut String, wordbuf: &str, wantspace: bool, wlen: i32, col: i32, width: i32| {
                if wantspace {
                    add_space(s, wlen + col + 1 > width);
                }
                s.push_str(wordbuf);
            };

        for ch in self.text.chars() {
            if ch == '\r' {
                continue;
            }
            if markup {
                if proc_markup(procm, ch) {
                    procm = !procm;
                    match ch {
                        '\n' | ' ' => {}
                        _ => {
                            if wlen == 0 {
                                s.push(ch);
                            } else {
                                wordbuf.push(ch);
                            }
                            continue;
                        }
                    }
                } else {
                    procm = false;
                }
            }
            if ch == ' ' {
                if start {
                    s.push(' ');
                    col += 1;
                    continue;
                }
                if wlen > 0 {
                    let newline = wlen + col + 1 > width;
                    if wantspace {
                        add_space(&mut s, newline);
                        if newline {
                            col = 0;
                        } else {
                            col += 1;
                        }
                    }
                    s.push_str(&wordbuf);
                    col += wlen;
                    wordbuf.clear();
                    wlen = 0;
                    wantspace = true;
                }
                continue;
            }
            if ch == '\n' {
                if wlen > 0 {
                    do_last_word(&mut s, &wordbuf, wantspace, wlen, col, width);
                    wordbuf.clear();
                    wlen = 0;
                }
                s.push('\n');
                col = 0;
                wantspace = false;
                start = true;
                continue;
            }
            start = false;
            wordbuf.push(ch);
            wlen += 1;
        }
        if wlen > 0 {
            do_last_word(&mut s, &wordbuf, wantspace, wlen, col, width);
        }
        // Trim trailing spaces and newlines
        let trimmed = s.trim_end_matches([' ', '\n']);
        StyledText {
            text: trimmed.to_string(),
            style: self.style,
            markups: self.markups.clone(),
        }
    }

    /// Split the text into lines, preserving markup state across line
    /// boundaries.
    ///
    /// When markups are active, continuation lines are prefixed with the
    /// active `@X` sequence so that each line can be rendered independently
    /// with the correct initial style.
    pub fn lines(&self) -> Vec<StyledText> {
        if self.markups.is_none() {
            return self
                .text
                .split('\n')
                .map(|line| StyledText {
                    text: line.replace('\r', ""),
                    style: self.style,
                    markups: self.markups.clone(),
                })
                .collect();
        }

        let mut markup_rune_start: char = 'N'; // markup rune at line start
        let mut markup_rune: char = 'N'; // current markup rune
        let mut procm = false;
        let mut stts = Vec::new();
        let mut from = 0usize;

        for (i, r) in self.text.char_indices() {
            if r == '\n' {
                // do_newline inline
                procm = false;
                let mut line = self.text[from..i].replace('\r', "");
                if markup_rune_start != 'N' {
                    line = format!("@{}{}", markup_rune_start, line);
                }
                markup_rune_start = markup_rune;
                stts.push(StyledText {
                    text: line,
                    style: self.style,
                    markups: self.markups.clone(),
                });
                from = i + 1;
                continue;
            }
            if procm {
                procm = false;
                if r != '@' && r != ' ' && r != '\r' {
                    markup_rune = r;
                }
            } else if r == '@' {
                procm = true;
            }
        }
        // Handle remaining text after the last newline
        if from != self.text.len() {
            let mut line = self.text[from..].replace('\r', "");
            if markup_rune_start != 'N' {
                line = format!("@{}{}", markup_rune_start, line);
            }
            stts.push(StyledText {
                text: line,
                style: self.style,
                markups: self.markups.clone(),
            });
        }

        stts
    }

    /// Draw the styled text into the given grid starting at `(0, 0)`.
    /// Returns the range of cells that were written.
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

/// Determines whether the current character is part of markup processing.
///
/// When `procm` is true (we just saw `@`), returns true for any char that is
/// NOT `@` (those are markup commands like `@N`, `@X`). Returns false for `@`
/// so that `@@` falls through to emit a literal `@`.
///
/// When `procm` is false, returns true only for `@` (to start markup processing).
fn proc_markup(procm: bool, r: char) -> bool {
    if procm { r != '@' } else { r == '@' }
}

/// Append a newline or a space to the string builder.
fn add_space(s: &mut String, newline: bool) {
    if newline {
        s.push('\n');
    } else {
        s.push(' ');
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gruid_core::{Color, Style};

    // -----------------------------------------------------------------------
    // Ported from Go styledtext_test.go
    // -----------------------------------------------------------------------

    #[test]
    fn test_size_empty() {
        let stt = StyledText::text("");
        let max = stt.size();
        assert_eq!(max, Point::new(0, 0));
    }

    #[test]
    fn test_size_word() {
        let stt = StyledText::text("word");
        let max = stt.size();
        assert_eq!(max, Point::new(4, 1));
    }

    #[test]
    fn test_size_two_lines() {
        let stt = StyledText::text("word\nword");
        let max = stt.size();
        assert_eq!(max, Point::new(4, 2));
    }

    #[test]
    fn test_format_9() {
        // "word word word word word" formatted to width 9
        let text = "word word word word word";
        let stt = StyledText::text(text);
        let max = stt.size();
        assert_eq!(max.x, 4 * 5 + 4);
        assert_eq!(max.y, 1);

        let stt = stt.format(9);
        let max = stt.size();
        let newlines = stt.content().matches('\n').count();
        assert_eq!(newlines, 2, "text: {}", stt.content());
        let spaces = stt.content().matches(' ').count();
        assert_eq!(spaces, 2, "text: {}", stt.content());
        assert_eq!(max, Point::new(9, 3), "text: {}", stt.content());
    }

    #[test]
    fn test_format_8() {
        let text = "word word word word word\r";
        let stt = StyledText::text(text).format(8);
        let max = stt.size();
        let newlines = stt.content().matches('\n').count();
        assert_eq!(newlines, 4, "text: {}", stt.content());
        assert_eq!(max, Point::new(4, 5), "text: {}", stt.content());
    }

    #[test]
    fn test_format_10() {
        let text = "word word word word word";
        let stt = StyledText::text(text).format(10);
        let max = stt.size();
        let newlines = stt.content().matches('\n').count();
        assert_eq!(newlines, 2, "text: {}", stt.content());
        assert_eq!(max, Point::new(9, 3), "text: {}", stt.content());
    }

    #[test]
    fn test_format_1() {
        let text = "word word word word word";
        let stt = StyledText::text(text).format(1);
        let max = stt.size();
        let newlines = stt.content().matches('\n').count();
        assert_eq!(newlines, 4, "text: {}", stt.content());
        assert_eq!(max, Point::new(4, 5), "text: {}", stt.content());
    }

    #[test]
    fn test_format_20() {
        let text = "word word word word word";
        let stt = StyledText::text(text).format(20);
        let newlines = stt.content().matches('\n').count();
        assert_eq!(newlines, 1, "text: {}", stt.content());
        let spaces = stt.content().matches(' ').count();
        assert_eq!(spaces, 3, "text: {}", stt.content());
    }

    #[test]
    fn test_format_idempotent() {
        let text = "word word word word word";
        let stt = StyledText::text(text).format(10);
        let stt2 = stt.format(10);
        assert_eq!(stt.content(), stt2.content());
    }

    #[test]
    fn test_format_with_markup() {
        let text = "word @cword@N word word word";
        let st = Style::default();
        let stt = StyledText::text(text).with_markup('c', st);
        let max = stt.size();
        assert_eq!(max.x, 4 * 5 + 4, "pre-format size");
        assert_eq!(max.y, 1);

        let stt = stt.format(9);
        let max = stt.size();
        let newlines = stt.content().matches('\n').count();
        assert_eq!(newlines, 2, "text: {}", stt.content());
        assert_eq!(max, Point::new(9, 3), "text: {}", stt.content());
    }

    #[test]
    fn test_lines_no_markup() {
        let text = "a b c\nd e f g";
        let stts = StyledText::text(text).lines();
        assert_eq!(stts.len(), 2);
        assert_eq!(stts[0].content(), "a b c");
        assert_eq!(stts[1].content(), "d e f g");
    }

    #[test]
    fn test_lines_with_markup() {
        let text = "a b @Bc\nd e@N f\nmore stuff";
        let st = Style::default();
        let stts = StyledText::text(text).with_markup('B', st).lines();
        assert_eq!(stts.len(), 3);
        assert_eq!(stts[0].content(), "a b @Bc");
        assert_eq!(stts[1].content(), "@Bd e@N f");
        assert_eq!(stts[2].content(), "more stuff");
    }

    #[test]
    fn test_size_markup() {
        let st = Style::default();
        let stt = StyledText::text("@t\u{2022}@N ").with_markup('t', st);
        assert_eq!(stt.size(), Point::new(2, 1));
        let mut count = 0;
        stt.iter(|_, _| count += 1);
        assert_eq!(count, 2);
        let stt = stt.format(10);
        assert_eq!(stt.size(), Point::new(1, 1));
    }

    #[test]
    fn test_markup_consecutive() {
        // "@N@@@t•@N " → @N resets, @@ emits '@', @t switches, '•', @N resets, ' '
        // visible: '@', '•', ' ' → 3 chars
        let st = Style::default();
        let stt = StyledText::text("@N@@@t\u{2022}@N ").with_markup('t', st);
        assert_eq!(stt.size(), Point::new(3, 1));
        let mut count = 0;
        stt.iter(|_, _| count += 1);
        assert_eq!(count, 3);
    }

    // -----------------------------------------------------------------------
    // Additional tests for the @ protocol
    // -----------------------------------------------------------------------

    #[test]
    fn test_at_escape_produces_literal_at() {
        let st = Style::default().with_fg(Color::from_rgb(255, 0, 0));
        let stt = StyledText::text("hi@@you").with_markup('x', st);
        let mut chars = Vec::new();
        stt.iter(|p, c| chars.push((p, c.ch)));
        let text: String = chars.iter().map(|(_, ch)| ch).collect();
        assert_eq!(text, "hi@you");
    }

    #[test]
    fn test_at_reset() {
        let red = Style::default().with_fg(Color::from_rgb(255, 0, 0));
        let base = Style::default().with_fg(Color::from_rgb(0, 255, 0));
        let stt = StyledText::new("@Ra@Nb", base).with_markup('R', red);
        let mut cells = Vec::new();
        stt.iter(|_, c| cells.push(c));
        assert_eq!(cells.len(), 2);
        assert_eq!(cells[0].ch, 'a');
        assert_eq!(cells[0].style, red);
        assert_eq!(cells[1].ch, 'b');
        assert_eq!(cells[1].style, base);
    }

    #[test]
    fn test_at_unrecognized_char() {
        // @z where z is not a registered markup key → z is consumed (not emitted),
        // style resets to default (same as Go: markupStyle returns stt.style for
        // unknown runes).
        let stt = StyledText::text("a@zb").with_markup('x', Style::default());
        let mut chars = Vec::new();
        stt.iter(|_, c| chars.push(c.ch));
        assert_eq!(chars, vec!['a', 'b']);
    }

    #[test]
    fn test_at_end_of_string() {
        let stt = StyledText::text("hello@").with_markup('x', Style::default());
        let mut chars = Vec::new();
        stt.iter(|_, c| chars.push(c.ch));
        assert_eq!(chars, vec!['h', 'e', 'l', 'l', 'o']);
    }

    #[test]
    fn test_no_markup_mode() {
        // When no markups are registered, '@' is treated as a normal char.
        let stt = StyledText::text("a@b");
        let mut chars = Vec::new();
        stt.iter(|_, c| chars.push(c.ch));
        assert_eq!(chars, vec!['a', '@', 'b']);
    }

    #[test]
    fn test_nested_markup_switches() {
        let red = Style::default().with_fg(Color::from_rgb(255, 0, 0));
        let blue = Style::default().with_fg(Color::from_rgb(0, 0, 255));
        let base = Style::default();
        let stt = StyledText::new("@Ra@Bb@Nc", base)
            .with_markup('R', red)
            .with_markup('B', blue);
        let mut cells = Vec::new();
        stt.iter(|_, c| cells.push(c));
        assert_eq!(cells.len(), 3);
        assert_eq!(cells[0].style, red);
        assert_eq!(cells[1].style, blue);
        assert_eq!(cells[2].style, base);
    }

    #[test]
    fn test_newline_positions() {
        let stt = StyledText::text("ab\ncd");
        let mut positions = Vec::new();
        stt.iter(|p, c| positions.push((p, c.ch)));
        assert_eq!(positions.len(), 4);
        assert_eq!(positions[0], (Point::new(0, 0), 'a'));
        assert_eq!(positions[1], (Point::new(1, 0), 'b'));
        assert_eq!(positions[2], (Point::new(0, 1), 'c'));
        assert_eq!(positions[3], (Point::new(1, 1), 'd'));
    }

    #[test]
    fn test_size_returns_correct_bounding() {
        let stt = StyledText::text("ab\ncde");
        assert_eq!(stt.size(), Point::new(3, 2));
    }

    #[test]
    fn test_carriage_return_stripped() {
        let stt = StyledText::text("a\r\nb");
        let mut chars = Vec::new();
        stt.iter(|_, c| chars.push(c.ch));
        assert_eq!(chars, vec!['a', 'b']);
        assert_eq!(stt.size(), Point::new(1, 2));
    }

    #[test]
    fn test_lines_preserves_markup_state_across_lines() {
        // If markup 'B' is active at end of line 1, line 2 should start with @B prefix
        let st = Style::default().with_fg(Color::from_rgb(0, 0, 255));
        let stt = StyledText::text("@Bhello\nworld").with_markup('B', st);
        let lines = stt.lines();
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].content(), "@Bhello");
        assert_eq!(lines[1].content(), "@Bworld");
    }

    #[test]
    fn test_lines_reset_across_lines() {
        let st = Style::default();
        let stt = StyledText::text("@Bhi@N\nthere").with_markup('B', st);
        let lines = stt.lines();
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].content(), "@Bhi@N");
        // After @N reset, markup_rune is 'N', so no prefix
        assert_eq!(lines[1].content(), "there");
    }

    #[test]
    fn test_format_with_at_escape() {
        // @@ takes 1 visible char width
        let stt = StyledText::text("@@").with_markup('x', Style::default());
        assert_eq!(stt.size(), Point::new(1, 1));
    }

    #[test]
    fn test_with_textf() {
        let stt = StyledText::text("old").with_textf(format!("HP: {}/{}", 10, 20));
        assert_eq!(stt.content(), "HP: 10/20");
    }

    #[test]
    fn test_with_text_and_style() {
        let st = Style::default().with_fg(Color::from_rgb(255, 0, 0));
        let stt = StyledText::text("old").with("new", st);
        assert_eq!(stt.content(), "new");
        assert_eq!(stt.style(), st);
    }

    #[test]
    fn test_with_markup_n_sets_base_style() {
        let custom = Style::default().with_fg(Color::from_rgb(42, 42, 42));
        let stt = StyledText::text("hello").with_markup('N', custom);
        assert_eq!(stt.style(), custom);
    }
}
