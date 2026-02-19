//! Translates winit input events into gruid [`Msg`] values.

use std::time::Instant;

use winit::dpi::PhysicalPosition;
use winit::event::{ElementState, KeyEvent, MouseButton, MouseScrollDelta};
use winit::keyboard::{Key as WKey, NamedKey};

use gruid_core::{
    Point,
    messages::{Key, ModMask, MouseAction, Msg},
};

// ---------------------------------------------------------------------------
// Keyboard
// ---------------------------------------------------------------------------

pub(crate) fn translate_keyboard(event: &KeyEvent) -> Option<Msg> {
    if event.state != ElementState::Pressed {
        return None;
    }

    let key = match &event.logical_key {
        WKey::Named(named) => match named {
            NamedKey::ArrowUp => Key::ArrowUp,
            NamedKey::ArrowDown => Key::ArrowDown,
            NamedKey::ArrowLeft => Key::ArrowLeft,
            NamedKey::ArrowRight => Key::ArrowRight,
            NamedKey::Escape => Key::Escape,
            NamedKey::Enter => Key::Enter,
            NamedKey::Tab => Key::Tab,
            NamedKey::Space => Key::Space,
            NamedKey::Backspace => Key::Backspace,
            NamedKey::Delete => Key::Delete,
            NamedKey::Home => Key::Home,
            NamedKey::End => Key::End,
            NamedKey::PageUp => Key::PageUp,
            NamedKey::PageDown => Key::PageDown,
            NamedKey::Insert => Key::Insert,
            _ => return None,
        },
        WKey::Character(s) => {
            let mut chars = s.chars();
            match (chars.next(), chars.next()) {
                (Some(c), None) => Key::Char(c),
                _ => return None,
            }
        }
        _ => return None,
    };

    let modifiers = ModMask::NONE;

    Some(Msg::KeyDown {
        key,
        modifiers,
        time: Instant::now(),
    })
}

// ---------------------------------------------------------------------------
// Mouse
// ---------------------------------------------------------------------------

pub(crate) fn pixel_to_grid(px: f64, py: f64, cell_w: usize, cell_h: usize) -> Point {
    Point::new(
        (px as i32) / (cell_w as i32).max(1),
        (py as i32) / (cell_h as i32).max(1),
    )
}

pub(crate) fn translate_mouse_button(
    btn_state: ElementState,
    button: MouseButton,
    cell_w: usize,
    cell_h: usize,
) -> Option<Msg> {
    let action = match btn_state {
        ElementState::Pressed => match button {
            MouseButton::Left => MouseAction::Main,
            MouseButton::Right => MouseAction::Secondary,
            MouseButton::Middle => MouseAction::Auxiliary,
            _ => return None,
        },
        ElementState::Released => MouseAction::Release,
    };

    let pos = pixel_to_grid(0.0, 0.0, cell_w, cell_h);

    Some(Msg::Mouse {
        action,
        pos,
        modifiers: ModMask::NONE,
        time: Instant::now(),
    })
}

pub(crate) fn translate_cursor_moved(
    position: PhysicalPosition<f64>,
    cell_w: usize,
    cell_h: usize,
) -> Option<Msg> {
    let pos = pixel_to_grid(position.x, position.y, cell_w, cell_h);
    Some(Msg::Mouse {
        action: MouseAction::Move,
        pos,
        modifiers: ModMask::NONE,
        time: Instant::now(),
    })
}

pub(crate) fn translate_mouse_wheel(
    delta: MouseScrollDelta,
    cell_w: usize,
    cell_h: usize,
) -> Option<Msg> {
    let (_, y) = match delta {
        MouseScrollDelta::LineDelta(x, y) => (x as f64, y as f64),
        MouseScrollDelta::PixelDelta(pos) => (pos.x, pos.y),
    };

    let action = if y > 0.0 {
        MouseAction::WheelUp
    } else if y < 0.0 {
        MouseAction::WheelDown
    } else {
        return None;
    };

    let pos = pixel_to_grid(0.0, 0.0, cell_w, cell_h);
    Some(Msg::Mouse {
        action,
        pos,
        modifiers: ModMask::NONE,
        time: Instant::now(),
    })
}
