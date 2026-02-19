//! Winit graphical backend for gruid.
//!
//! Renders the grid as colored text tiles in a native window using:
//! - [`winit`] for window creation and input events
//! - [`softbuffer`] for CPU-based pixel rendering
//! - [`fontdue`] for lightweight font rasterization
//!
//! Handles high-DPI (Retina) displays automatically by scaling the font
//! size by the monitor's scale factor.

mod input;
mod renderer;

use std::num::NonZeroU32;
use std::sync::Arc;

use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoop},
    window::{Window, WindowId},
};

use gruid_core::{
    app::{AppRunner, EventLoopDriver},
    messages::Msg,
};

pub use gruid_core::TileManager;

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
    /// Font size in *logical* points. This is multiplied by the monitor's
    /// scale factor to get the actual pixel size (e.g. 18pt × 2.0 = 36px
    /// on a Retina display).
    pub font_size: f32,
    /// Number of grid columns.
    pub grid_width: i32,
    /// Number of grid rows.
    pub grid_height: i32,
    /// Optional tile manager for custom tile-based rendering.
    /// When present, cell dimensions come from [`TileManager::tile_size()`]
    /// and tiles are rendered as colorized monochrome bitmaps.
    pub tile_manager: Option<Box<dyn TileManager>>,
    /// Integer scale factor for tiles (default 0 = auto-detect from DPI).
    /// A value of 2 renders each tile pixel as a 2×2 block, etc.
    /// When 0, the scale is chosen automatically based on the monitor's
    /// DPI scale factor.
    pub tile_scale: u32,
}

impl Default for WinitConfig {
    fn default() -> Self {
        Self {
            title: "gruid".into(),
            font_data: None,
            font_size: 18.0,
            grid_width: 80,
            grid_height: 24,
            tile_manager: None,
            tile_scale: 0,
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

pub(crate) struct WinitState {
    window: Arc<Window>,
    surface: softbuffer::Surface<Arc<Window>, Arc<Window>>,
    pub(crate) renderer: GridRenderer,
    /// Current surface size in *physical* pixels.
    phys_width: u32,
    phys_height: u32,
    scale_factor: f64,
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

        // Drain messages from background effects (Cmd/Sub).
        self.runner.process_pending_msgs();

        let frame = self.runner.draw_frame();

        let state = match self.state.as_mut() {
            Some(s) => s,
            None => return,
        };

        if let Some(frame) = frame {
            state.renderer.apply_frame(&frame);
        }

        let width = state.phys_width;
        let height = state.phys_height;
        if width == 0 || height == 0 {
            return;
        }

        let mut buf = match state.surface.buffer_mut() {
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
            return;
        }

        // Probe the monitor's scale factor *before* creating the window so we
        // can size the font correctly.  If no monitor is available (e.g. CI),
        // fall back to 1.0.
        let scale_factor = event_loop
            .available_monitors()
            .next()
            .map(|m| m.scale_factor())
            .unwrap_or(1.0);

        // Scale the logical font size by the DPI factor.
        let physical_font_size = self.config.font_size * scale_factor as f32;

        // Determine tile scale: 0 means auto from DPI.
        let tile_scale = if self.config.tile_scale > 0 {
            self.config.tile_scale
        } else {
            (scale_factor.round() as u32).max(1)
        };

        let renderer = GridRenderer::new(
            self.config.font_data.as_deref(),
            physical_font_size,
            self.config.grid_width as usize,
            self.config.grid_height as usize,
            self.config.tile_manager.take(),
            tile_scale,
        );

        // The renderer now works entirely in physical pixels.
        let phys_w = renderer.pixel_width() as u32;
        let phys_h = renderer.pixel_height() as u32;

        let window_attrs = Window::default_attributes()
            .with_title(&self.config.title)
            .with_inner_size(PhysicalSize::new(phys_w, phys_h))
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
                NonZeroU32::new(phys_w).unwrap_or(NonZeroU32::new(1).unwrap()),
                NonZeroU32::new(phys_h).unwrap_or(NonZeroU32::new(1).unwrap()),
            )
            .ok();

        self.state = Some(WinitState {
            window,
            surface,
            renderer,
            phys_width: phys_w,
            phys_height: phys_h,
            scale_factor,
        });

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

            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                if let Some(state) = self.state.as_mut() {
                    state.scale_factor = scale_factor;
                    // Rebuild renderer with the new physical font size.
                    let physical_font_size = self.config.font_size * scale_factor as f32;
                    let tile_manager = state.renderer.take_tile_manager();
                    let tile_scale = if self.config.tile_scale > 0 {
                        self.config.tile_scale
                    } else {
                        (scale_factor.round() as u32).max(1)
                    };
                    state.renderer = GridRenderer::new(
                        self.config.font_data.as_deref(),
                        physical_font_size,
                        self.runner.width() as usize,
                        self.runner.height() as usize,
                        tile_manager,
                        tile_scale,
                    );
                    // Force full redraw.
                    self.runner.handle_msg(Msg::Screen {
                        width: self.runner.width(),
                        height: self.runner.height(),
                        time: std::time::Instant::now(),
                    });
                }
                self.render();
            }

            WindowEvent::Resized(PhysicalSize { width, height }) => {
                if let Some(state) = self.state.as_mut() {
                    state.phys_width = width;
                    state.phys_height = height;
                    state
                        .surface
                        .resize(
                            NonZeroU32::new(width).unwrap_or(NonZeroU32::new(1).unwrap()),
                            NonZeroU32::new(height).unwrap_or(NonZeroU32::new(1).unwrap()),
                        )
                        .ok();

                    // Recompute grid dimensions in physical pixels.
                    let (cw, ch) = state.renderer.cell_size();
                    if cw > 0 && ch > 0 {
                        let new_cols = (width as i32) / (cw as i32);
                        let new_rows = (height as i32) / (ch as i32);
                        if new_cols > 0 && new_rows > 0 {
                            state
                                .renderer
                                .resize_grid(new_cols as usize, new_rows as usize);
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

            WindowEvent::MouseInput {
                state: btn_state,
                button,
                ..
            } => {
                if let Some(msg) =
                    input::translate_mouse_button(btn_state, button, self.state.as_ref())
                {
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
