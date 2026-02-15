//! Winit graphical backend for gruid.
//!
//! Renders the grid as colored text tiles in a native window using:
//! - [`winit`] for window creation and input events
//! - [`softbuffer`] for CPU-based pixel rendering
//! - [`fontdue`] for lightweight font rasterization
//!
//! # Usage
//!
//! ```rust,no_run
//! use gruid_winit::{WinitDriver, WinitConfig};
//! use gruid_core::app::AppRunner;
//!
//! let config = WinitConfig::default();
//! let driver = WinitDriver::new(config);
//! // let runner = AppRunner::new(Box::new(my_model), 80, 24);
//! // driver.run(runner).unwrap();
//! ```

mod input;
mod renderer;

use std::num::NonZeroU32;
use std::sync::Arc;

use winit::{
    application::ApplicationHandler,
    dpi::{LogicalSize, PhysicalSize},
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoop},
    window::{Window, WindowId},
};

use gruid_core::{
    app::{AppRunner, EventLoopDriver},
    messages::Msg,
};

use renderer::GridRenderer;

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// Configuration for the winit driver.
pub struct WinitConfig {
    /// Window title.
    pub title: String,
    /// Embedded font bytes (TTF/OTF). If `None`, uses a built-in default.
    pub font_data: Option<Vec<u8>>,
    /// Font size in pixels.
    pub font_size: f32,
    /// Number of grid columns.
    pub grid_width: i32,
    /// Number of grid rows.
    pub grid_height: i32,
}

impl Default for WinitConfig {
    fn default() -> Self {
        Self {
            title: "gruid".into(),
            font_data: None,
            font_size: 16.0,
            grid_width: 80,
            grid_height: 24,
        }
    }
}

// ---------------------------------------------------------------------------
// WinitDriver
// ---------------------------------------------------------------------------

/// Winit-based graphical driver for gruid.
///
/// Implements [`EventLoopDriver`] — it owns the main-thread event loop
/// and drives an [`AppRunner`].
pub struct WinitDriver {
    config: WinitConfig,
}

impl WinitDriver {
    pub fn new(config: WinitConfig) -> Self {
        Self { config }
    }
}

impl EventLoopDriver for WinitDriver {
    fn run(self, runner: AppRunner) -> Result<(), Box<dyn std::error::Error>> {
        let event_loop = EventLoop::new()?;
        let mut app = WinitApp::new(self.config, runner);
        event_loop.run_app(&mut app)?;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// WinitApp — ApplicationHandler
// ---------------------------------------------------------------------------

struct WinitApp {
    config: WinitConfig,
    runner: AppRunner,
    state: Option<WinitState>,
}

struct WinitState {
    window: Arc<Window>,
    surface: softbuffer::Surface<Arc<Window>, Arc<Window>>,
    renderer: GridRenderer,
    pixel_width: u32,
    pixel_height: u32,
}

impl WinitApp {
    fn new(config: WinitConfig, runner: AppRunner) -> Self {
        Self {
            config,
            runner,
            state: None,
        }
    }

    fn render(&mut self) {
        if self.runner.should_quit() {
            return;
        }

        let frame = self.runner.draw_frame();

        let state = match self.state.as_mut() {
            Some(s) => s,
            None => return,
        };

        // Apply frame diff to internal pixel buffer
        if let Some(frame) = frame {
            state.renderer.apply_frame(&frame);
        }

        // Blit to softbuffer
        let width = state.pixel_width;
        let height = state.pixel_height;
        if width == 0 || height == 0 {
            return;
        }

        let mut buf = match state
            .surface
            .buffer_mut()
        {
            Ok(b) => b,
            Err(_) => return,
        };

        state
            .renderer
            .blit_to_buffer(&mut buf, width as usize, height as usize);

        buf.present().ok();
    }
}

impl ApplicationHandler for WinitApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.state.is_some() {
            return; // already initialized
        }

        // Build renderer to learn cell size
        let renderer = GridRenderer::new(
            self.config.font_data.as_deref(),
            self.config.font_size,
            self.config.grid_width as usize,
            self.config.grid_height as usize,
        );

        let pixel_w = renderer.pixel_width() as u32;
        let pixel_h = renderer.pixel_height() as u32;

        let window_attrs = Window::default_attributes()
            .with_title(&self.config.title)
            .with_inner_size(LogicalSize::new(pixel_w, pixel_h))
            .with_resizable(true);

        let window = Arc::new(
            event_loop
                .create_window(window_attrs)
                .expect("failed to create window"),
        );

        let context =
            softbuffer::Context::new(window.clone()).expect("failed to create softbuffer context");
        let mut surface = softbuffer::Surface::new(&context, window.clone())
            .expect("failed to create softbuffer surface");

        surface
            .resize(
                NonZeroU32::new(pixel_w).unwrap_or(NonZeroU32::new(1).unwrap()),
                NonZeroU32::new(pixel_h).unwrap_or(NonZeroU32::new(1).unwrap()),
            )
            .ok();

        self.state = Some(WinitState {
            window,
            surface,
            renderer,
            pixel_width: pixel_w,
            pixel_height: pixel_h,
        });

        // Send Init to the model
        self.runner.init();
        self.render();
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                self.runner.handle_msg(Msg::Quit);
                event_loop.exit();
            }

            WindowEvent::Resized(PhysicalSize { width, height }) => {
                if let Some(state) = self.state.as_mut() {
                    state.pixel_width = width;
                    state.pixel_height = height;
                    state
                        .surface
                        .resize(
                            NonZeroU32::new(width).unwrap_or(NonZeroU32::new(1).unwrap()),
                            NonZeroU32::new(height).unwrap_or(NonZeroU32::new(1).unwrap()),
                        )
                        .ok();

                    // Recompute grid dimensions based on new pixel size
                    let (cw, ch) = state.renderer.cell_size();
                    if cw > 0 && ch > 0 {
                        let new_cols = (width as i32) / (cw as i32);
                        let new_rows = (height as i32) / (ch as i32);
                        if new_cols > 0 && new_rows > 0 {
                            state.renderer.resize_grid(new_cols as usize, new_rows as usize);
                            self.runner.resize(new_cols, new_rows);
                            self.runner.handle_msg(Msg::Screen {
                                width: new_cols,
                                height: new_rows,
                                time: std::time::Instant::now(),
                            });
                        }
                    }
                }
                self.render();
            }

            WindowEvent::RedrawRequested => {
                self.render();
            }

            WindowEvent::KeyboardInput { event, .. } => {
                if let Some(msg) = input::translate_keyboard(&event) {
                    self.runner.handle_msg(msg);
                    if self.runner.should_quit() {
                        event_loop.exit();
                        return;
                    }
                    self.render();
                    if let Some(state) = self.state.as_ref() {
                        state.window.request_redraw();
                    }
                }
            }

            WindowEvent::MouseInput { state: btn_state, button, .. } => {
                if let Some(msg) = input::translate_mouse_button(btn_state, button, self.state.as_ref()) {
                    self.runner.handle_msg(msg);
                    if self.runner.should_quit() {
                        event_loop.exit();
                        return;
                    }
                    self.render();
                }
            }

            WindowEvent::CursorMoved { position, .. } => {
                if let Some(msg) = input::translate_cursor_moved(position, self.state.as_ref()) {
                    self.runner.handle_msg(msg);
                    self.render();
                }
            }

            WindowEvent::MouseWheel { delta, .. } => {
                if let Some(msg) = input::translate_mouse_wheel(delta, self.state.as_ref()) {
                    self.runner.handle_msg(msg);
                    if self.runner.should_quit() {
                        event_loop.exit();
                        return;
                    }
                    self.render();
                }
            }

            _ => {}
        }
    }
}
