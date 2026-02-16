//! Single-line text input widget with cursor, prompt, and mouse support.

use gruid_core::messages::{Key, MouseAction, Msg};
use gruid_core::{Cell, Grid, Point, Style};

use crate::{BoxDecor, StyledText};

/// Configuration for a [`TextInput`] widget.
#[derive(Debug, Clone)]
pub struct TextInputConfig {
    /// Grid to draw into.
    pub grid: Grid,
    /// Initial content.
    pub content: String,
    /// Optional prompt text displayed before the input.
    pub prompt: Option<StyledText>,
    /// Key bindings.
    pub keys: TextInputKeys,
    /// Optional box decoration.
    pub box_: Option<BoxDecor>,
    /// Visual style.
    pub style: TextInputStyle,
}

/// Key bindings for text input.
#[derive(Debug, Clone)]
pub struct TextInputKeys {
    /// Keys that confirm/submit the input.
    pub confirm: Vec<Key>,
    /// Keys that cancel the input.
    pub cancel: Vec<Key>,
}

impl Default for TextInputKeys {
    fn default() -> Self {
        Self {
            confirm: vec![Key::Enter],
            cancel: vec![Key::Escape],
        }
    }
}

/// Visual style for text input.
#[derive(Debug, Clone, Default)]
pub struct TextInputStyle {
    /// Style for the text.
    pub text: Style,
    /// Style for the cursor.
    pub cursor: Style,
}

/// Actions returned by [`TextInput::update`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TextInputAction {
    /// No meaningful action.
    Pass,
    /// The text content changed.
    Change,
    /// The user confirmed the input.
    Confirm,
    /// The user cancelled the input.
    Cancel,
}

/// A single-line text input widget.
#[derive(Debug, Clone)]
pub struct TextInput {
    grid: Grid,
    content: String,
    cursor: usize,
    prompt: Option<StyledText>,
    keys: TextInputKeys,
    box_: Option<BoxDecor>,
    style: TextInputStyle,
    action: TextInputAction,
}

impl TextInput {
    /// Create a new text input from the given configuration.
    ///
    /// If no explicit cursor style is set (i.e. it equals `Style::default()`),
    /// the cursor style is automatically derived by swapping the foreground
    /// and background of the text style. This matches Go gruid behaviour.
    pub fn new(config: TextInputConfig) -> Self {
        let cursor = config.content.len();
        let style = {
            let mut s = config.style;
            if s.cursor == Style::default() {
                // Auto-reverse: swap fg/bg of text style for cursor.
                s.cursor = Style {
                    fg: s.text.bg,
                    bg: s.text.fg,
                    attrs: s.text.attrs,
                };
            }
            s
        };
        Self {
            grid: config.grid,
            content: config.content,
            cursor,
            prompt: config.prompt,
            keys: config.keys,
            box_: config.box_,
            style,
            action: TextInputAction::Pass,
        }
    }

    /// Process an input message and return the resulting action.
    pub fn update(&mut self, msg: Msg) -> TextInputAction {
        self.action = TextInputAction::Pass;

        match msg {
            Msg::KeyDown { ref key, .. } => {
                if self.keys.confirm.contains(key) {
                    self.action = TextInputAction::Confirm;
                } else if self.keys.cancel.contains(key) {
                    self.action = TextInputAction::Cancel;
                } else {
                    match key {
                        Key::Char(ch) => {
                            self.content.insert(self.cursor, *ch);
                            self.cursor += ch.len_utf8();
                            self.action = TextInputAction::Change;
                        }
                        Key::Backspace => {
                            if self.cursor > 0 {
                                let prev = self.content[..self.cursor]
                                    .char_indices()
                                    .next_back()
                                    .map(|(i, _)| i)
                                    .unwrap_or(0);
                                self.content.remove(prev);
                                self.cursor = prev;
                                self.action = TextInputAction::Change;
                            }
                        }
                        Key::Delete => {
                            if self.cursor < self.content.len() {
                                self.content.remove(self.cursor);
                                self.action = TextInputAction::Change;
                            }
                        }
                        Key::ArrowLeft => {
                            if self.cursor > 0 {
                                let prev = self.content[..self.cursor]
                                    .char_indices()
                                    .next_back()
                                    .map(|(i, _)| i)
                                    .unwrap_or(0);
                                self.cursor = prev;
                            }
                        }
                        Key::ArrowRight => {
                            if self.cursor < self.content.len() {
                                let next = self.content[self.cursor..]
                                    .char_indices()
                                    .nth(1)
                                    .map(|(i, _)| self.cursor + i)
                                    .unwrap_or(self.content.len());
                                self.cursor = next;
                            }
                        }
                        Key::Home => {
                            self.cursor = 0;
                        }
                        Key::End => {
                            self.cursor = self.content.len();
                        }
                        _ => {}
                    }
                }
            }
            Msg::Mouse {
                action: MouseAction::Main,
                pos,
                ..
            } => {
                let inner = self.inner_range();
                if pos.y == inner.min.y && pos.x >= inner.min.x && pos.x < inner.max.x {
                    let prompt_len = self.prompt_char_len();
                    let click_col = (pos.x - inner.min.x) as usize;
                    if click_col >= prompt_len {
                        let text_col = click_col - prompt_len + self.compute_scroll();
                        let chars: Vec<char> = self.content.chars().collect();
                        let target = text_col.min(chars.len());
                        // Convert char position to byte offset
                        self.cursor = chars[..target].iter().map(|c| c.len_utf8()).sum();
                    }
                }
            }
            _ => {}
        }

        self.action
    }

    /// Draw the text input into the given grid.
    pub fn draw(&self, grid: &Grid) {
        let inner_range = if let Some(ref box_decor) = self.box_ {
            box_decor.draw(grid)
        } else {
            grid.range_()
        };

        let start = inner_range.min;
        let vis_w = (inner_range.max.x - inner_range.min.x) as usize;
        let y = start.y;

        // Draw prompt first
        let prompt_len = self.prompt_char_len();
        if let Some(ref prompt) = self.prompt {
            let style = prompt.style();
            for (i, ch) in prompt.content().chars().enumerate() {
                if i >= vis_w {
                    break;
                }
                let p = Point::new(start.x + i as i32, y);
                if grid.contains(p) {
                    grid.set(p, Cell::default().with_char(ch).with_style(style));
                }
            }
        }

        let input_w = vis_w.saturating_sub(prompt_len);
        if input_w == 0 {
            return;
        }

        let scroll = self.compute_scroll();
        let cursor_char_pos = self.content[..self.cursor].chars().count();
        let chars: Vec<char> = self.content.chars().collect();

        for col in 0..input_w {
            let char_idx = scroll + col;
            let x = start.x + prompt_len as i32 + col as i32;
            let p = Point::new(x, y);
            if !grid.contains(p) {
                break;
            }

            let is_cursor = char_idx == cursor_char_pos;
            let style = if is_cursor {
                self.style.cursor
            } else {
                self.style.text
            };

            let ch = if char_idx < chars.len() {
                chars[char_idx]
            } else if is_cursor {
                '_'
            } else {
                ' '
            };

            grid.set(p, Cell::default().with_char(ch).with_style(style));
        }
    }

    /// Return the current content.
    pub fn content(&self) -> &str {
        &self.content
    }

    /// Set the content and move cursor to end.
    pub fn set_content(&mut self, s: &str) {
        self.content = s.to_string();
        self.cursor = self.content.len();
    }

    /// Return the last action.
    pub fn action(&self) -> TextInputAction {
        self.action
    }

    /// Set the cursor byte position.
    pub fn set_cursor(&mut self, pos: usize) {
        self.cursor = pos.min(self.content.len());
    }

    /// Replace the box decoration.
    pub fn set_box(&mut self, box_: Option<BoxDecor>) {
        self.box_ = box_;
    }

    /// Set the prompt.
    pub fn set_prompt(&mut self, prompt: Option<StyledText>) {
        self.prompt = prompt;
    }

    // -- private helpers --

    fn inner_range(&self) -> gruid_core::Range {
        if let Some(ref _box_decor) = self.box_ {
            let r = self.grid.range_();
            gruid_core::Range::new(r.min.x + 1, r.min.y + 1, r.max.x - 1, r.max.y - 1)
        } else {
            self.grid.range_()
        }
    }

    fn prompt_char_len(&self) -> usize {
        self.prompt
            .as_ref()
            .map_or(0, |p| p.content().chars().count())
    }

    fn compute_scroll(&self) -> usize {
        let inner = self.inner_range();
        let vis_w = (inner.max.x - inner.min.x) as usize;
        let input_w = vis_w.saturating_sub(self.prompt_char_len());
        let cursor_char_pos = self.content[..self.cursor].chars().count();
        if cursor_char_pos >= input_w {
            cursor_char_pos - input_w + 1
        } else {
            0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_input(content: &str) -> TextInput {
        TextInput::new(TextInputConfig {
            grid: Grid::new(20, 1),
            content: content.to_string(),
            prompt: None,
            keys: TextInputKeys::default(),
            box_: None,
            style: TextInputStyle::default(),
        })
    }

    #[test]
    fn type_characters() {
        let mut input = make_input("");
        input.update(Msg::key(Key::Char('h')));
        input.update(Msg::key(Key::Char('i')));
        assert_eq!(input.content(), "hi");
    }

    #[test]
    fn backspace_and_delete() {
        let mut input = make_input("abc");
        // Cursor at end
        input.update(Msg::key(Key::Backspace));
        assert_eq!(input.content(), "ab");

        // Move to start, delete
        input.update(Msg::key(Key::Home));
        input.update(Msg::key(Key::Delete));
        assert_eq!(input.content(), "b");
    }

    #[test]
    fn cursor_movement() {
        let mut input = make_input("hello");
        input.update(Msg::key(Key::Home));
        input.update(Msg::key(Key::Char('X')));
        assert_eq!(input.content(), "Xhello");

        input.update(Msg::key(Key::End));
        input.update(Msg::key(Key::Char('Y')));
        assert_eq!(input.content(), "XhelloY");
    }

    #[test]
    fn confirm_cancel() {
        let mut input = make_input("test");
        let action = input.update(Msg::key(Key::Enter));
        assert_eq!(action, TextInputAction::Confirm);

        let action = input.update(Msg::key(Key::Escape));
        assert_eq!(action, TextInputAction::Cancel);
    }

    #[test]
    fn prompt_support() {
        let mut input = TextInput::new(TextInputConfig {
            grid: Grid::new(30, 1),
            content: String::new(),
            prompt: Some(StyledText::new("> ", Style::default())),
            keys: TextInputKeys::default(),
            box_: None,
            style: TextInputStyle::default(),
        });
        assert_eq!(input.prompt_char_len(), 2);
        input.update(Msg::key(Key::Char('a')));
        assert_eq!(input.content(), "a");
    }

    #[test]
    fn mouse_click_positions_cursor() {
        let mut input = make_input("hello");
        // Click at column 2 (should position cursor at char index 2)
        input.update(Msg::Mouse {
            action: MouseAction::Main,
            pos: Point::new(2, 0),
            modifiers: Default::default(),
            time: std::time::Instant::now(),
        });
        input.update(Msg::key(Key::Char('X')));
        assert_eq!(input.content(), "heXllo");
    }

    #[test]
    fn cursor_auto_reverse() {
        let text_style = Style::default()
            .with_fg(gruid_core::Color::from_rgb(255, 255, 255))
            .with_bg(gruid_core::Color::from_rgb(0, 0, 0));
        let input = TextInput::new(TextInputConfig {
            grid: Grid::new(20, 1),
            content: String::new(),
            prompt: None,
            keys: TextInputKeys::default(),
            box_: None,
            style: TextInputStyle {
                text: text_style,
                cursor: Style::default(), // triggers auto-reverse
            },
        });
        // Cursor should have fg/bg swapped
        assert_eq!(input.style.cursor.fg, text_style.bg);
        assert_eq!(input.style.cursor.bg, text_style.fg);
    }

    #[test]
    fn cursor_explicit_no_reverse() {
        let cursor_style = Style::default().with_fg(gruid_core::Color::from_rgb(255, 0, 0));
        let input = TextInput::new(TextInputConfig {
            grid: Grid::new(20, 1),
            content: String::new(),
            prompt: None,
            keys: TextInputKeys::default(),
            box_: None,
            style: TextInputStyle {
                text: Style::default(),
                cursor: cursor_style, // explicit — no auto-reverse
            },
        });
        assert_eq!(input.style.cursor, cursor_style);
    }

    #[test]
    fn set_cursor_and_box() {
        let mut input = make_input("abc");
        input.set_cursor(1);
        input.update(Msg::key(Key::Char('X')));
        assert_eq!(input.content(), "aXbc");

        input.set_box(None);
        assert!(input.box_.is_none());
    }
}
