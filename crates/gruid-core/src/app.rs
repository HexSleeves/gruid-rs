//! The Elm-architecture application loop: [`Model`], [`Driver`], [`Effect`],
//! [`App`].
//!
//! Two driver models are supported:
//!
//! - **Poll-based** ([`Driver`]): the app calls `poll_msgs` in a loop
//!   (crossterm, stdin-based terminals).
//! - **Event-loop-based** ([`EventLoopDriver`]): the driver owns the main
//!   thread event loop and pushes events into an [`AppRunner`] that the
//!   driver calls into (winit, SDL2, browser).

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Arc;

use crate::grid::{compute_frame, Frame, Grid};
use crate::messages::Msg;

// ---------------------------------------------------------------------------
// Context (cancellation token)
// ---------------------------------------------------------------------------

/// A simple cooperative-cancellation token backed by an [`AtomicBool`].
#[derive(Clone, Debug)]
pub struct Context {
    done: Arc<AtomicBool>,
}

impl Context {
    /// Create a new, non-cancelled context.
    pub fn new() -> Self {
        Self {
            done: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Whether cancellation has been requested.
    #[inline]
    pub fn is_done(&self) -> bool {
        self.done.load(Ordering::Relaxed)
    }

    /// Request cancellation.
    #[inline]
    pub fn cancel(&self) {
        self.done.store(true, Ordering::Relaxed);
    }
}

impl Default for Context {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Effect / Cmd
// ---------------------------------------------------------------------------

/// A side-effect returned by [`Model::update`].
pub enum Effect {
    /// A one-shot command that produces an optional follow-up message.
    Cmd(Box<dyn FnOnce() -> Option<Msg> + Send>),
    /// A long-running subscription that may send many messages.
    Sub(Box<dyn FnOnce(Context, Sender<Msg>) + Send>),
    /// Multiple effects batched together.
    Batch(Vec<Effect>),
    /// Signal the application loop to stop.
    End,
}

impl std::fmt::Debug for Effect {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Cmd(_) => f.write_str("Effect::Cmd(..)"),
            Self::Sub(_) => f.write_str("Effect::Sub(..)"),
            Self::Batch(v) => f.debug_tuple("Effect::Batch").field(&v.len()).finish(),
            Self::End => f.write_str("Effect::End"),
        }
    }
}

/// Convenience constructor for a [`Effect::Cmd`].
pub fn cmd<F>(f: F) -> Effect
where
    F: FnOnce() -> Option<Msg> + Send + 'static,
{
    Effect::Cmd(Box::new(f))
}

/// Convenience type alias.
pub type Cmd = Effect;

// ---------------------------------------------------------------------------
// Model trait
// ---------------------------------------------------------------------------

/// The application model (Elm architecture).
pub trait Model {
    /// Process a message, optionally returning a side-effect.
    fn update(&mut self, msg: Msg) -> Option<Effect>;

    /// Render the current state into `grid`.
    fn draw(&self, grid: &mut Grid);
}

// ---------------------------------------------------------------------------
// Driver trait (poll-based: crossterm, etc.)
// ---------------------------------------------------------------------------

/// Poll-based back-end driver (e.g. terminal via crossterm).
///
/// The [`App`] calls [`poll_msgs`](Driver::poll_msgs) in a loop on the main
/// thread.  For event-loop-based backends (winit, SDL2) see
/// [`EventLoopDriver`] and [`AppRunner`] instead.
pub trait Driver {
    /// Initialise the back-end.
    fn init(&mut self) -> Result<(), Box<dyn std::error::Error>>;

    /// Poll for input messages, sending them through `tx`.
    ///
    /// Should return promptly (non-blocking or short timeout) so the
    /// app can draw.  Honour `ctx.is_done()` and return when it is `true`.
    fn poll_msgs(
        &mut self,
        ctx: &Context,
        tx: Sender<Msg>,
    ) -> Result<(), Box<dyn std::error::Error>>;

    /// Flush a computed frame to the screen.
    fn flush(&mut self, frame: Frame) -> Result<(), Box<dyn std::error::Error>>;

    /// Clean up / restore the terminal.
    fn close(&mut self);
}

// ---------------------------------------------------------------------------
// EventLoopDriver trait (winit, SDL2, browser)
// ---------------------------------------------------------------------------

/// Event-loop-based back-end driver.
///
/// The driver owns the main thread event loop and drives the application
/// through an [`AppRunner`].  This is the correct pattern for winit, SDL2,
/// and browser backends where the platform event loop must run on the main
/// thread.
pub trait EventLoopDriver {
    /// Run the event loop.  The driver should:
    ///
    /// 1. Create its window / surface.
    /// 2. Call `runner.init()` once.
    /// 3. For each input event, call `runner.handle_msg(msg)`.
    /// 4. When `runner.should_quit()` is true, exit.
    /// 5. After processing events, call `runner.draw_frame()` to get the
    ///    frame diff and render it.
    fn run(self, runner: AppRunner) -> Result<(), Box<dyn std::error::Error>>;
}

// ---------------------------------------------------------------------------
// AppRunner — the model+grid state machine for event-loop drivers
// ---------------------------------------------------------------------------

/// Encapsulates the Model-View-Update state machine for use by
/// [`EventLoopDriver`] implementations.
///
/// The driver pushes messages in, and pulls frames out.
pub struct AppRunner {
    model: Box<dyn Model>,
    prev_grid: Grid,
    curr_grid: Grid,
    ctx: Context,
    needs_draw: bool,
}

impl AppRunner {
    /// Create a new runner.  The driver should call [`init`](Self::init)
    /// before processing events.
    pub fn new(model: Box<dyn Model>, width: i32, height: i32) -> Self {
        Self {
            model,
            prev_grid: Grid::new(width, height),
            curr_grid: Grid::new(width, height),
            ctx: Context::new(),
            needs_draw: false,
        }
    }

    /// Send the `Msg::Init` message to the model.  Call once at startup.
    pub fn init(&mut self) {
        self.handle_msg(Msg::Init);
    }

    /// Push a message into the model.
    pub fn handle_msg(&mut self, msg: Msg) {
        if let Some(effect) = self.model.update(msg) {
            self.handle_effect(effect);
        }
        self.needs_draw = true;
    }

    /// Whether the model has requested the app to stop.
    pub fn should_quit(&self) -> bool {
        self.ctx.is_done()
    }

    /// Compute a diff frame if anything changed since the last call.
    ///
    /// Returns `Some(frame)` if the model was updated, `None` otherwise.
    pub fn draw_frame(&mut self) -> Option<Frame> {
        if !self.needs_draw {
            return None;
        }
        self.needs_draw = false;
        self.model.draw(&mut self.curr_grid);
        let frame = compute_frame(&self.prev_grid, &self.curr_grid);
        self.prev_grid.copy_from(&self.curr_grid);
        if frame.cells.is_empty() {
            None
        } else {
            Some(frame)
        }
    }

    /// The current grid width.
    pub fn width(&self) -> i32 {
        self.curr_grid.width()
    }

    /// The current grid height.
    pub fn height(&self) -> i32 {
        self.curr_grid.height()
    }

    /// Resize the grids (e.g. when the window is resized and the cell
    /// count changes).
    pub fn resize(&mut self, width: i32, height: i32) {
        self.prev_grid = Grid::new(width, height);
        self.curr_grid = Grid::new(width, height);
        self.needs_draw = true;
    }

    fn handle_effect(&mut self, effect: Effect) {
        match effect {
            Effect::End => {
                self.ctx.cancel();
            }
            Effect::Cmd(f) => {
                if let Some(msg) = f() {
                    self.handle_msg(msg);
                }
            }
            Effect::Sub(_f) => {
                // TODO: spawn background task
            }
            Effect::Batch(effects) => {
                for e in effects {
                    self.handle_effect(e);
                    if self.ctx.is_done() {
                        return;
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// AppConfig / App  (poll-based driver)
// ---------------------------------------------------------------------------

/// Configuration for creating an [`App`].
pub struct AppConfig<M: Model, D: Driver> {
    pub model: M,
    pub driver: D,
    pub width: i32,
    pub height: i32,
    pub frame_writer: Option<Box<dyn std::io::Write>>,
}

/// The main application runner for poll-based [`Driver`]s.
pub struct App<M: Model, D: Driver> {
    model: M,
    driver: D,
    width: i32,
    height: i32,
    _frame_writer: Option<Box<dyn std::io::Write>>,
}

impl<M: Model, D: Driver> App<M, D> {
    /// Create a new application from a configuration.
    pub fn new(config: AppConfig<M, D>) -> Self {
        Self {
            model: config.model,
            driver: config.driver,
            width: config.width,
            height: config.height,
            _frame_writer: config.frame_writer,
        }
    }

    /// Run the main Model-View-Update loop.
    ///
    /// 1. Initialises the driver.
    /// 2. Sends `Msg::Init` through the model.
    /// 3. Enters the event loop: poll → update → draw → diff → flush.
    /// 4. Stops when the model returns `Effect::End` or the driver signals
    ///    quit.
    pub fn run(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.driver.init()?;

        let ctx = Context::new();
        let (tx, rx): (Sender<Msg>, Receiver<Msg>) = mpsc::channel();

        // Seed with Init.
        tx.send(Msg::Init).ok();

        let mut prev_grid = Grid::new(self.width, self.height);
        let mut curr_grid = Grid::new(self.width, self.height);

        // Process the Init message first.
        self.process_pending(&rx, &ctx, &tx, &mut prev_grid, &mut curr_grid)?;

        // Main loop: poll then process.
        while !ctx.is_done() {
            match self.driver.poll_msgs(&ctx, tx.clone()) {
                Ok(()) => {}
                Err(e) => {
                    ctx.cancel();
                    self.driver.close();
                    return Err(e);
                }
            }

            if ctx.is_done() {
                break;
            }

            self.process_pending(&rx, &ctx, &tx, &mut prev_grid, &mut curr_grid)?;
        }

        self.driver.close();
        Ok(())
    }

    fn process_pending(
        &mut self,
        rx: &Receiver<Msg>,
        ctx: &Context,
        _tx: &Sender<Msg>,
        prev_grid: &mut Grid,
        curr_grid: &mut Grid,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut needs_draw = false;

        while let Ok(msg) = rx.try_recv() {
            if let Some(effect) = self.model.update(msg) {
                if self.handle_effect(effect, ctx) {
                    return Ok(());
                }
            }
            needs_draw = true;
        }

        if needs_draw {
            self.model.draw(curr_grid);
            let frame = compute_frame(prev_grid, curr_grid);
            if !frame.cells.is_empty() {
                self.driver.flush(frame)?;
            }
            prev_grid.copy_from(curr_grid);
        }

        Ok(())
    }

    fn handle_effect(&self, effect: Effect, ctx: &Context) -> bool {
        match effect {
            Effect::End => {
                ctx.cancel();
                true
            }
            Effect::Cmd(f) => {
                let _msg = f();
                false
            }
            Effect::Sub(_f) => {
                false
            }
            Effect::Batch(effects) => {
                for e in effects {
                    if self.handle_effect(e, ctx) {
                        return true;
                    }
                }
                false
            }
        }
    }
}
