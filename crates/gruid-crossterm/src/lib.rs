//! Crossterm terminal driver for gruid.
//!
//! Provides a [`CrosstermDriver`] that implements [`gruid_core::Driver`],
//! mapping gruid's grid-based rendering model to a terminal via crossterm.

use std::io::{self, Write};
use std::sync::mpsc::Sender;
use std::time::Duration;

use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEventKind},
    execute,
    style::{self, Attribute, Color as CtColor, SetBackgroundColor, SetForegroundColor},
    terminal::{self, ClearType},
};

use gruid_core::{
    app::{Context, Driver},
    grid::Frame,
    messages::{Key, ModMask, Msg, MouseAction},
    style::{AttrMask, Color},
    Point,
};

use std::time::Instant;

/// Maps a [`gruid_core::Color`] to a [`crossterm::style::Color`].
fn to_ct_color(c: Color) -> CtColor {
    if c == Color::DEFAULT {
        CtColor::Reset
    } else {
        let (r, g, b) = (c.r(), c.g(), c.b());
        CtColor::Rgb { r, g, b }
    }
}

/// Maps crossterm key modifiers to gruid's [`ModMask`].
fn to_mod_mask(mods: KeyModifiers) -> ModMask {
    let mut m = ModMask::NONE;
    if mods.contains(KeyModifiers::SHIFT) {
        m = m | ModMask::SHIFT;
    }
    if mods.contains(KeyModifiers::CONTROL) {
        m = m | ModMask::CTRL;
    }
    if mods.contains(KeyModifiers::ALT) {
        m = m | ModMask::ALT;
    }
    if mods.contains(KeyModifiers::META) {
        m = m | ModMask::META;
    }
    m
}

/// Maps a crossterm [`KeyCode`] to a gruid [`Key`].
fn to_key(code: KeyCode) -> Option<Key> {
    match code {
        KeyCode::Char(c) => Some(Key::Char(c)),
        KeyCode::Enter => Some(Key::Enter),
        KeyCode::Esc => Some(Key::Escape),
        KeyCode::Backspace => Some(Key::Backspace),
        KeyCode::Tab => Some(Key::Tab),
        KeyCode::Delete => Some(Key::Delete),
        KeyCode::Insert => Some(Key::Insert),
        KeyCode::Home => Some(Key::Home),
        KeyCode::End => Some(Key::End),
        KeyCode::PageUp => Some(Key::PageUp),
        KeyCode::PageDown => Some(Key::PageDown),
        KeyCode::Up => Some(Key::ArrowUp),
        KeyCode::Down => Some(Key::ArrowDown),
        KeyCode::Left => Some(Key::ArrowLeft),
        KeyCode::Right => Some(Key::ArrowRight),
        _ => None,
    }
}

/// A terminal back-end for gruid using crossterm.
pub struct CrosstermDriver {
    mouse_enabled: bool,
}

impl CrosstermDriver {
    /// Create a new driver.
    pub fn new() -> Self {
        Self {
            mouse_enabled: true,
        }
    }

    /// Configure whether mouse events are captured.
    pub fn with_mouse(mut self, enabled: bool) -> Self {
        self.mouse_enabled = enabled;
        self
    }
}

impl Default for CrosstermDriver {
    fn default() -> Self {
        Self::new()
    }
}

impl Driver for CrosstermDriver {
    fn init(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        terminal::enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(
            stdout,
            terminal::EnterAlternateScreen,
            cursor::Hide,
            terminal::Clear(ClearType::All)
        )?;
        if self.mouse_enabled {
            execute!(stdout, event::EnableMouseCapture)?;
        }
        Ok(())
    }

    fn poll_msgs(
        &mut self,
        ctx: &Context,
        tx: Sender<Msg>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Non-blocking poll: check for an event with a short timeout.
        if !event::poll(Duration::from_millis(16))? {
            return Ok(());
        }

        while event::poll(Duration::ZERO)? {
            if ctx.is_done() {
                return Ok(());
            }

            let ev = event::read()?;

            let msg = match ev {
                Event::Key(KeyEvent {
                    code, modifiers, ..
                }) => {
                    if let Some(key) = to_key(code) {
                        Some(Msg::KeyDown {
                            key,
                            modifiers: to_mod_mask(modifiers),
                            time: Instant::now(),
                        })
                    } else {
                        None
                    }
                }
                Event::Mouse(me) => {
                    let pos = Point::new(me.column as i32, me.row as i32);
                    let modifiers = to_mod_mask(me.modifiers);
                    match me.kind {
                        MouseEventKind::Down(btn) => {
                            let action = match btn {
                                MouseButton::Left => MouseAction::Main,
                                MouseButton::Right => MouseAction::Secondary,
                                MouseButton::Middle => MouseAction::Auxiliary,
                            };
                            Some(Msg::Mouse {
                                action,
                                pos,
                                modifiers,
                                time: Instant::now(),
                            })
                        }
                        MouseEventKind::Up(_) => Some(Msg::Mouse {
                            action: MouseAction::Release,
                            pos,
                            modifiers,
                            time: Instant::now(),
                        }),
                        MouseEventKind::Moved | MouseEventKind::Drag(_) => Some(Msg::Mouse {
                            action: MouseAction::Move,
                            pos,
                            modifiers,
                            time: Instant::now(),
                        }),
                        MouseEventKind::ScrollUp => Some(Msg::Mouse {
                            action: MouseAction::WheelUp,
                            pos,
                            modifiers,
                            time: Instant::now(),
                        }),
                        MouseEventKind::ScrollDown => Some(Msg::Mouse {
                            action: MouseAction::WheelDown,
                            pos,
                            modifiers,
                            time: Instant::now(),
                        }),
                        _ => None,
                    }
                }
                Event::Resize(w, h) => Some(Msg::Screen {
                    width: w as i32,
                    height: h as i32,
                    time: Instant::now(),
                }),
                _ => None,
            };

            if let Some(m) = msg {
                tx.send(m).ok();
            }
        }

        Ok(())
    }

    fn flush(&mut self, frame: Frame) -> Result<(), Box<dyn std::error::Error>> {
        let mut stdout = io::stdout();

        for fc in &frame.cells {
            let p = fc.pos;
            let cell = &fc.cell;

            // Move cursor.
            execute!(stdout, cursor::MoveTo(p.x as u16, p.y as u16))?;

            // Set colours.
            execute!(
                stdout,
                SetForegroundColor(to_ct_color(cell.style.fg)),
                SetBackgroundColor(to_ct_color(cell.style.bg))
            )?;

            // Set attributes.
            let attrs = cell.style.attrs;
            if attrs.contains(AttrMask::BOLD) {
                execute!(stdout, style::SetAttribute(Attribute::Bold))?;
            }
            if attrs.contains(AttrMask::ITALIC) {
                execute!(stdout, style::SetAttribute(Attribute::Italic))?;
            }
            if attrs.contains(AttrMask::UNDERLINE) {
                execute!(stdout, style::SetAttribute(Attribute::Underlined))?;
            }
            if attrs.contains(AttrMask::REVERSE) {
                execute!(stdout, style::SetAttribute(Attribute::Reverse))?;
            }
            if attrs.contains(AttrMask::DIM) {
                execute!(stdout, style::SetAttribute(Attribute::Dim))?;
            }

            // Print character.
            write!(stdout, "{}", cell.ch)?;

            // Reset attributes.
            if attrs != AttrMask::NONE {
                execute!(stdout, style::SetAttribute(Attribute::Reset))?;
            }
        }

        stdout.flush()?;
        Ok(())
    }

    fn close(&mut self) {
        let mut stdout = io::stdout();
        if self.mouse_enabled {
            let _ = execute!(stdout, event::DisableMouseCapture);
        }
        let _ = execute!(
            stdout,
            cursor::Show,
            terminal::LeaveAlternateScreen
        );
        let _ = terminal::disable_raw_mode();
    }
}
