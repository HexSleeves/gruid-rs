//! WASM browser driver for **gruid** using Canvas 2D.
//!
//! This crate provides [`WebDriver`], an [`EventLoopDriver`] that renders a
//! gruid application inside an HTML `<canvas>` element.  Each grid cell is
//! drawn as a background-colour rectangle plus a foreground-colour character
//! via `CanvasRenderingContext2d.fillText()`.
//!
//! # Limitations
//!
//! * **No threading.** WASM's main thread cannot spawn OS threads, so
//!   [`Effect::Cmd`] and [`Effect::Sub`] will panic at runtime.  Avoid
//!   returning those effects in WASM builds — use `Effect::End` or
//!   `Effect::Batch` only.
//! * The driver takes ownership of the browser event loop via
//!   `requestAnimationFrame` and event listeners; there is no way to
//!   "return" from [`EventLoopDriver::run`].
//!
//! # Quick start
//!
//! ```html
//! <canvas id="gruid-canvas" tabindex="1"></canvas>
//! <script type="module">
//!   import init, { start } from './pkg/my_app.js';
//!   await init();
//!   start();
//! </script>
//! ```
//!
//! ```rust,ignore
//! use gruid_core::{AppRunner, EventLoopDriver, Model};
//! use gruid_web::{WebConfig, WebDriver};
//! use wasm_bindgen::prelude::*;
//!
//! #[wasm_bindgen]
//! pub fn start() {
//!     let config = WebConfig::default();
//!     let driver = WebDriver::new(config);
//!     let runner = AppRunner::new(Box::new(MyModel::new()), 80, 24);
//!     driver.run(runner).expect("driver failed");
//! }
//! ```

use std::cell::RefCell;
use std::rc::Rc;

use gruid_core::{
    AppRunner, EventLoopDriver, Point,
    grid::Frame,
    messages::{Key, ModMask, MouseAction, Msg},
    style::Color,
};

use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use web_sys::{
    CanvasRenderingContext2d, Document, HtmlCanvasElement, KeyboardEvent, MouseEvent, WheelEvent,
    Window,
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn window() -> Window {
    web_sys::window().expect("no global `window`")
}

fn document() -> Document {
    window().document().expect("no `document`")
}

fn now_instant() -> std::time::Instant {
    // `Instant::now()` works in wasm32-unknown-unknown (delegates to
    // `performance.now()` when available).  If the target doesn't
    // support it the call still compiles — it just returns epoch.
    std::time::Instant::now()
}

/// Convert a gruid [`Color`] to a CSS colour string.
fn color_to_css(color: Color, default: &str) -> String {
    if color == Color::DEFAULT {
        default.to_string()
    } else {
        format!("rgb({},{},{})", color.r(), color.g(), color.b())
    }
}

/// Translate a browser `KeyboardEvent.key` string to a gruid [`Key`].
fn translate_key(key: &str, code: &str) -> Option<Key> {
    // Handle Numpad5 with non-"5" key (treated as Enter, matching Go driver)
    if code == "Numpad5" && key != "5" {
        return Some(Key::Enter);
    }
    match key {
        "ArrowDown" => Some(Key::ArrowDown),
        "ArrowUp" => Some(Key::ArrowUp),
        "ArrowLeft" => Some(Key::ArrowLeft),
        "ArrowRight" => Some(Key::ArrowRight),
        "Backspace" => Some(Key::Backspace),
        "Delete" => Some(Key::Delete),
        "End" => Some(Key::End),
        "Enter" => Some(Key::Enter),
        "Escape" => Some(Key::Escape),
        "Home" => Some(Key::Home),
        "Insert" => Some(Key::Insert),
        "PageUp" => Some(Key::PageUp),
        "PageDown" => Some(Key::PageDown),
        " " => Some(Key::Space),
        "Tab" => Some(Key::Tab),
        other => {
            let mut chars = other.chars();
            let first = chars.next()?;
            if chars.next().is_some() {
                // Multi-character string → not a single printable key
                return None;
            }
            Some(Key::Char(first))
        }
    }
}

/// Build a [`ModMask`] from a browser keyboard/mouse event's modifier flags.
fn modifier_mask(shift: bool, ctrl: bool, alt: bool, meta: bool) -> ModMask {
    let mut m = ModMask::NONE;
    if shift {
        m = m | ModMask::SHIFT;
    }
    if ctrl {
        m = m | ModMask::CTRL;
    }
    if alt {
        m = m | ModMask::ALT;
    }
    if meta {
        m = m | ModMask::META;
    }
    m
}

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// Configuration for [`WebDriver`].
#[derive(Clone, Debug)]
pub struct WebConfig {
    /// The `id` attribute of the `<canvas>` element (default: `"gruid-canvas"`).
    pub canvas_id: String,
    /// Font size in pixels for `fillText` (default: `16.0`).
    pub font_size: f64,
    /// Font family CSS value (default: `"monospace"`).
    pub font_family: String,
    /// Grid width in cells (default: `80`).
    pub width: i32,
    /// Grid height in cells (default: `24`).
    pub height: i32,
}

impl Default for WebConfig {
    fn default() -> Self {
        Self {
            canvas_id: "gruid-canvas".into(),
            font_size: 16.0,
            font_family: "monospace".into(),
            width: 80,
            height: 24,
        }
    }
}

// ---------------------------------------------------------------------------
// WebDriver
// ---------------------------------------------------------------------------

/// A WASM browser driver that renders a gruid app on an HTML canvas.
///
/// Implements [`EventLoopDriver`].  See the [crate-level docs](crate) for
/// usage instructions.
pub struct WebDriver {
    config: WebConfig,
}

impl WebDriver {
    /// Create a new driver from the given configuration.
    pub fn new(config: WebConfig) -> Self {
        Self { config }
    }
}

// ---------------------------------------------------------------------------
// Shared state used inside closures
// ---------------------------------------------------------------------------

/// All mutable state shared between the rAF loop and event-listener closures.
struct Shared {
    runner: AppRunner,
    ctx: CanvasRenderingContext2d,
    cell_w: f64,
    cell_h: f64,
    font_css: String,
    mouse_pos: Point,
    mouse_drag: i32, // button number being dragged, or -1
}

impl Shared {
    /// Push a message through the runner and immediately try to render.
    fn handle_and_render(&mut self, msg: Msg) {
        self.runner.handle_msg(msg);
        self.render();
    }

    /// Render any pending frame diff to the canvas.
    fn render(&mut self) {
        self.runner.process_pending_msgs();
        if let Some(frame) = self.runner.draw_frame() {
            self.flush(frame);
        }
    }

    /// Paint a frame diff onto the canvas.
    fn flush(&self, frame: Frame) {
        let ctx = &self.ctx;
        let cw = self.cell_w;
        let ch = self.cell_h;

        for fc in &frame.cells {
            let px = fc.pos.x as f64 * cw;
            let py = fc.pos.y as f64 * ch;

            // Background
            let bg = color_to_css(fc.cell.style.bg, "#000000");
            ctx.set_fill_style_str(&bg);
            ctx.fill_rect(px, py, cw, ch);

            // Foreground character
            if fc.cell.ch != ' ' {
                let fg = color_to_css(fc.cell.style.fg, "#ffffff");
                ctx.set_fill_style_str(&fg);
                ctx.set_font(&self.font_css);
                // Draw text at baseline (roughly cell bottom minus a small descent)
                let text_y = py + ch * 0.85;
                let _ = ctx.fill_text(&fc.cell.ch.to_string(), px, text_y);
            }
        }
    }

    /// Convert a mouse event's client coordinates to grid cell coordinates.
    fn mouse_to_cell(&self, evt: &MouseEvent, canvas: &HtmlCanvasElement) -> Point {
        let rect = canvas.get_bounding_client_rect();
        let scale_x = canvas.width() as f64 / rect.width();
        let scale_y = canvas.height() as f64 / rect.height();
        let x = (evt.client_x() as f64 - rect.left()) * scale_x;
        let y = (evt.client_y() as f64 - rect.top()) * scale_y;
        Point::new(
            ((x - 1.0).max(0.0) / self.cell_w) as i32,
            ((y - 1.0).max(0.0) / self.cell_h) as i32,
        )
    }
}

// ---------------------------------------------------------------------------
// EventLoopDriver implementation
// ---------------------------------------------------------------------------

impl EventLoopDriver for WebDriver {
    fn run(self, mut runner: AppRunner) -> Result<(), Box<dyn std::error::Error>> {
        let cfg = &self.config;

        // --- canvas & context -----------------------------------------------
        let canvas: HtmlCanvasElement = document()
            .get_element_by_id(&cfg.canvas_id)
            .unwrap_or_else(|| panic!("canvas element '{}' not found", cfg.canvas_id))
            .dyn_into::<HtmlCanvasElement>()
            .expect("element is not a canvas");
        canvas
            .set_attribute("tabindex", "1")
            .expect("failed to set tabindex");

        let ctx: CanvasRenderingContext2d = canvas
            .get_context("2d")
            .expect("getContext failed")
            .expect("no 2d context")
            .dyn_into::<CanvasRenderingContext2d>()
            .expect("context is not CanvasRenderingContext2d");

        // --- font metrics ---------------------------------------------------
        let font_css = format!("{}px {}", cfg.font_size, cfg.font_family);
        ctx.set_font(&font_css);
        let metrics = ctx.measure_text("M").expect("measureText failed");
        let cell_w = metrics.width().ceil();
        // Use font_size as cell height (good enough for monospace)
        let cell_h = (cfg.font_size * 1.2).ceil();

        // --- size canvas ----------------------------------------------------
        canvas.set_width((cell_w * cfg.width as f64) as u32);
        canvas.set_height((cell_h * cfg.height as f64) as u32);

        // --- init model -----------------------------------------------------
        runner.init();

        let shared = Rc::new(RefCell::new(Shared {
            runner,
            ctx,
            cell_w,
            cell_h,
            font_css,
            mouse_pos: Point::new(-1, -1),
            mouse_drag: -1,
        }));

        // Initial render
        {
            let mut s = shared.borrow_mut();
            s.render();
        }

        // --- event listeners ------------------------------------------------

        // We keep Closures alive for the lifetime of the page by leaking them
        // (`.forget()`).  This is standard practice for wasm_bindgen event
        // listeners that should live forever.

        // -- contextmenu (prevent right-click menu) --------------------------
        {
            let closure = Closure::<dyn FnMut(MouseEvent)>::new(move |e: MouseEvent| {
                e.prevent_default();
            });
            canvas
                .add_event_listener_with_callback("contextmenu", closure.as_ref().unchecked_ref())
                .expect("addEventListener contextmenu");
            closure.forget();
        }

        // -- keydown ---------------------------------------------------------
        {
            let shared = Rc::clone(&shared);
            let closure = Closure::<dyn FnMut(KeyboardEvent)>::new(move |e: KeyboardEvent| {
                // Skip events with ctrl/meta/alt to avoid conflicting with
                // browser shortcuts (matches Go driver behaviour).
                if e.ctrl_key() || e.meta_key() || e.alt_key() {
                    return;
                }
                let key_str = e.key();
                let code = e.code();
                if let Some(key) = translate_key(&key_str, &code) {
                    let mut mods = ModMask::NONE;
                    if e.shift_key() {
                        mods = mods | ModMask::SHIFT;
                    }
                    e.prevent_default();
                    let msg = Msg::KeyDown {
                        key,
                        modifiers: mods,
                        time: now_instant(),
                    };
                    shared.borrow_mut().handle_and_render(msg);
                }
            });
            // Listen on document (not just canvas) so keys are caught even
            // when the canvas isn't focused, matching the Go driver.
            document()
                .add_event_listener_with_callback("keydown", closure.as_ref().unchecked_ref())
                .expect("addEventListener keydown");
            closure.forget();
        }

        // -- mousedown -------------------------------------------------------
        {
            let shared = Rc::clone(&shared);
            let canvas_clone = canvas.clone();
            let closure = Closure::<dyn FnMut(MouseEvent)>::new(move |e: MouseEvent| {
                if e.ctrl_key() || e.meta_key() || e.shift_key() || e.alt_key() {
                    return;
                }
                e.prevent_default();
                let mut s = shared.borrow_mut();
                if s.mouse_drag >= 0 {
                    return;
                }
                let button = e.button();
                let action = match button {
                    0 => MouseAction::Main,
                    1 => MouseAction::Auxiliary,
                    2 => MouseAction::Secondary,
                    _ => return,
                };
                s.mouse_drag = button as i32;
                let pos = s.mouse_to_cell(&e, &canvas_clone);
                let mods = modifier_mask(e.shift_key(), e.ctrl_key(), e.alt_key(), e.meta_key());
                let msg = Msg::Mouse {
                    action,
                    pos,
                    modifiers: mods,
                    time: now_instant(),
                };
                s.handle_and_render(msg);
            });
            canvas
                .add_event_listener_with_callback("mousedown", closure.as_ref().unchecked_ref())
                .expect("addEventListener mousedown");
            closure.forget();
        }

        // -- mouseup ---------------------------------------------------------
        {
            let shared = Rc::clone(&shared);
            let canvas_clone = canvas.clone();
            let closure = Closure::<dyn FnMut(MouseEvent)>::new(move |e: MouseEvent| {
                if e.ctrl_key() || e.meta_key() || e.shift_key() || e.alt_key() {
                    return;
                }
                e.prevent_default();
                let mut s = shared.borrow_mut();
                let button = e.button() as i32;
                if s.mouse_drag != button {
                    return;
                }
                s.mouse_drag = -1;
                let pos = s.mouse_to_cell(&e, &canvas_clone);
                let mods = modifier_mask(e.shift_key(), e.ctrl_key(), e.alt_key(), e.meta_key());
                let msg = Msg::Mouse {
                    action: MouseAction::Release,
                    pos,
                    modifiers: mods,
                    time: now_instant(),
                };
                s.handle_and_render(msg);
            });
            canvas
                .add_event_listener_with_callback("mouseup", closure.as_ref().unchecked_ref())
                .expect("addEventListener mouseup");
            closure.forget();
        }

        // -- mousemove -------------------------------------------------------
        {
            let shared = Rc::clone(&shared);
            let canvas_clone = canvas.clone();
            let closure = Closure::<dyn FnMut(MouseEvent)>::new(move |e: MouseEvent| {
                e.prevent_default();
                let mut s = shared.borrow_mut();
                let pos = s.mouse_to_cell(&e, &canvas_clone);
                if pos != s.mouse_pos {
                    s.mouse_pos = pos;
                    let mods =
                        modifier_mask(e.shift_key(), e.ctrl_key(), e.alt_key(), e.meta_key());
                    let msg = Msg::Mouse {
                        action: MouseAction::Move,
                        pos,
                        modifiers: mods,
                        time: now_instant(),
                    };
                    s.handle_and_render(msg);
                }
            });
            canvas
                .add_event_listener_with_callback("mousemove", closure.as_ref().unchecked_ref())
                .expect("addEventListener mousemove");
            closure.forget();
        }

        // -- wheel -----------------------------------------------------------
        {
            let shared = Rc::clone(&shared);
            let canvas_clone = canvas.clone();
            let closure = Closure::<dyn FnMut(WheelEvent)>::new(move |e: WheelEvent| {
                e.prevent_default();
                let delta = e.delta_y();
                let action = if delta > 0.0 {
                    MouseAction::WheelDown
                } else if delta < 0.0 {
                    MouseAction::WheelUp
                } else {
                    return;
                };
                let mut s = shared.borrow_mut();
                // WheelEvent inherits from MouseEvent
                let mouse_evt: &MouseEvent = e.as_ref();
                let pos = s.mouse_to_cell(mouse_evt, &canvas_clone);
                let mods = modifier_mask(e.shift_key(), e.ctrl_key(), e.alt_key(), e.meta_key());
                let msg = Msg::Mouse {
                    action,
                    pos,
                    modifiers: mods,
                    time: now_instant(),
                };
                s.handle_and_render(msg);
            });
            canvas
                .add_event_listener_with_callback("wheel", closure.as_ref().unchecked_ref())
                .expect("addEventListener wheel");
            closure.forget();
        }

        // --- requestAnimationFrame loop -------------------------------------
        // We use a recurring rAF callback to process any pending background
        // messages and re-render.  Actual input handling happens eagerly in
        // the event-listener closures above, so the rAF loop mainly services
        // Cmd/Sub feedback and keeps the display up to date.
        {
            let shared = Rc::clone(&shared);
            // The closure must own an Rc to itself so it can re-register.
            let raf_cb: Rc<RefCell<Option<Closure<dyn FnMut()>>>> = Rc::new(RefCell::new(None));
            let raf_cb2 = Rc::clone(&raf_cb);

            *raf_cb.borrow_mut() = Some(Closure::<dyn FnMut()>::new(move || {
                {
                    let mut s = shared.borrow_mut();
                    if s.runner.should_quit() {
                        return; // stop the rAF loop
                    }
                    s.render();
                }
                // Schedule the next frame.
                let cb_ref = raf_cb2.borrow();
                if let Some(cb) = cb_ref.as_ref() {
                    let _ = window().request_animation_frame(cb.as_ref().unchecked_ref());
                }
            }));

            // Kick off the first frame.
            {
                let cb_ref = raf_cb.borrow();
                if let Some(cb) = cb_ref.as_ref() {
                    window()
                        .request_animation_frame(cb.as_ref().unchecked_ref())
                        .expect("requestAnimationFrame");
                }
            }

            // Leak the closure so it lives for the page lifetime.
            // (We already `.forget()` the event listeners; the rAF closure
            // must also be leaked since there's no teardown path in WASM.)
            std::mem::forget(raf_cb);
        }

        Ok(())
    }
}
