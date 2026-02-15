//! Translates winit input events into gruid [`Msg`] values.

use std::time::Instant;

use winit::dpi::PhysicalPosition;
use winit::event::{ElementState, KeyEvent, MouseButton, MouseScrollDelta};
use winit::keyboard::{Key as WKey, NamedKey};

use gruid_core::{
    Point,
    messages::{Key, ModMask, MouseAction, Msg},
};

use crate::WinitState;

// ---------------------------------------------------------------------------
// Keyboard
// ---------------------------------------------------------------------------

pub(crate) fn translate_keyboard(event: &KeyEvent) -> Option<Msg> {
    // Only key-down (pressed) events.
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

    // Modifier extraction — winit doesn't expose modifiers on KeyEvent
    // directly in 0.30 in a simple way; we rely on the logical key already
    // incorporating shift (e.g. 'A' vs 'a').  For Ctrl/Alt combos the
    // character is already translated.  We pass NONE for now — a more
    // complete implementation would track modifier state via
    // WindowEvent::ModifiersChanged.
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

fn pixel_to_grid(px: f64, py: f64, state: Option<&WinitState>) -> Point {
    let (cw, ch) = state.map(|s| s.renderer.cell_size()).unwrap_or((8, 16));
    Point::new(
        (px as i32) / (cw as i32).max(1),
        (py as i32) / (ch as i32).max(1),
    )
}

pub(crate) fn translate_mouse_button(
    btn_state: ElementState,
    button: MouseButton,
    state: Option<&WinitState>,
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

    // We don't have the cursor position in mouse button events.
    // Use (0,0) as fallback — a real implementation would track last cursor pos.
    let pos = pixel_to_grid(0.0, 0.0, state);

    Some(Msg::Mouse {
        action,
        pos,
        modifiers: ModMask::NONE,
        time: Instant::now(),
    })
}

pub(crate) fn translate_cursor_moved(
    position: PhysicalPosition<f64>,
    state: Option<&WinitState>,
) -> Option<Msg> {
    let pos = pixel_to_grid(position.x, position.y, state);
    Some(Msg::Mouse {
        action: MouseAction::Move,
        pos,
        modifiers: ModMask::NONE,
        time: Instant::now(),
    })
}

pub(crate) fn translate_mouse_wheel(
    delta: MouseScrollDelta,
    state: Option<&WinitState>,
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

    let pos = pixel_to_grid(0.0, 0.0, state);
    Some(Msg::Mouse {
        action,
        pos,
        modifiers: ModMask::NONE,
        time: Instant::now(),
    })
}
