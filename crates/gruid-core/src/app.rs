//! The Elm-architecture application loop: [`Model`], [`Driver`], [`Effect`],
//! [`App`].

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
// Driver trait
// ---------------------------------------------------------------------------

/// Back-end driver (e.g. terminal, graphical tile engine).
pub trait Driver {
    /// Initialise the back-end.
    fn init(&mut self) -> Result<(), Box<dyn std::error::Error>>;

    /// Poll for input messages, sending them through `tx`.
    /// The implementation should honour `ctx.is_done()` and return when it
    /// becomes `true`.
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
// AppConfig / App
// ---------------------------------------------------------------------------

/// Configuration for creating an [`App`].
pub struct AppConfig<M: Model, D: Driver> {
    pub model: M,
    pub driver: D,
    pub width: i32,
    pub height: i32,
    pub frame_writer: Option<Box<dyn std::io::Write>>,
}

/// The main application runner.
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

        // Start polling in a background thread.
        let poll_ctx = ctx.clone();
        let poll_tx = tx.clone();
        // We need to hand off polling to the driver, but Driver is !Send in
        // general (it lives on the main thread). Instead we do synchronous
        // polling inline (the simple / portable approach).
        //
        // For a real async driver you would spawn here; for now the driver
        // can push messages before we enter the loop, or we interleave
        // poll + draw.
        let _ = (poll_ctx, poll_tx); // suppress unused

        let mut prev_grid = Grid::new(self.width, self.height);
        let mut curr_grid = Grid::new(self.width, self.height);

        // Process the Init message first.
        self.process_pending(&rx, &ctx, &tx, &mut prev_grid, &mut curr_grid)?;

        // Main loop: poll then process.
        while !ctx.is_done() {
            // Synchronous poll: the driver pushes messages into tx and then
            // returns (non-blocking or single-event).
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

    /// Drain queued messages, update the model, draw, diff, and flush.
    fn process_pending(
        &mut self,
        rx: &Receiver<Msg>,
        ctx: &Context,
        _tx: &Sender<Msg>,
        prev_grid: &mut Grid,
        curr_grid: &mut Grid,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut needs_draw = false;

        // Drain all currently available messages.
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
            // Swap: copy current into previous.
            prev_grid.copy_from(curr_grid);
        }

        Ok(())
    }

    /// Returns `true` if the app should stop.
    fn handle_effect(&self, effect: Effect, ctx: &Context) -> bool {
        match effect {
            Effect::End => {
                ctx.cancel();
                true
            }
            Effect::Cmd(f) => {
                // Run synchronously for now.
                let _msg = f();
                // If it produced a message we could feed it back; for now
                // we drop it. A full implementation would re-enqueue.
                false
            }
            Effect::Sub(_f) => {
                // Subscriptions need a background thread; TODO.
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
