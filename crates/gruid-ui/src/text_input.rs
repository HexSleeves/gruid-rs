use gruid_core::messages::{Key, Msg};
use gruid_core::{Cell, Grid, Point, Style};

use crate::BoxDecor;

/// Configuration for a [`TextInput`] widget.
#[derive(Debug, Clone)]
pub struct TextInputConfig {
    /// Grid to draw into.
    pub grid: Grid,
    /// Initial content.
    pub content: String,
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
    _grid: Grid,
    content: String,
    cursor: usize,
    keys: TextInputKeys,
    box_: Option<BoxDecor>,
    style: TextInputStyle,
    action: TextInputAction,
}

impl TextInput {
    /// Create a new text input from the given configuration.
    pub fn new(config: TextInputConfig) -> Self {
        let cursor = config.content.len();
        Self {
            _grid: config.grid,
            content: config.content,
            cursor,
            keys: config.keys,
            box_: config.box_,
            style: config.style,
            action: TextInputAction::Pass,
        }
    }

    /// Process an input message and return the resulting action.
    pub fn update(&mut self, msg: Msg) -> TextInputAction {
        self.action = TextInputAction::Pass;

        if let Msg::KeyDown { ref key, .. } = msg {
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

        self.action
    }

    /// Draw the text input into the given grid.
    pub fn draw(&self, grid: &Grid) {
        let inner_range = if let Some(ref box_decor) = self.box_ {
            box_decor.draw(grid)
        } else {
            grid.bounds()
        };

        let start = inner_range.min;
        let vis_w = (inner_range.max.x - inner_range.min.x) as usize;
        let y = start.y;

        // Compute a scroll offset so the cursor is visible.
        let cursor_char_pos = self.content[..self.cursor].chars().count();
        let scroll = if cursor_char_pos >= vis_w {
            cursor_char_pos - vis_w + 1
        } else {
            0
        };

        let chars: Vec<char> = self.content.chars().collect();
        for col in 0..vis_w {
            let char_idx = scroll + col;
            let x = start.x + col as i32;
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
}
